use crate::client::Client;

use super::binary::{Node, Value};

impl Client {
    pub fn handle_qr(&self, node: &Node) {
        let child = node.content.as_ref().unwrap();
        let Value::List(content) = child else { return };
        match content[0].tag.as_str() {
            "pair-device" => {
                let Value::List(pair_device) = content[0].content.as_ref().unwrap() else { return };
                for node in pair_device {
                    if node.tag != "ref" {
                        continue;
                    }
                    let Value::Bytes(bytes) = node.content.as_ref().unwrap() else { return };
                    println!("Bytes: {:?}", bytes);
                }
                if let Some(handle) = &self.handle {
                    handle.on_qr("Hello world");
                }
            }
            "pair-success" => {

            }
            _ => {
                panic!("Unknown node: {}", content[0].tag);
            }
        }
    }
}