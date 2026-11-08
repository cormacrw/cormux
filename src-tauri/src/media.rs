//! The image and video files a thread shows, by extension, and how big they may be. The
//! webview gets the extensions as `MEDIA_EXTENSIONS`.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Images read whole into memory: sent to the model inline, or shown as a data URL. Larger ones
/// go by path (to the model) or through the asset protocol (to the webview).
pub const INLINE_IMAGE_LIMIT: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Image,
    Video,
}

impl MediaKind {
    /// The largest file of this kind a reply may show (images inline, recordings in a player),
    /// so a stray reference can't copy the agent's whole disk into the app folder.
    pub fn shown_limit(self) -> u64 {
        match self {
            Self::Image => 20 * 1024 * 1024,
            Self::Video => 200 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MediaFormat {
    pub kind: MediaKind,
    /// The extension a copy is saved under: `jpeg` files become `jpg`.
    pub canonical: &'static str,
    pub mime: &'static str,
    /// Whether a file's first bytes are this format's, so a renamed text file is refused.
    pub magic: fn(&[u8]) -> bool,
}

impl PartialEq for MediaFormat {
    fn eq(&self, other: &Self) -> bool {
        (self.kind, self.canonical, self.mime) == (other.kind, other.canonical, other.mime)
    }
}

const FORMATS: &[(&str, MediaFormat)] = &[
    (
        "png",
        image("png", "image/png", |head| {
            head.starts_with(&[0x89, 0x50, 0x4E, 0x47])
        }),
    ),
    ("jpg", image("jpg", "image/jpeg", is_jpeg)),
    ("jpeg", image("jpg", "image/jpeg", is_jpeg)),
    (
        "gif",
        image("gif", "image/gif", |head| head.starts_with(b"GIF8")),
    ),
    (
        "webp",
        image("webp", "image/webp", |head| {
            head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP"
        }),
    ),
    ("mp4", video("mp4", "video/mp4", is_iso_media)),
    (
        "webm",
        video("webm", "video/webm", |head| {
            head.starts_with(&[0x1A, 0x45, 0xDF, 0xA3])
        }),
    ),
    ("mov", video("mov", "video/quicktime", is_iso_media)),
    ("m4v", video("m4v", "video/x-m4v", is_iso_media)),
];

fn is_jpeg(head: &[u8]) -> bool {
    head.starts_with(&[0xFF, 0xD8, 0xFF])
}

/// MP4, QuickTime and M4V files open with an `ftyp` box.
fn is_iso_media(head: &[u8]) -> bool {
    head.len() >= 8 && &head[4..8] == b"ftyp"
}

const fn image(
    canonical: &'static str,
    mime: &'static str,
    magic: fn(&[u8]) -> bool,
) -> MediaFormat {
    MediaFormat {
        kind: MediaKind::Image,
        canonical,
        mime,
        magic,
    }
}

const fn video(
    canonical: &'static str,
    mime: &'static str,
    magic: fn(&[u8]) -> bool,
) -> MediaFormat {
    MediaFormat {
        kind: MediaKind::Video,
        canonical,
        mime,
        magic,
    }
}

pub fn image_data_url(path: &Path) -> Option<String> {
    let format = format_of(path.to_str()?).filter(|format| format.kind == MediaKind::Image)?;
    if std::fs::metadata(path).ok()?.len() > INLINE_IMAGE_LIMIT {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(format!(
        "data:{};base64,{}",
        format.mime,
        STANDARD.encode(bytes)
    ))
}

pub fn format_of(name: &str) -> Option<MediaFormat> {
    let extension = std::path::Path::new(name)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    FORMATS
        .iter()
        .find(|(known, _)| *known == extension)
        .map(|(_, format)| *format)
}

/// Each extension a thread shows, and as what.
pub fn extension_kinds() -> std::collections::BTreeMap<&'static str, MediaKind> {
    FORMATS
        .iter()
        .map(|(extension, format)| (*extension, format.kind))
        .collect()
}

fn extensions(kind: MediaKind) -> Vec<&'static str> {
    FORMATS
        .iter()
        .filter(|(_, format)| format.kind == kind)
        .map(|(extension, _)| *extension)
        .collect()
}

/// Told to the agent when its session starts, so it knows the thread renders the media it
/// references in Markdown (the same fix t3code made for the same problem). Absolute paths work
/// from anywhere: the core copies the file before serving it.
pub fn instruction() -> String {
    format!(
        "This conversation is rendered by Cormux, which shows Markdown images and linked videos inline in the thread. Embed a file in your reply as plain Markdown: ![short description](/absolute/path/to/screenshot.png) for images, [short description](/absolute/path/to/recording.mp4) for videos ({}; {}). Use the absolute path the tool saved to. When the user asked for a change with a visible result (UI, layout, styling, rendered output), your closing reply must embed a screenshot or recording of the finished result without being asked. For other work, embed a screenshot, recording, plot or diagram only when it helps the user, and do not embed every file you touch. Cormux caches images by path, so every embedded image must be a new file at a path not yet embedded in this conversation, for example with a version or timestamp suffix. Never overwrite a file and embed it again; if a tool always writes to the same path, copy the result to a new unique filename and embed the copy. Cormux also renders fenced mermaid blocks as diagrams, and a fenced chart block as a chart: JSON like {{\"type\":\"bar\",\"title\":\"Bundle size (kB)\",\"labels\":[\"a\",\"b\"],\"series\":[{{\"name\":\"Before\",\"values\":[1,2]}}]}}, where type is bar or line. Use them when a diagram or chart says it better than prose.",
        extensions(MediaKind::Image).join(", "),
        extensions(MediaKind::Video).join(", "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_go_by_extension_in_any_case() {
        assert_eq!(
            format_of("Shot.PNG").map(|format| format.mime),
            Some("image/png")
        );
        assert_eq!(
            format_of("a.jpeg").map(|format| format.canonical),
            Some("jpg")
        );
        assert_eq!(
            format_of("/tmp/clip.m4v").map(|format| format.kind),
            Some(MediaKind::Video)
        );
        assert_eq!(format_of("notes.pdf"), None);
        assert_eq!(format_of("README"), None);
    }
}
