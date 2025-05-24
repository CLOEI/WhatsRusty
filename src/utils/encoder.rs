use std::{collections::HashMap, io::Write};

use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

use crate::types::jid::JID;

use super::{decoder::{Node, Value}, token};

pub struct BinaryEncoder {
    buffer: Vec<u8>,
}

impl BinaryEncoder {
    pub fn new() -> Self {
        BinaryEncoder { buffer: Vec::new() }
    }

    pub fn write_node(&mut self, node: &Node) -> Vec<u8> {
        self.buffer.write_u8(0).unwrap();
        if node.tag == "0" {
            self.buffer.write_u8(token::LIST_8).unwrap();
            self.buffer.write_u8(token::LIST_EMPTY).unwrap();
            return self.buffer.clone();
        }

        let has_content = match node.content {
            Some(_) => 1,
            None => 0,
        };

        self.write_list_start(2 * self.count_attributes(&node.attributes) + 1 + has_content);
        self.write_string(node.tag.clone());
        self.write_attributes(&node.attributes);

        if let Some(content) = &node.content {
            self.write(content);
        }

        self.buffer.clone()
    }
    
    fn count_attributes(&self, attributes: &HashMap<String, Value>) -> usize {
        attributes.iter()
            .filter(|(_, val)| {
                match val {
                    Value::Str(s) if s.is_empty() => false,
                    Value::Null => false,
                    _ => true
                }
            })
            .count()
    }

    fn write_attributes(&mut self, attributes: &HashMap<String, Value>) {
        for (key, val) in attributes {
            match val {
                Value::Str(s) if s.is_empty() => continue,
                Value::Null => continue,
                _ => {}
            }
    
            self.write_string(key.clone());
            self.write(val);
        }
    }

