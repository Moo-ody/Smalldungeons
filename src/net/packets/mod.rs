use crate::net::var_int::read_var_int;
use anyhow::{bail, Context};
use bytes::BytesMut;

pub mod packet;
pub mod client_bound;
pub mod server_bound;
pub mod packet_context;
pub mod packet_registry;
pub mod packet_write;

// maybe own file?
pub fn read_string_from_buf(buf: &mut BytesMut, max_length: i32) -> anyhow::Result<String> {
    let len = read_var_int(buf).context("Failed to read string length")?;
    if len > max_length * 4 {
        bail!("String too long. {:?} / {}", len, max_length * 4);
    }
    if len < 0 {
        bail!("String length is less than 0???")
    }

    let string = String::from_utf8(buf.split_to(len as usize).to_vec())?;

    if string.len() > max_length as usize {
        bail!("String too long. {:?} > {}", len, max_length);
    }
    Ok(string)
}