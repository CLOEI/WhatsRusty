use crate::constant;
use crate::utils::key::Key;

#[derive(PartialEq)]
pub enum FrameSocketState {
    Handshake,
    Authenticated,
}

pub struct FrameSocket {
    pub key: Key,
    pub state: FrameSocketState,
}

impl FrameSocket {
    pub fn new() -> Self {
        let key = Key::new();
        Self {
            key,
            state: FrameSocketState::Handshake,
        }
    }

    pub fn process_data(&self, data: Vec<u8>) -> Vec<u8> {
        data[constant::FRAME_LENGTH_SIZE..].to_vec()
    }

    pub fn make_frame(&self, data: Vec<u8>) -> Vec<u8> {
        let data_length = data.len();
        let header_length = 4;

        if data_length >= constant::FRAME_MAX_SIZE {
            panic!("Frame too large got {}, max {}", data_length, constant::FRAME_MAX_SIZE);
        }

        let mut frame = Vec::with_capacity(header_length + constant::FRAME_LENGTH_SIZE + data_length);

        if self.state == FrameSocketState::Handshake {
            frame.extend_from_slice( &[b'W', b'A', 6, 3]);
        }
        frame.push((data_length >> 16) as u8);
        frame.push((data_length >> 8) as u8);
        frame.push(data_length as u8);
        frame.extend_from_slice(&data);

        frame
    }
}