use std::{collections::HashMap, io::{Cursor, Read}};

use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;

use crate::{types::jid::JID, utils::token::{DICTIONARY_0, DICTIONARY_3, DOUBLE_BYTE_TOKENS, SINGLE_BYTE_TOKENS}};

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
    pub tag: String,
    pub attributes: HashMap<String, Value>,
    pub content: Option<Value>,
}

pub struct BinaryDecoder {
    reader: Cursor<Vec<u8>>,
}

impl Node {
    pub fn new(tag: String, attributes: HashMap<String, Value>, content: Option<Value>) -> Self {
        Node { tag, attributes, content }
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
        }
    }

    pub fn decode(&mut self) -> Node {
        let token = self.reader.read_u8().unwrap();
        let size = self.read_size(token);
        if size == 0 {
            panic!("Cannot decode with empty body")
        }

        let description = self.read_string();
        let attrs = self.read_attributes(size);

        let mut node = Node::new(description, attrs, None);
        if size % 2 == 0 {
            node.content = Some(self.read(false));
        }
        node
    }

    fn read_size(&mut self, token: u8) -> usize {
        match token {
            LIST_EMPTY => 0,
            LIST_8 => self.reader.read_i8().unwrap() as usize,
            LIST_16 => self.reader.read_i16::<BigEndian>().unwrap() as usize,
            _ => panic!("Invalid list token"),
        }
    }

    fn read_attributes(&mut self, size: usize) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        for _ in 0..((size - 1) >> 1) {
            let key = self.read_string();
            let value = self.read(true);
            map.insert(key.clone(), value);
        }
        map
    }

    fn read_string(&mut self) -> String {
        match self.read(true) {
            Value::Str(s) => s,
            Value::Null => "".to_string(),
            _ => panic!("Expected string"),
        }
    }

    fn read(&mut self, parse_bytes: bool) -> Value {
        let tag = self.reader.read_u8().unwrap();

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
            NIBBLE_8 => self.read_packed8(tag),
            HEX_8 => self.read_packed8(tag),
            _ => {
                self.read_string_from_token(tag)
            },
        }
    }

    fn read_packed8(&mut self, tag: u8) -> Value {
        let start_byte = self.reader.read_u8().unwrap();

        let mut data = Vec::new();
        for _ in 0..(start_byte & 127) {
            let curr_byte = self.reader.read_u8().unwrap();
            let lower = self.unpack_byte(tag, (curr_byte & 0xF0) >> 4);
            let upper = self.unpack_byte(tag, curr_byte & 0x0F);
            data.push(lower);
            data.push(upper);
        }

        let mut result = String::from_utf8_lossy(&data).into_owned();
        if start_byte >> 7 != 0 {
            result.pop();
        }
        Value::Str(result)
    }

    fn unpack_byte(&mut self, tag: u8, value: u8) -> u8 {
        match tag {
            NIBBLE_8 => self.unpack_nibble(value),
            HEX_8 => self.unpack_hex(value),
            _ => panic!("Invalid tag"),
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

    fn unpack_nibble(&mut self, value: u8) -> u8 {
        match value {
            0..=9 => b'0' + value,
            10 => b'-',
            11 => b'.',
            15 => 0,
            _ => panic!("unpackNibble with value {}", value)
        }
    }

    fn unpack_hex(&mut self, value: u8) -> u8 {
        match value {
            0..=9 => b'0' + value,
            10..=15 => b'A' + (value - 10),
            _ => panic!("unpackHex with value {}", value)
        }
    }

    fn read_string_from_token(&mut self, tag: u8) -> Value {
        if tag < DICTIONARY_0 || tag > DICTIONARY_3 {
            return Value::Str(SINGLE_BYTE_TOKENS[tag as usize].to_string());
        }

        let i = self.reader.read_u8().unwrap() as usize;
        Value::Str(DOUBLE_BYTE_TOKENS[tag as usize - DICTIONARY_0 as usize][i].to_string())
    }
}


pub fn unpack(data: &[u8]) -> Vec<u8> {
    let mut e = ZlibDecoder::new(data);
    let mut s = String::new();
    e.read_to_string(&mut s).unwrap();
    s.into_bytes()
}