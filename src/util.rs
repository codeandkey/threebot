// Encode a varint in mumble format.
pub fn encode_varint_16(value: u16) -> Vec<u8> {
    let mut out = Vec::new();
    let mut v = value;

    // Mumble varint encoding: 7 bits per byte, MSB indicates continuation
    while v > 0 {
        let byte = (v & 0x7F) as u8;
        v >>= 7;
        if v > 0 {
            out.push(byte | 0x80);
        } else {
            out.push(byte);
        }
    }

    out
}

// Encode a varint in mumble format (64-bit version)
pub fn encode_varint_long(value: u64) -> Vec<u8> {
    // TODO: actual varint encoding,

    let mut out = vec![0b11110100];
    out.extend(value.to_be_bytes());

    out
}
