//! Save the AWTRIX framebuffer as an RGB PNG without returning pixel data.
use clap::Subcommand;
use serde_json::{json, Value};
use std::{io::Write, path::PathBuf, time::Duration};
use tempfile::NamedTempFile;

const MAX_PIXELS: u64 = 4_194_304;
// Ten bytes per possible packed decimal pixel (digits plus separator), plus bounded object overhead.
const MAX_RESPONSE_BYTES: u64 = MAX_PIXELS * 10 + 4096;

#[derive(Subcommand)]
pub enum Command {
    /// Capture the current framebuffer as PNG, true-color ANSI, or emoji.
    #[command(
        about = "Capture the current framebuffer as PNG, ANSI, or emoji",
        long_about = "PNG is the default and requires --output. ANSI uses exact 24-bit RGB colors and the upper-half block; emoji maps each pixel to the nearest representative palette emoji by squared Euclidean RGB distance. ANSI stdout needs a true-color terminal; emoji stdout is a text rendering. --output atomically replaces a file for ANSI and emoji. Human stdout contains only the rendering. With --json and no output, fields are format,width,height,text; with --output, fields are path,format,width,height. --fields projects those results. Examples: screen capture --format emoji; screen capture --format emoji --output capture.txt. Color approximation cannot reproduce physical brightness or LED corrections; captures are not synchronized deterministically to a frame."
    )]
    Capture {
        /// Capture format.
        #[arg(long, value_enum, default_value = "png")]
        format: Format,
        /// Optional destination for rendered text; required for PNG and atomically replaces existing files.
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Png,
    Ansi,
    Emoji,
}

pub fn run(command: &Command, api: &crate::ApiClient) -> crate::CliResult<Value> {
    run_with_timeout(command, api, Duration::from_secs(86400))
}
pub(crate) fn validate(command: &Command) -> crate::CliResult<()> {
    match command {
        Command::Capture {
            format: Format::Png,
            output: None,
        } => Err(("ARGUMENT", "--output is required for PNG capture".into())),
        _ => Ok(()),
    }
}
pub(crate) fn run_with_timeout(
    command: &Command,
    api: &crate::ApiClient,
    timeout: Duration,
) -> crate::CliResult<Value> {
    validate(command)?;
    match command {
        Command::Capture { output, format } => {
            let bytes = api.raw_bytes_get_with_timeout(
                "/api/v1/display/screen",
                MAX_RESPONSE_BYTES,
                timeout,
            )?;
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
            let mut packed_pixels = Vec::with_capacity(count as usize);
            for pixel in pixels {
                let packed = pixel
                    .as_u64()
                    .filter(|value| *value <= 0x00ff_ffff)
                    .ok_or((
                        "INVALID_RESPONSE",
                        "packed RGB pixels must be unsigned decimal integers from 0 to 16777215"
                            .into(),
                    ))? as u32;
                rgb.extend_from_slice(&[
                    ((packed >> 16) & 0xff) as u8,
                    ((packed >> 8) & 0xff) as u8,
                    (packed & 0xff) as u8,
                ]);
                packed_pixels.push(packed);
            }
            if *format == Format::Ansi {
                let mut text = String::new();
                for y in (0..height as usize).step_by(2) {
                    for x in 0..width as usize {
                        let upper = packed_pixels[y * width as usize + x];
                        let lower = if y + 1 < height as usize {
                            packed_pixels[(y + 1) * width as usize + x]
                        } else {
                            0
                        };
                        text.push_str(&format!(
                            "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                            upper >> 16 & 255,
                            upper >> 8 & 255,
                            upper & 255,
                            lower >> 16 & 255,
                            lower >> 8 & 255,
                            lower & 255
                        ));
                    }
                    text.push_str("\x1b[0m\n");
                }
                if let Some(path) = output {
                    persist_sibling(path, text.as_bytes())?;
                    return Ok(json!({"path":path,"format":"ansi","width":width,"height":height}));
                }
                return Ok(
                    json!({"format":"ansi","width":width,"height":height,"text":text,"__raw_stdout":true}),
                );
            }
            if *format == Format::Emoji {
                let mut text = String::new();
                for (index, pixel) in packed_pixels.iter().enumerate() {
                    text.push(nearest_emoji(*pixel));
                    if index % width as usize == width as usize - 1 {
                        text.push('\n');
                    }
                }
                if let Some(path) = output {
                    persist_sibling(path, text.as_bytes())?;
                    return Ok(json!({"path":path,"format":"emoji","width":width,"height":height}));
                }
                return Ok(
                    json!({"format":"emoji","width":width,"height":height,"text":text,"__raw_stdout":true}),
                );
            }
            // Encode everything in memory before opening any file, so malformed input never
            // truncates a previously captured image.
            let mut encoded = Vec::new();
            let mut encoder = png::Encoder::new(&mut encoded, width as u32, height as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|_| ("FILE_WRITE", "could not write PNG header".into()))?;
            writer
                .write_image_data(&rgb)
                .map_err(|_| ("FILE_WRITE", "could not write PNG pixels".into()))?;
            drop(writer);
            let output = output
                .as_ref()
                .ok_or(("ARGUMENT", "--output is required for PNG capture".into()))?;
            persist_sibling(output, &encoded)?;
            Ok(json!({"path":output,"format":"png","width":width,"height":height}))
        }
    }
}

