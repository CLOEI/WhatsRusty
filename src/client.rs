use std::sync::{Arc, Mutex};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use paris::{error, info};
use prost::Message;
use rand_core::RngCore;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{ClientRequestBuilder};
use crate::constant;
use crate::device::Device;
use crate::proto::whatsapp::handshake_message::{ClientFinish, ClientHello};
use crate::proto::whatsapp::HandshakeMessage;
use crate::socket::frame_socket::{FrameSocket, FrameSocketState};
use crate::socket::noise_socket::NoiseSocket;
use crate::utils::decoder::{BinaryDecoder, Node};
use crate::utils::gcm;
use crate::utils::noise_handshake::NoiseHandShake;

pub struct Client {
    write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tokio_tungstenite::tungstenite::Message>,
    fs: FrameSocket,
    ns: Option<NoiseSocket>,
    pub unique_id: String,
    pub device: Device,
    pub handle: Option<Box<dyn Events>>
}

impl Client {
    pub fn process(&self, node: &Node) {
        match node.tag.as_str() {
            "iq" => self.handle_qr(node),
            _ => error!("Node not handled: {}", node.tag)
        }
    }

    pub fn keep_alive(&self) {

    }

    async fn do_handshake(&mut self, read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>) {
        let client_hello = HandshakeMessage {
            client_hello: Some(ClientHello {
                ephemeral: Some(self.fs.key.public.to_bytes().to_vec()),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.write.send(self.fs.make_frame(client_hello.encode_to_vec()).into()).await.unwrap();

        let message = if let Some(message) = read.next().await {
            self.fs.process_data(Vec::from(message.expect("Failed to read message").into_data()))
        } else {
            panic!("No message received");
        };

        let mut nhs = NoiseHandShake::default();
        nhs.start(constant::CONN_HEADER.to_vec());
        nhs.authenticate(&self.fs.key.public.to_bytes().to_vec());

        let message = HandshakeMessage::decode(&message[..]).unwrap();
        let (server_ephemeral, server_static_cipher_text, certificate_cipher_text) = {
            let server_hello = message.server_hello.unwrap();
            (server_hello.ephemeral.unwrap(), server_hello.r#static.unwrap(), server_hello.payload.unwrap())
        };

        nhs.authenticate(&server_ephemeral);
        nhs.mix_shared_secret(self.fs.key.private.to_bytes(), server_ephemeral.clone().try_into().unwrap());

        let static_decrypted = nhs.decrypt(&server_static_cipher_text);
        nhs.mix_shared_secret(self.fs.key.private.to_bytes(), static_decrypted.clone().try_into().unwrap());

        nhs.decrypt(&certificate_cipher_text);

        let encrypted_pubkey = nhs.encrypt(&self.device.noise_key.public.as_bytes().to_vec());
        nhs.mix_shared_secret(self.device.noise_key.private.to_bytes(), server_ephemeral.try_into().unwrap());

        let encrypted_client_payload = nhs.encrypt(&self.device.create_register_payload().encode_to_vec());
        let client_finish = HandshakeMessage {
            client_finish: Some(ClientFinish {
                payload: Some(encrypted_client_payload),
                r#static: Some(encrypted_pubkey),
            }),
            ..Default::default()
        };

        self.fs.state = FrameSocketState::Authenticated;
        self.write.send(self.fs.make_frame(client_finish.encode_to_vec()).into()).await.expect("Failed to send handshake message");

        let (write_key, read_key) = nhs.extract_and_expand(None);
        let write_key = gcm::prepare(write_key);
        let read_key = gcm::prepare(read_key);
        self.ns = Some(NoiseSocket::new(write_key, read_key));
    }
}

pub async fn connect<E: Events + 'static>(handle: E) -> Arc<Mutex<Client>> {
    info!("Dialing {}", constant::WS_URL);
    let request = ClientRequestBuilder::new(constant::WS_URL.parse().unwrap())
        .with_header("Origin", constant::ORIGIN)
        .into_client_request().unwrap();

    let (ws_stream, _) = connect_async(request).await.expect("Can't connect to whatsapp");

    let (write, mut read) = ws_stream.split();

    let mut unique_ids = [0u8; 2];
    rand_core::OsRng.fill_bytes(&mut unique_ids);

    let client = Arc::new(Mutex::new(Client {
        write,
        fs: FrameSocket::new(),
        ns: None,
        unique_id: format!("{}.{}-", unique_ids[0], unique_ids[1]),
        device: Device::new(),
        handle: Some(Box::new(handle))
    }));

    {
        let mut client = client.lock().unwrap();
        client.do_handshake(&mut read).await;
    }

    let client_clone = Arc::clone(&client);

    info!("Message processor started");
    tokio::spawn(async move {
        let mut read = read;
        let client = client_clone;
        while let Some(message) = read.next().await {
            match message {
                Ok(msg) => {
                    let data = {
                        let mut client = client.lock().unwrap();
                        let data = client.fs.process_data(Vec::from(msg.into_data()));
                        client.ns.as_mut().unwrap().receive_encrypted_frame(&data)
                    };
                    let node = BinaryDecoder::new(data).decode();
                    info!("Received node: {}", node.to_xml());
                    {
                        client.lock().unwrap().process(&node);
                    }
                }
                Err(e) => {
                    error!("Error: {}", e);
                    break;
                }
            }
        }
    });

    client
}

pub trait Events: Send {
    fn on_qr(&self, qr: &str);
}