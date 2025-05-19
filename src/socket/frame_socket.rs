use std::sync::Arc;

use async_trait::async_trait;
use prost::Message;

use crate::{client::Client, constant, proto::whatsapp::{client_payload::{self, user_agent::{self, AppVersion}, web_info, DevicePairingRegistrationData, UserAgent, WebInfo}, device_props, handshake_message::{ClientFinish, ClientHello}, ClientPayload, DeviceProps, HandshakeMessage}, socket::noise_socket::NoiseSocket, util::{binary::{self, BinaryDecoder}, gcm, key::Key, noise_hand_shake::NoiseHandShake}};

#[derive(Default, PartialEq)]
pub enum FrameSocketState {
    #[default]
    HANDSHAKE,
    CONNECTED
}

pub struct FrameSocket {
    pub handle: ezsockets::Client<Self>,
    pub state: FrameSocketState,
    pub key: Key,
    pub client: Arc<Client>,
    pub ns: Option<NoiseSocket>
}

impl FrameSocket {
    pub fn new(handle: ezsockets::Client<Self>, client: Arc<Client>) -> Self {
        Self {
            handle,
            state: FrameSocketState::HANDSHAKE,
            key: Key::new(),
            client,
            ns: None
        }
    }

    pub fn send_frame(&self, data: Vec<u8>) {
        let data_length = data.len();
        let header_length = constant::CONN_HEADER.len();
        
        if data_length >= constant::FRAME_MAX_SIZE {
            println!("Frame too large got {}, max {}", data_length, constant::FRAME_MAX_SIZE);
        }

        let mut frame = Vec::with_capacity(header_length + constant::FRAME_LENGTH_SIZE + data_length);
        if self.state == FrameSocketState::HANDSHAKE {
            frame.extend_from_slice(&constant::CONN_HEADER);
        }
        frame.push((data_length >> 16) as u8);
        frame.push((data_length >> 8) as u8);
        frame.push(data_length as u8);
        frame.extend_from_slice(&data);

        println!("Frame length: {}", frame.len());
        println!("Frame: {:?}", frame);
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
                nhs.mix_shared_secret(self.key.private, static_decrypted.clone().try_into().unwrap());

                nhs.decrypt(&certificate_cipher_text);

                let encrypted_pubkey = nhs.encrypt(&self.client.device.noise_key.public.to_vec());
                nhs.mix_shared_secret(self.client.device.noise_key.private, server_ephemeral.try_into().unwrap());

                let reg_id: [u8; 4] = self.client.device.registration_id.to_be_bytes();
                let pre_key_id: [u8; 4] = self.client.device.signed_pre_key.id.to_be_bytes();

                let client_payload = ClientPayload {
                    user_agent: Some(UserAgent {
                        platform: Some(user_agent::Platform::Web.into()),
                        release_channel: Some(user_agent::ReleaseChannel::Release.into()),
                        app_version: Some(AppVersion {
                            primary: Some(2),
                            secondary: Some(3000),
                            tertiary: Some(1022419966),
                            ..Default::default()
                        }),
                        mcc: Some("000".to_string()),
                        mnc: Some("000".to_string()),
                        os_version: Some("0.1.0".to_string()),
                        manufacturer: Some("".to_string()),
                        device: Some("Desktop".to_string()),
                        os_build_number: Some("0.1.0".to_string()),
                        locale_language_iso6391: Some("en".to_string()),
                        locale_country_iso31661_alpha2: Some("en".to_string()),
                        ..Default::default()
                    }),
                    web_info: Some(WebInfo {
                        web_sub_platform: Some(web_info::WebSubPlatform::WebBrowser.into()),
                        ..Default::default()
                    }),
                    connect_type: Some(client_payload::ConnectType::WifiUnknown.into()),
                    connect_reason: Some(client_payload::ConnectReason::UserActivated.into()),
                    device_pairing_data: Some(DevicePairingRegistrationData {
                        e_regid: Some(reg_id.to_vec()),
                        e_keytype: Some(vec![0x05]),
                        e_ident: Some(self.client.device.identity_key.public.to_vec()),
                        e_skey_id: Some(pre_key_id[1..].to_vec()),
                        e_skey_val: Some(self.client.device.signed_pre_key.key.public.to_vec()),
                        e_skey_sig: Some(self.client.device.signed_pre_key.signature.to_vec()),
                        build_hash: Some(calculate_wa_version_hash().to_vec()),
                        device_props: Some(DeviceProps {
                            os: Some("WhatsRusty".to_string()),
                            version: Some(device_props::AppVersion {
                                primary: Some(0),
                                secondary: Some(1),
                                tertiary: Some(0),
                                ..Default::default()
                            }),
                            platform_type: Some(0),
                            require_full_sync: Some(false),
                            ..Default::default()
                        }.encode_to_vec()),
                        ..Default::default()
                    }),
                    passive: Some(false),
                    pull: Some(false),
                    ..Default::default()
                };

                let encrypted_client_payload = nhs.encrypt(&client_payload.encode_to_vec());

                let client_finish = HandshakeMessage {
                    client_finish: Some(ClientFinish {
                        payload: Some(encrypted_client_payload),
                        r#static: Some(encrypted_pubkey),
                    }),
                    ..Default::default()
                };
                
                self.state = FrameSocketState::CONNECTED;
                self.send_frame(client_finish.encode_to_vec());
                let (write_key, read_key) = nhs.extract_and_expand(None);
                let write_key = gcm::prepare(write_key);
                let read_key = gcm::prepare(read_key);
                self.ns = Some(NoiseSocket::new(write_key, read_key));
            },
            FrameSocketState::CONNECTED => {
                let decrypted = self.ns.as_mut().unwrap().receive_encrypted_frame(&data);
                let node = BinaryDecoder::new(decrypted).decode();
                println!("Node: {}", node.to_xml());
            }
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

fn calculate_wa_version_hash() -> [u8; 16] {
    let version = "2.3000.1022419966";
    let digest = md5::compute(version.as_bytes());
    digest.into()
}
