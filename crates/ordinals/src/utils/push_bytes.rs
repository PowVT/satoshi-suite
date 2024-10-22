use std::error::Error;

use bitcoin::script::PushBytesBuf;

pub fn bytes_to_push_bytes(bytes: &[u8]) -> Result<PushBytesBuf, Box<dyn Error>> {
    let mut push_bytes = PushBytesBuf::with_capacity(bytes.len());
    push_bytes.extend_from_slice(bytes)?;

    Ok(push_bytes)
}
