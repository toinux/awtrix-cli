//! Save the AWTRIX framebuffer as an RGB PNG without returning pixel data.
use clap::Subcommand;
use serde_json::{json, Value};
use std::{fs::File, io::BufWriter, path::PathBuf};

const MAX_PIXELS: u64 = 4_194_304;

#[derive(Subcommand)]
pub enum Command {
    /// Capture the current framebuffer to a PNG.
    #[command(
        about = "Capture the current framebuffer to PNG",
        long_about = "Framebuffer captures do not reproduce physical brightness or LED corrections and are not synchronized deterministically to a frame."
    )]
    Capture {
        #[arg(long)]
        output: PathBuf,
    },
}

pub fn run(command: &Command, api: &crate::ApiClient) -> crate::CliResult<Value> {
    match command {
        Command::Capture { output } => {
            let bytes = api.raw_bytes_get("/api/v1/screen")?;
            let frame: Value = serde_json::from_slice(&bytes)
                .map_err(|_| ("INVALID_RESPONSE", "invalid framebuffer JSON".into()))?;
            let width = frame
                .get("width")
                .and_then(Value::as_u64)
                .filter(|v| *v > 0 && *v <= u32::MAX as u64)
                .ok_or(("INVALID_RESPONSE", "invalid framebuffer width".into()))?;
            let height = frame
                .get("height")
                .and_then(Value::as_u64)
                .filter(|v| *v > 0 && *v <= u32::MAX as u64)
                .ok_or(("INVALID_RESPONSE", "invalid framebuffer height".into()))?;
            let count = width
                .checked_mul(height)
                .filter(|n| *n <= MAX_PIXELS)
                .ok_or((
                    "INVALID_RESPONSE",
                    "framebuffer dimensions exceed supported limit".into(),
                ))?;
            let pixels = frame
                .get("pixels")
                .and_then(Value::as_array)
                .filter(|pixels| pixels.len() as u64 == count)
                .ok_or((
                    "INVALID_RESPONSE",
                    "framebuffer pixel count does not match dimensions".into(),
                ))?;
            let mut rgb = Vec::with_capacity(count as usize * 3);
            for pixel in pixels {
                let channels = pixel
                    .as_array()
                    .filter(|channels| channels.len() == 3)
                    .ok_or((
                        "INVALID_RESPONSE",
                        "framebuffer pixel must contain three RGB channels".into(),
                    ))?;
                for channel in channels {
                    let value = channel.as_u64().filter(|v| *v <= 255).ok_or((
                        "INVALID_RESPONSE",
                        "RGB channels must be decimal integers from 0 to 255".into(),
                    ))?;
                    rgb.push(value as u8);
                }
            }
            // API contract for this CLI slice: /api/v1/screen returns width, height and
            // row-major pixels as [red, green, blue] decimal integer triplets.
            let file = File::create(output).map_err(|_| {
                (
                    "FILE_WRITE",
                    format!("could not create output file {}", output.display()),
                )
            })?;
            let mut encoder = png::Encoder::new(BufWriter::new(file), width as u32, height as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|_| ("FILE_WRITE", "could not write PNG header".into()))?;
            writer
                .write_image_data(&rgb)
                .map_err(|_| ("FILE_WRITE", "could not write PNG pixels".into()))?;
            Ok(json!({"path":output,"format":"png","width":width,"height":height}))
        }
    }
}

pub fn describe() -> Value {
    json!({"command":"screen capture","parameters":{"--output":"required PNG destination path"},"inputs":["GET /api/v1/screen framebuffer JSON"],"outputs":["path, format, width, height"],"examples":["awtrix screen capture --output capture.png","awtrix --json screen capture --output capture.png"],"prerequisites":["AWTRIX NG framebuffer route"],"limitations":["Framebuffer does not reproduce physical brightness or LED corrections and is not synchronized deterministically to a frame"]})
}