    fn write(&mut self, value: &Value) {
        match value {
            Value::Str(s) => self.write_string(s.clone()),
            Value::Jid(jid) => self.write_jid(&jid),
            Value::Bytes(bytes) => self.write_bytes(bytes),
            _ => panic!("{:?} Not handled", value)
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.write_byte_length(bytes.len());
        self.buffer.write_all(bytes).unwrap();
    }

    fn write_byte_length(&mut self, length: usize) {
        if length < 256 {
            self.buffer.write_u8(token::BINARY_8).unwrap();
            self.buffer.write_u8(length as u8).unwrap();
        } else if length < (1 << 20) {
            self.buffer.write_u8(token::BINARY_20).unwrap();
            self.buffer.write_all(&[(length >> 16) as u8 & 0x0F, (length >> 8) as u8 & 0xFF, length as u8 & 0xFF]).unwrap();
        } else if length < i32::MAX as usize {
            self.buffer.write_u8(token::BINARY_32).unwrap();
            self.buffer.write_i32::<BigEndian>(length as i32).unwrap();
        } else {
            panic!("length is too large: {}", length);
        }
    }

    fn write_jid(&mut self, jid: &JID) {
        let server = jid.server.as_deref().unwrap_or("");
        let user = jid.user.as_deref().unwrap_or("");
        let device = jid.device.unwrap_or(0);
        let integrator = jid.integrator.unwrap_or(0);
        let raw_agent = jid.raw_agent.unwrap_or(0);
    
        if (server == "s.whatsapp.net" && device > 0) || 
           server == "lid" || 
           server == "hosted" {
            self.buffer.write_u8(token::AD_JID).unwrap();
            self.buffer.write_u8(raw_agent).unwrap();
            self.buffer.write_u8(device as u8).unwrap();
            self.write_string(user.to_string());
        }
        else if server == "msgr" {
            self.buffer.write_u8(token::FB_JID).unwrap();
            self.write_string(user.to_string());
            self.buffer.write_i16::<LittleEndian>(device as i16).unwrap();
            self.write_string(server.to_string());
        }
        else if server == "interop" {
            self.buffer.write_u8(token::INTEROP_JID).unwrap();
            self.write_string(user.to_string());
            self.buffer.write_i16::<LittleEndian>(device as i16).unwrap();
            self.buffer.write_i16::<LittleEndian>(integrator as i16).unwrap();
            self.write_string(server.to_string());
        }
        else {
            self.buffer.write_u8(token::JID_PAIR).unwrap();
            if user.is_empty() {
                self.buffer.write_u8(token::LIST_EMPTY).unwrap();
            } else {
                self.write_string(user.to_string());
            }
            self.write_string(server.to_string());
        }
    }

    fn write_string(&mut self, data: String) {
        match token::SINGLE_BYTE_TOKENS.iter().position(|&s| s == &data) {
            Some(index) => {
                self.buffer.write_u8(index as u8).unwrap();
            }
            None => {
                match token::DOUBLE_BYTE_TOKENS.iter().enumerate().find(|(_, tokens)| {
                    tokens.iter().position(|t| t == &data).is_some()
                }) {
                    Some((dict_index, tokens)) => {
                        let token_index: usize = tokens.iter().position(|t| t == &data).unwrap();
                        self.buffer.write_u8(token::DICTIONARY_0 + dict_index as u8).unwrap();
                        self.buffer.write_u8(token_index as u8).unwrap();
                    }
                    None => {
                        if self.validate_nibble(&data) {
                            self.write_packed_bytes(&data, token::NIBBLE_8);
                        } else if self.validate_hex(&data) {
                            self.write_packed_bytes(&data, token::HEX_8);
                        } else {
                            self.write_byte_length(data.len());
                            self.buffer.write_all(data.as_bytes()).unwrap();
                        }
                    }
                }
            }
        }
    }

    fn validate_hex(&self, value: &str) -> bool {
        if value.len() > 127 { 
            return false;
        }
        for c in value.chars() {
            if !c.is_ascii_digit() && !(c >= 'A' && c <= 'F') {
                return false;
            }
        }
        true
    }

    fn write_packed_bytes(&mut self, value: &str, data_type: u8) {
        if value.len() > 127 {
            panic!("too many bytes to pack: {}", value.len());
        }

        self.buffer.write_u8(data_type).unwrap();
        let rounded_length = ((value.len() as f64 / 2.0).ceil() as u8) | if value.len() % 2 != 0 { 128 } else { 0 };
        self.buffer.write_u8(rounded_length).unwrap();

        match data_type {
            token::NIBBLE_8 => {
                for i in 0..(value.len() / 2) {
                    let first = value.chars().nth(2 * i).unwrap() as u8;
                    let second = value.chars().nth(2 * i + 1).unwrap() as u8;
                    let packed = (self.pack_nibble(first) << 4) | self.pack_nibble(second);
                    self.buffer.write_u8(packed).unwrap();
                }
                if value.len() % 2 != 0 {
                    let last = value.chars().nth(value.len() - 1).unwrap() as u8;
                    let packed = (self.pack_nibble(last) << 4) | self.pack_nibble(0);
                    self.buffer.write_u8(packed).unwrap();
                }
            }
            token::HEX_8 => {
                for i in 0..(value.len() / 2) {
                    let first = value.chars().nth(2 * i).unwrap() as u8;
                    let second = value.chars().nth(2 * i + 1).unwrap() as u8;
                    let packed = (self.pack_hex(first) << 4) | self.pack_hex(second);
                    self.buffer.write_u8(packed).unwrap();
                }
                if value.len() % 2 != 0 {
                    let last = value.chars().nth(value.len() - 1).unwrap() as u8;
                    let packed = (self.pack_hex(last) << 4) | self.pack_hex(0);
                    self.buffer.write_u8(packed).unwrap();
                }
            }
            _ => panic!("Invalid packed bytes type")
        }
        
    }

    fn validate_nibble(&self, value: &str) -> bool {
        if value.len() > 127 {
            return false;
        }
        for c in value.chars() {
            if !c.is_ascii_digit() && c != '-' && c != '.' {
                return false;
            }
        }
        true
    }

    fn pack_nibble(&self, value: u8) -> u8 {
        match value {
            b'-' => 10,
            b'.' => 11,
            0 => 15,
            b'0'..=b'9' => value - b'0',
            _ => panic!("invalid string to pack as nibble: {} / '{}'", value, value as char)
        }
    }

    fn pack_hex(&self, value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'A'..=b'F' => 10 + value - b'A',
            0 => 15,
            _ => panic!("invalid string to pack as hex: {} / '{}'", value, value as char)
        }
    }

    fn write_list_start(&mut self, list_size: usize) {
        if list_size == 0 {
            self.buffer.write_u8(token::LIST_EMPTY).unwrap();
        } else if list_size < 256 {
            self.buffer.write_u8(token::LIST_8).unwrap();
            self.buffer.write_i8(list_size as i8).unwrap();
        } else {
            self.buffer.write_u8(token::LIST_16).unwrap();
            self.buffer.write_i16::<LittleEndian>(list_size as i16).unwrap();
        }
    }
}