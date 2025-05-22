use byteorder::{LittleEndian, WriteBytesExt};

use super::{decoder::Node, token};

struct BinaryEncoder {
    buffer: Vec<u8>,
}

impl BinaryEncoder {
    fn new() -> Self {
        BinaryEncoder { buffer: Vec::new() }
    }

    pub fn write_node(&mut self, node: &Node) {
        if node.tag == "0" {
            self.buffer.write_u8(token::LIST_8).unwrap();
            self.buffer.write_u8(token::LIST_EMPTY).unwrap();
            return
        }

        let has_content = match node.content {
            Some(_) => 1,
            None => 0,
        };

        self.write_list_start(2 * node.attributes.len() + 1 + has_content);
        self.write_string(node.tag.clone());
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

                        }
                    }
                }
            }
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