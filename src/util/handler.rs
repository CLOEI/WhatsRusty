use crate::socket::frame_socket::FrameSocket;

use super::binary::{Node, Value};

impl FrameSocket {
    pub fn handle_qr(&self, node: &Node) {
        let child = node.content.as_ref().unwrap();
        let Value::List(content) = child else { return };
        match content[0].description.as_str() {
            "pair-device" => {
                let Value::List(pair_device) = content[0].content.as_ref().unwrap() else { return };
                println!("Pair device: {:?}", pair_device);
            }
            "pair-success" => {

            }
            _ => {
                panic!("Unknown node: {}", content[0].description);
            }
        }
    }
}