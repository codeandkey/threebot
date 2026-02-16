// Encode a varint in mumble format (64-bit version)
pub fn encode_varint_long(value: u64) -> Vec<u8> {
    // TODO: actual varint encoding,

    let mut out = vec![0b11110100];
    out.extend(value.to_be_bytes());

    out
}
