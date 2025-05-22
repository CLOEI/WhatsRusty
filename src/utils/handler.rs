use base64::{engine::general_purpose, Engine};

use crate::client::Client;

use super::decoder::{Node, Value};

impl Client {
    pub fn handle_qr(&self, node: &Node) {
        let child = node.content.as_ref().unwrap();
        let Value::List(content) = child else { return };
        match content[0].tag.as_str() {
            "pair-device" => {
                let Value::List(pair_device) = content[0].content.as_ref().unwrap() else { return };
                let mut codes = Vec::new();
                for node in pair_device {
                    if node.tag != "ref" {
                        continue;
                    }
                    let Value::Bytes(bytes) = node.content.as_ref().unwrap() else { return };
                    let data = self.make_qr_data(String::from_utf8(bytes.to_vec()).unwrap());
                    codes.push(data);
                }
                if let Some(handle) = &self.handle {
                    handle.on_qr(&codes[0]);
                }
            }
            "pair-success" => {

            }
            _ => {
                panic!("Unknown node: {}", content[0].tag);
            }
        }
    }

    fn make_qr_data(&self, data: String) -> String {
        let noise = general_purpose::STANDARD.encode(self.device.noise_key.public.as_bytes());
        let identity = general_purpose::STANDARD.encode(self.device.identity_key.public.as_bytes());
        let adv = general_purpose::STANDARD.encode(self.device.adv_secret_key);
        format!("{},{},{},{}", data, noise, identity, adv)
    }
}