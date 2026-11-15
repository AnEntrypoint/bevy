use serde_json::{Value, json};
use std::io::Cursor;
use std::path::Path;

const IEND_CHUNK: [u8; 12] = [0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82];

pub fn check(path: &Path) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let has_trailer = bytes.ends_with(&IEND_CHUNK);
    let mut reader = png::Decoder::new(Cursor::new(&bytes))
        .read_info()
        .map_err(|error| error.to_string())?;
    let (width, height, color_type, bit_depth) = {
        let info = reader.info();
        (info.width, info.height, info.color_type, info.bit_depth)
    };
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "decoded image size overflows".to_string())?;
    let mut pixels = vec![0; size];
    let frame = reader
        .next_frame(&mut pixels)
        .map_err(|error| error.to_string())?;
    reader.finish().map_err(|error| error.to_string())?;
    if !has_trailer {
        return Err("image decodes but the file does not end with an IEND chunk".to_string());
    }
    Ok(json!({
        "width": width,
        "height": height,
        "color_type": format!("{color_type:?}"),
        "bit_depth": format!("{bit_depth:?}"),
        "decoded_bytes": frame.buffer_size(),
    }))
}
