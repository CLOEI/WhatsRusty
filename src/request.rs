use std::collections::HashMap;

use futures_util::SinkExt;
use paris::info;

use crate::{client::Client, types::jid::JID, utils::{decoder::{Node, Value}, encoder::BinaryEncoder}};

#[derive(Default)]
pub struct InfoQuery {
    pub namespace: Option<String>,
    pub r#type: Option<String>,
    pub to: Option<JID>,
    pub target: Option<JID>,
    pub id: Option<String>,
    pub content: Option<Value>
}

impl Client {
    pub async fn send_iq(&mut self, query: InfoQuery) {
        let mut attr = HashMap::new();
        attr.insert("id".to_string(), Value::Str(self.generate_request_id()));
        attr.insert("xmlns".to_string(), Value::Str(query.namespace.unwrap()));
        attr.insert("type".to_string(), Value::Str(query.r#type.unwrap()));

        if let Some(to) = query.to {
            attr.insert("to".to_string(), Value::Jid(to));
        }

        if let Some(target) = query.target {
            attr.insert("target".to_string(), Value::Jid(target));
        }

        self.send_node_and_get_data(Node::new("iq".to_string(), attr, query.content)).await;
    }

    pub async fn send_node_and_get_data(&mut self, node: Node) {
        let data = BinaryEncoder::new().write_node(&node);
        info!("Sending node: {}", node.to_xml());
        let frame = self.ns.as_mut().expect("Noise socket not initialized").make_frame(data.clone());
        self.write.send(self.fs.make_frame(frame).into()).await.unwrap();
    }

    pub fn generate_request_id(&mut self) -> String {
        let id = format!("{}{}", self.unique_id, self.id_counter);
        self.id_counter += 1;
        id
    }
}