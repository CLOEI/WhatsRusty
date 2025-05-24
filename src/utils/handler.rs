use std::collections::HashMap;

use base64::{engine::general_purpose, Engine};

use crate::client::Client;

use super::decoder::{Node, Value};

impl Client {
    pub async fn handle_qr(&mut self, node: &Node) {
        let child = match node.content.as_ref() {
            None => return,
            Some(Value::List(content)) => content,
            Some(_) => return
        };
        match child[0].tag.as_str() {
            "pair-device" => {
                let Value::List(pair_device) = child[0].content.as_ref().unwrap() else { return };
                let mut pair_attr = HashMap::new();

                pair_attr.insert("to".to_string(), node.attributes.get("from").unwrap().clone());
                pair_attr.insert("id".to_string(), node.attributes.get("id").unwrap().clone());
                pair_attr.insert("type".to_string(), Value::Str("result".to_string()));

                let pair_node = Node::new("iq".to_string(), pair_attr, None);
                self.send_node_and_get_data(pair_node).await;

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
                panic!("Unknown node: {}", child[0].tag);
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