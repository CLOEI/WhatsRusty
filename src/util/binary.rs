use std::{collections::HashMap, io::{Cursor, Read}};

use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;

use crate::{r#type::jid::JID, util::token::{DICTIONARY_0, DICTIONARY_3, DOUBLE_BYTE_TOKENS, SINGLE_BYTE_TOKENS}};

use super::token::{BINARY_20, BINARY_32, BINARY_8, HEX_8, JID_PAIR, LIST_16, LIST_8, LIST_EMPTY, NIBBLE_8};

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Str(String),
    Bytes(Vec<u8>),
    Jid(JID),
    List(Vec<Node>),
    Node(Box<Node>),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub description: String,
    pub attributes: HashMap<String, Value>,
    pub content: Option<Value>,
}

pub struct BinaryDecoder {
    reader: Cursor<Vec<u8>>,
    closed: bool,
}

impl Node {
    pub fn new(description: String, attributes: HashMap<String, Value>, content: Option<Value>) -> Self {
        Node { description, attributes, content }
    }
}

impl BinaryDecoder {
    pub fn new(buffer: Vec<u8>) -> Self {
        let token = buffer[0] & 2;
        let data = if token == 0 {
            buffer[1..].to_vec()
        } else {
            unpack(&buffer[1..])
        };
        Self {
            reader: Cursor::new(data),
            closed: false,
        }
    }

    pub fn decode(&mut self) -> Node {
        if self.closed {
            panic!("Decoder closed")
        }
        let token = self.reader.read_u8().unwrap();
        let size = self.read_size(token);
        if size == 0 {
            panic!("Cannot decode with empty body")
        }

        let description = self.read_string();
        let attrs = self.read_attributes(size);

        if size % 2 != 0 {
            Node::new(description, attrs, None)
        } else {
            let content = self.read(false);
            Node::new(description, attrs, Some(content))
        }
    }

    fn read_size(&mut self, token: u8) -> usize {
        println!("Token: {}", token);
        match token {
            LIST_8 => self.reader.read_u8().unwrap() as usize,
            LIST_16 => self.reader.read_u16::<BigEndian>().unwrap() as usize,
            _ => panic!("Invalid list token"),
        }
    }

    fn read_attributes(&mut self, size: usize) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        for _ in 0..(size - 1) / 2 {
            let key = self.read_string();
            let value = self.read(true);
            map.insert(key.clone(), get_value_with_context(&key, value));
        }
        map
    }

    fn read_string(&mut self) -> String {
        match self.read(true) {
            Value::Str(s) => s,
            _ => panic!("Expected string"),
        }
    }

    fn read(&mut self, parse_bytes: bool) -> Value {
        let tag = self.reader.read_u8().unwrap();
        println!("Tag: {}", tag);
        match tag {
            LIST_EMPTY => Value::Null,
            JID_PAIR => self.read_jid_pair(),
            LIST_8 => {
                let size = self.reader.read_u8().unwrap() as usize;
                self.read_list(size)
            }
            LIST_16 => {
                let size = self.reader.read_u16::<BigEndian>().unwrap() as usize;
                self.read_list(size)
            }
            HEX_8 => self.read_hex_string(),
            BINARY_8 => {
                let size = self.reader.read_u8().unwrap() as usize;
                self.read_binary(size, parse_bytes)
            }
            BINARY_20 => {
                let size = self.read_string20_length();
                self.read_binary(size, parse_bytes)
            }
            BINARY_32 => {
                let size = self.reader.read_u16::<BigEndian>().unwrap() as usize;
                self.read_binary(size, parse_bytes)
            }
            NIBBLE_8 => self.read_nibble(),
            token => self.read_string_from_token(token),
        }
    }

    fn read_binary(&mut self, size: usize, parse_bytes: bool) -> Value {
        let mut data = vec![0u8; size];
        self.reader.read_exact(&mut data).unwrap();
        if parse_bytes {
            Value::Str(String::from_utf8_lossy(&data).into_owned())
        } else {
            Value::Bytes(data)
        }
    }

    fn read_string20_length(&mut self) -> usize {
        let b1 = self.reader.read_u8().unwrap() as usize;
        let b2 = self.reader.read_u8().unwrap() as usize;
        let b3 = self.reader.read_u8().unwrap() as usize;
        ((b1 & 0x0F) << 16) | (b2 << 8) | b3
    }

    fn read_list(&mut self, size: usize) -> Value {
        let mut list = Vec::with_capacity(size);
        for _ in 0..size {
            list.push(self.decode());
        }
        Value::List(list)
    }

    fn read_jid_pair(&mut self) -> Value {
        let user = self.read_string();
        let server = self.read_string();
        Value::Jid(JID {
            user: Some(user),
            server: Some(server),
            raw_agent: None,
            device: None,
            integrator: None,
        })
    }

    fn read_nibble(&mut self) -> Value {
        let size = self.reader.read_u8().unwrap() as usize;
        let mut bytes = vec![0u8; size];
        self.reader.read_exact(&mut bytes).unwrap();
        Value::Bytes(bytes)
    }

    fn read_hex_string(&mut self) -> Value {
        let size = self.reader.read_u8().unwrap() as usize;
        let mut hex = vec![0u8; size];
        self.reader.read_exact(&mut hex).unwrap();
        Value::Str(hex.iter().map(|b| format!("{:02x}", b)).collect())
    }

    fn read_string_from_token(&mut self, token: u8) -> Value {
        if token < DICTIONARY_0 || token > DICTIONARY_3 {
            if let Some(s) = SINGLE_BYTE_TOKENS.get((token as usize).wrapping_sub(1)) {
                println!("Single byte token: {}", s);
                return Value::Str(s.to_string());
            }
            panic!("Unknown single byte token: {}", token);
        }

        let delta = (DOUBLE_BYTE_TOKENS.len() / 4) * (token as usize - DICTIONARY_0 as usize);
        let index = self.reader.read_u8().unwrap() as usize + delta;
        
        if let Some(row) = DOUBLE_BYTE_TOKENS.get(delta / (DOUBLE_BYTE_TOKENS.len() / 4)) {
            if let Some(s) = row.get(index % row.len()) {
                return Value::Str(s.to_string());
            }
        }
        panic!("Unknown double byte token: {} at delta {}", token, delta);
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

}


pub fn unpack(data: &[u8]) -> Vec<u8> {
    let mut e = ZlibDecoder::new(data);
    let mut s = String::new();
    e.read_to_string(&mut s).unwrap();
    s.into_bytes()
}

fn get_value_with_context(key: &str, value: Value) -> Value {
    if key == "lid" {
        if let Value::Jid(mut jid) = value {
            jid.server = Some("lid".to_string());
            return Value::Jid(jid);
        }
    }
    value
}