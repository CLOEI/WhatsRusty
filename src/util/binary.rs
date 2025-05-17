use std::io::Read;

use flate2::read::ZlibDecoder;

pub fn unpack(data: Vec<u8>) -> Vec<u8> {
    if data[0] & 2 > 0 {
        let mut e = ZlibDecoder::new(&data[1..]);
        let mut s = String::new();
        e.read_to_string(&mut s).unwrap();
        s.into_bytes()
    } else {
        data
    }
}

