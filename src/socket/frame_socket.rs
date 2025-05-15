use async_trait::async_trait;
use prost::Message;

use crate::{constant, proto::whatsapp::{handshake_message::ClientHello, HandshakeMessage}, util::key::Key};

#[derive(Default)]
pub enum FrameSocketState {
    #[default]
    HANDSHAKE,
    CONNECTED
}

#[derive(Default)]
pub struct FrameSocket {
    pub handle: Option<ezsockets::Client<Self>>,
    pub state: FrameSocketState
}

impl FrameSocket {
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

        self.handle.as_ref().unwrap().binary(frame).expect("Fail sending frame");
    }

    pub fn process_data(&self, data: Vec<u8>) -> Vec<u8> {
        // let data_length = ((data[0] as usize) << 16) + ((data[1] as usize) << 8) + (data[2] as usize);
        data[constant::FRAME_LENGTH_SIZE..].to_vec()
    }

    pub fn send_hello(&self) {
        let key = Key::new();
        let client_hello = HandshakeMessage {
            client_hello: Some(ClientHello {
                ephemeral: Some(key.public.to_vec()),
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
                let server_hello = HandshakeMessage::decode(&data[..]);
                println!("{server_hello:?}");
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