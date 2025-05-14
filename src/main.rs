use async_trait::async_trait;
use ezsockets::{Bytes, ClientConfig};
use prost::Message;
use proto::whatsapp::{handshake_message::{ClientHello}, HandshakeMessage};
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, MontgomeryPoint, scalar::Scalar};
use rand_core::OsRng;

mod proto;

#[derive(Debug, PartialEq)]
enum SocketState {
    WAITING,
    CONNECTED,
}

struct Client {
    handle: Option<ezsockets::Client<Self>>,
    key: Key,
    state: SocketState,
    noise: Option<snow::HandshakeState>,
    incoming: Option<Vec<u8>>,
    incoming_length: usize,
    received_length: usize,
    partial_header: Option<Vec<u8>>,
}

impl Client {
    fn set_handle(&mut self, handle: ezsockets::Client<Self>) {
        self.handle = Some(handle);
    }

    fn send_frame(&self, data: &[u8]) -> Result<(), ezsockets::Error> {
        let handle = self.handle.as_ref().unwrap();
        let data_length = data.len();
        if data_length >= (2 << 23) {
            println!("Frame too large (got {} bytes, max {} bytes)", data_length, (2 << 23));
            return Ok(());
        }

        let header = vec![b'W', b'A', 6, 3];
        let header_length = header.len();
        let mut whole_frame = Vec::with_capacity(header_length + 3 + data_length);

        whole_frame.extend_from_slice(&header);

        whole_frame.push((data_length >> 16) as u8);
        whole_frame.push((data_length >> 8) as u8);
        whole_frame.push(data_length as u8);

        whole_frame.extend_from_slice(data);

        // Send the frame
        println!("sending frame: {:?}", whole_frame);
        handle.binary(Bytes::from(whole_frame))?;

        Ok(())
    }

    fn process_data(&mut self, msg: &[u8]) {
        let mut msg = msg.to_vec();
        while !msg.is_empty() {
            // Handle partial header
            if let Some(partial) = self.partial_header.take() {
                msg = [&partial[..], &msg[..]].concat();
            }

            if self.incoming.is_none() {
                if msg.len() >= 3 {
                    // Parse length (3 bytes, big-endian)
                    let length = ((msg[0] as usize) << 16) + 
                               ((msg[1] as usize) << 8) + 
                               (msg[2] as usize);
                    self.incoming_length = length;
                    self.received_length = msg.len() - 3;
                    msg = msg[3..].to_vec();

                    if msg.len() >= length {
                        self.incoming = Some(msg[..length].to_vec());
                        msg = msg[length..].to_vec();
                        self.frame_complete();
                    } else {
                        self.incoming = Some(vec![0; length]);
                        self.incoming.as_mut().unwrap()[..msg.len()].copy_from_slice(&msg);
                        msg = vec![];
                    }
                } else {
                    println!("Received partial header");
                    self.partial_header = Some(msg);
                    msg = vec![];
                }
            } else {
                let incoming = self.incoming.as_mut().unwrap();
                if self.received_length + msg.len() >= self.incoming_length {
                    let remaining = self.incoming_length - self.received_length;
                    incoming[self.received_length..].copy_from_slice(&msg[..remaining]);
                    msg = msg[remaining..].to_vec();
                    self.frame_complete();
                } else {
                    incoming[self.received_length..].copy_from_slice(&msg);
                    self.received_length += msg.len();
                    msg = vec![];
                }
            }
        }
    }

    fn frame_complete(&mut self) {
        if let Some(data) = self.incoming.take() {
            // Process the complete frame
            println!("Received complete frame: {:02x?}", data);
            if let Ok(msg) = HandshakeMessage::decode(&data[..]) {
                println!("Decoded handshake message: {:?}", msg);
            }
        }
        self.partial_header = None;
        self.incoming_length = 0;
        self.received_length = 0;
    }
}

struct Key {
    public: MontgomeryPoint,
    private: Scalar,
}

impl Key {
    fn new() -> Self {
        let private = Scalar::random(&mut OsRng);
        let public = (ED25519_BASEPOINT_POINT * private).to_montgomery();
        Self { public, private }
    }
}

#[async_trait]
impl ezsockets::ClientExt for Client {
    type Call = ();

    async fn on_text(&mut self, text: ezsockets::Utf8Bytes) -> Result<(), ezsockets::Error> {
        println!("received message: {text}");
        Ok(())
    }

    async fn on_binary(&mut self, bytes: ezsockets::Bytes) -> Result<(), ezsockets::Error> {
        self.process_data(bytes.as_ref());
        Ok(())
    }

    async fn on_call(&mut self, call: Self::Call) -> Result<(), ezsockets::Error> {
        let () = call;
        Ok(())
    }

    async fn on_connect(&mut self) -> Result<(), ezsockets::Error> {
        if self.state == SocketState::CONNECTED {
            return Ok(());
        }
        println!("on_connect");
        let mut prologue = vec![b'W', b'A', 6, 3];
        prologue.extend_from_slice(&self.key.public.to_bytes().to_vec());
        self.noise = Some(snow::Builder::new("Noise_XX_25519_AESGCM_SHA256".parse().unwrap())
            .prologue(&prologue)
            .build_initiator().unwrap());

        let hand_shake_message = HandshakeMessage {
            client_hello: Some(ClientHello {
                ephemeral: Some(self.key.public.to_bytes().to_vec()),
                r#static: None,
                payload: None
            }),
            ..Default::default()
        };
        let hand_shake_message = hand_shake_message.encode_to_vec();
        self.send_frame(&hand_shake_message).unwrap();
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let uri = "wss://web.whatsapp.com/ws/chat";
    let config = ClientConfig::new(uri).header("Origin", "https://web.whatsapp.com");

    let mut client = Client { 
        handle: None, 
        key: Key::new(), 
        state: SocketState::WAITING, 
        noise: None,
        incoming: None,
        incoming_length: 0,
        received_length: 0,
        partial_header: None,
    };
    let (handle, future) = ezsockets::connect(|handle| {
        client.set_handle(handle);
        client
    }, config).await;
    tokio::spawn(async move {
        future.await.unwrap();
    });
    loop {}
}