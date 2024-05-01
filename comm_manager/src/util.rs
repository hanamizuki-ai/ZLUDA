pub fn encode_7bit(mut num: u32) -> Vec<u8> {
    let mut result = Vec::new();
    while num >= 0x80 {
        result.push((num & 0x7F | 0x80) as u8);
        num >>= 7;
    }
    result.push(num as u8);
    result
}

pub fn encode_7bit_string(string: Option<&str>) -> Vec<u8> {
    if let Some(string) = string {
        let encoded = string.as_bytes();
        let mut result = encode_7bit(encoded.len() as u32);
        result.extend_from_slice(encoded);
        result
    } else {
        encode_7bit(0)
    }
}
