use std::sync::Arc;

use async_trait::async_trait;
use prost::Message;

use crate::{client::Client, constant, proto::whatsapp::{client_payload::{user_agent::AppVersion, DevicePairingRegistrationData, UserAgent, WebInfo}, handshake_message::{ClientFinish, ClientHello}, ClientPayload, HandshakeMessage}, util::{key::Key, noise_hand_shake::NoiseHandShake}};

#[derive(Default)]
pub enum FrameSocketState {
    #[default]
    HANDSHAKE,
    CONNECTED
}

pub struct FrameSocket {
    pub handle: ezsockets::Client<Self>,
    pub state: FrameSocketState,
    pub key: Key,
    pub client: Arc<Client>
}

impl FrameSocket {
    pub fn new(handle: ezsockets::Client<Self>, client: Arc<Client>) -> Self {
        Self {
            handle,
            state: FrameSocketState::HANDSHAKE,
            key: Key::new(),
            client
        }
    }

    pub fn send_frame(&self, data: Vec<u8>) {
        let data_length = data.len();
        let header_length = constant::CONN_HEADER.len();
        
        if data_length >= constant::FRAME_MAX_SIZE {
            println!("Frame too large got {}, max {}", data_length, constant::FRAME_MAX_SIZE);
        }

        let mut frame = Vec::with_capacity(header_length + constant::FRAME_LENGTH_SIZE + data_length);
        frame.extend_from_slice(&constant::CONN_HEADER);
        frame.push((data_length >> 16) as u8);
        frame.push((data_length >> 8) as u8);
        frame.push(data_length as u8);
        frame.extend_from_slice(&data);

        self.handle.binary(frame).expect("Fail sending frame");
    }

    pub fn process_data(&self, data: Vec<u8>) -> Vec<u8> {
        // let data_length = ((data[0] as usize) << 16) + ((data[1] as usize) << 8) + (data[2] as usize);
        data[constant::FRAME_LENGTH_SIZE..].to_vec()
    }

    pub fn send_hello(&self) {
        let client_hello = HandshakeMessage {
            client_hello: Some(ClientHello {
                ephemeral: Some(self.key.public.to_vec()),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.send_frame(client_hello.encode_to_vec());
    }
}

#[async_trait]
impl ezsockets::ClientExt for FrameSocket {
    type Call = ();

    async fn on_text(&mut self, text: ezsockets::Utf8Bytes) -> Result<(), ezsockets::Error> {
        Ok(())
    }

    async fn on_binary(&mut self, bytes: ezsockets::Bytes) -> Result<(), ezsockets::Error> {
        // println!("Received bytes {:?}", bytes);
        let data = self.process_data(bytes.to_vec());
        match self.state {
            FrameSocketState::HANDSHAKE => {
                let mut nhs = NoiseHandShake::default();
                nhs.start(constant::CONN_HEADER.to_vec());
                nhs.authenticate(self.key.public.to_vec());
                let server_hello = HandshakeMessage::decode(&data[..]).unwrap();

                let (server_ephemeral, server_static_cipher_text, certificate_cipher_text) = {
                    let server_hello = server_hello.server_hello.unwrap();
                    (server_hello.ephemeral.unwrap(), server_hello.r#static.unwrap(), server_hello.payload.unwrap())
                };
                
                nhs.authenticate(server_ephemeral.clone());
                nhs.mix_shared_secret(self.key.private, server_ephemeral.clone().try_into().unwrap());
                let static_decrypted = nhs.decrypt(&server_static_cipher_text);
                nhs.mix_shared_secret(self.key.private, static_decrypted.try_into().unwrap());
                let cert_decrypted = nhs.decrypt(&certificate_cipher_text);
                let encrypted_pubkey = nhs.encrypt(&cert_decrypted);
                nhs.mix_shared_secret(self.client.device.noise_key.private, server_ephemeral.try_into().unwrap());

                let reg_id: [u8; 4] = self.client.device.registration_id.to_be_bytes();
                let pre_key_id: [u8; 4] = self.client.device.signed_pre_key.key_id.to_be_bytes();

                let client_payload = ClientPayload {
                    user_agent: Some(UserAgent {
                        platform: Some(14),
                        app_version: Some(AppVersion {
                            primary: Some(2),
                            secondary: Some(2413),
                            tertiary: Some(51),
                            ..Default::default()
                        }),
                        mcc: Some("000".to_string()),
                        mnc: Some("000".to_string()),
                        os_version: Some("0.1.0".to_string()),
                        manufacturer: Some("".to_string()),
                        device: Some("Desktop".to_string()),
                        os_build_number: Some("0.1.0".to_string()),
                        locale_language_iso6391: Some("en".to_string()),
                        locale_country_iso31661_alpha2: Some("en".to_string())
                    }),
                    web_info: Some(WebInfo {
                        web_sub_platform: Some(0),
                        ..Default::default()
                    }),
                    connect_type: Some(1),
                    connect_reason: Some(1),
                    device_pairing_data: Some(DevicePairingRegistrationData {

                    })
                };

                let client_finish = HandshakeMessage {
                    client_finish: Some(ClientFinish {
                        payload: None,
                        r#static: Some(encrypted_pubkey),
                    }),
                    ..Default::default()
                };

                self.state = FrameSocketState::CONNECTED;
            },
            FrameSocketState::CONNECTED => {}
        }
        Ok(())
    }

    async fn on_call(&mut self, call: Self::Call) -> Result<(), ezsockets::Error> {
        let () = call;
        Ok(())
    }

    async fn on_connect(&mut self) -> Result<(), ezsockets::Error> {
        match self.state {
            FrameSocketState::HANDSHAKE => {
                self.send_hello();
            },
            FrameSocketState::CONNECTED => {}
        }
        Ok(())
    }
}