const EMOJI_PALETTE: &[(u32, char)] = &[
    (0x000000, '⬛'),
    (0xffffff, '⬜'),
    (0xff0000, '🟥'),
    (0xff8000, '🟧'),
    (0xffff00, '🟨'),
    (0x00ff00, '🟩'),
    (0x0064ff, '🟦'),
    (0xc800ff, '🟪'),
    (0x804000, '🟫'),
];

/// Choose the closest representative by squared Euclidean distance in RGB; palette order breaks ties.
fn nearest_emoji(pixel: u32) -> char {
    let channel = |color: u32, shift: u32| ((color >> shift) & 0xff) as i32;
    EMOJI_PALETTE
        .iter()
        .min_by_key(|(color, _)| {
            [16, 8, 0]
                .into_iter()
                .map(|shift| {
                    let diff = channel(*color, shift) - channel(pixel, shift);
                    diff * diff
                })
                .sum::<i32>()
        })
        .map(|(_, emoji)| *emoji)
        .unwrap_or('⬛')
}

/// Capture one bounded framebuffer to a durable caller-selected PNG path.
pub(crate) fn capture_png(
    api: &crate::ApiClient,
    output: &std::path::Path,
    timeout: Duration,
) -> crate::CliResult<Value> {
    let command = Command::Capture {
        format: Format::Png,
        output: Some(output.to_path_buf()),
    };
    run_with_timeout(&command, api, timeout)
}

/// Persist a fully encoded PNG through tempfile's cross-platform atomic replacement operation.
pub(crate) fn persist_sibling(path: &PathBuf, bytes: &[u8]) -> crate::CliResult<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut temporary = NamedTempFile::new_in(parent).map_err(|_| {
        (
            "FILE_WRITE",
            format!(
                "could not create temporary output beside {}",
                path.display()
            ),
        )
    })?;
    temporary
        .write_all(bytes)
        .map_err(|_| ("FILE_WRITE", "could not write temporary PNG".into()))?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| ("FILE_WRITE", "could not flush temporary PNG".into()))?;
    temporary.persist(path).map_err(|_| {
        (
            "FILE_WRITE",
            format!("could not atomically persist PNG at {}", path.display()),
        )
    })?;
    Ok(())
}

pub fn describe() -> Value {
    json!({"command":"screen capture","parameters":{"--format":"png (default, requires --output), ansi, or emoji (ANSI/emoji allow optional --output)","--output":"PNG is required; ANSI and emoji are optional. Atomically replaces destination; parent directory must exist and be writable"},"inputs":["GET /api/v1/display/screen; row-major pixels are unsigned decimal packed 0xRRGGBB"],"outputs":["path, format, width, height"],"stdout_json_fields":["format","width","height","text"],"examples":["awtrix-cli screen capture --output capture.png","awtrix-cli screen capture --format ansi","awtrix-cli screen capture --format emoji","awtrix-cli screen capture --format emoji --output capture.txt","awtrix-cli --json screen capture --format emoji"],"prerequisites":["AWTRIX NG framebuffer route", "existing writable destination parent directory; missing parents are not created"],"semantics":["PNG is the default; ANSI emits exact 24-bit RGB using ▀, with upper pixel foreground, lower pixel background, black for a missing odd-height lower pixel, and a reset at each output line", "ANSI stdout is a rendering only and requires a true-color ANSI-compatible terminal", "Emoji emits one palette square per pixel and newline per row; nearest representative uses squared Euclidean RGB distance, with palette order resolving ties", "Palette RGB representatives: black #000000 ⬛, white #FFFFFF ⬜, red #FF0000 🟥, orange #FF8000 🟧, yellow #FFFF00 🟨, green #00FF00 🟩, blue #0064FF 🟦, purple #C800FF 🟪, brown #804000 🟫", "Output files are rendered before sibling-temp atomic replacement", "--fields projects the stdout JSON or file receipt; projection failure does not undo replacement"],"limitations":["Framebuffer does not reproduce physical brightness or LED corrections and is not synchronized deterministically to a frame"]})
}
