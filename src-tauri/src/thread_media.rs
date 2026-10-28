//! Media an agent shares from its reply by referencing a local file (a screenshot or a
//! recording a browser tool saved). The referenced file is copied into the app's media
//! folder — the only folder the asset protocol may serve — so it survives worktree
//! teardown and the webview never loads an arbitrary path.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::Result;

/// Images are shown inline in the thread, recordings in a player; both are capped so a
/// stray reference can't copy the agent's whole disk into the app folder.
const IMAGE_LIMIT_BYTES: u64 = 20 * 1024 * 1024;
const VIDEO_LIMIT_BYTES: u64 = 200 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMedia {
    /// `image` or `video`.
    pub kind: String,
    pub name: String,
    /// The copy inside the media folder, loaded through the asset protocol.
    pub path: String,
    /// A data URL for images small enough to show directly; `None` otherwise.
    pub preview_url: Option<String>,
}

fn media_type(name: &str) -> Option<(&'static str, &'static str)> {
    // (kind, canonical extension)
    match Path::new(name)
        .extension()?
        .to_str()?
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some(("image", "png")),
        "jpg" | "jpeg" => Some(("image", "jpg")),
        "gif" => Some(("image", "gif")),
        "webp" => Some(("image", "webp")),
        "mp4" => Some(("video", "mp4")),
        "webm" => Some(("video", "webm")),
        "mov" => Some(("video", "mov")),
        "m4v" => Some(("video", "m4v")),
        _ => None,
    }
}

/// The file's own bytes must agree with its extension, so a renamed text file is refused.
fn sniff_matches(kind: &str, extension: &str, head: &[u8]) -> bool {
    match (kind, extension) {
        ("image", "png") => head.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        ("image", "jpg") => head.starts_with(&[0xFF, 0xD8, 0xFF]),
        ("image", "gif") => head.starts_with(b"GIF8"),
        ("image", "webp") => head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP",
        ("video", "webm") => head.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]),
        ("video", _) => head.len() >= 8 && &head[4..8] == b"ftyp",
        _ => false,
    }
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (
                (bytes[index + 1] as char).to_digit(16),
                (bytes[index + 2] as char).to_digit(16),
            )
        {
            out.push((hi * 16 + lo) as u8);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `file://` URLs lose their scheme; everything else is taken as a path as written.
fn source_path(source: &str) -> PathBuf {
    let trimmed = source.trim();
    if let Some(rest) = trimmed.strip_prefix("file://") {
        let rest = rest.strip_prefix("localhost").unwrap_or(rest);
        PathBuf::from(percent_decode(rest))
    } else {
        PathBuf::from(percent_decode(trimmed))
    }
}

/// Relative references resolve against the thread's worktree, the agent's working folder.
fn resolve_path(source: &str, workdir: Option<&Path>) -> Option<PathBuf> {
    let path = source_path(source);
    let joined = if path.is_absolute() {
        path
    } else {
        workdir?.join(path)
    };
    joined.canonicalize().ok().filter(|path| path.is_file())
}

fn safe_stem(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "media".into());
    let cleaned: String = stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('-');
    if cleaned.is_empty() {
        "media".into()
    } else {
        cleaned.chars().take(60).collect()
    }
}

/// Copies the referenced file into `media_dir` and describes how to show it. `Ok(None)`
/// when the reference isn't a showable image or recording (missing, oversized, or a
/// file whose bytes don't match its name) — the reply simply keeps its prose.
pub fn resolve(
    media_dir: &Path,
    source: &str,
    workdir: Option<&Path>,
) -> Result<Option<ThreadMedia>> {
    let Some(path) = resolve_path(source, workdir) else {
        return Ok(None);
    };
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let Some((kind, extension)) = media_type(&name) else {
        return Ok(None);
    };
    let Ok(metadata) = std::fs::metadata(&path) else {
        return Ok(None);
    };
    let limit = if kind == "image" {
        IMAGE_LIMIT_BYTES
    } else {
        VIDEO_LIMIT_BYTES
    };
    if metadata.len() > limit {
        return Ok(None);
    }
    let head = read_head(&path, 12);
    if !sniff_matches(kind, extension, &head) {
        return Ok(None);
    }

    // One copy per file version: re-renders of a thread reuse it, an overwritten
    // screenshot (new mtime or size) lands as a fresh copy.
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| (duration.as_secs(), duration.subsec_nanos()))
        .unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    modified.hash(&mut hasher);
    let stored_name = format!(
        "{:016x}-{}.{}",
        hasher.finish(),
        safe_stem(&name),
        extension
    );
    std::fs::create_dir_all(media_dir)
        .map_err(|error| crate::error::Error::Workspace(error.to_string()))?;
    let stored = media_dir.join(&stored_name);
    let already = std::fs::metadata(&stored).is_ok_and(|meta| meta.len() == metadata.len());
    if !already {
        std::fs::copy(&path, &stored)
            .map_err(|error| crate::error::Error::Workspace(error.to_string()))?;
    }
    let preview_url = if kind == "image" {
        crate::attachments::image_preview(&stored)
    } else {
        None
    };
    Ok(Some(ThreadMedia {
        kind: kind.into(),
        name,
        path: stored.display().to_string(),
        preview_url,
    }))
}

fn read_head(path: &Path, count: usize) -> Vec<u8> {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut buffer = vec![0u8; count];
    let read = file.read(&mut buffer).unwrap_or(0);
    buffer.truncate(read);
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cormux-media-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const PNG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];

    #[test]
    fn copies_a_screenshot_into_the_media_folder() {
        let root = root();
        let work = root.join("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("after.png"), PNG).unwrap();
        let media_dir = root.join("media");

        let by_absolute = resolve(&media_dir, work.join("after.png").to_str().unwrap(), None)
            .unwrap()
            .unwrap();
        assert_eq!(by_absolute.kind, "image");
        assert_eq!(by_absolute.name, "after.png");
        assert!(Path::new(&by_absolute.path).starts_with(&media_dir));
        assert!(
            by_absolute
                .preview_url
                .is_some_and(|url| url.starts_with("data:image/png"))
        );

        // Relative references resolve against the worktree, and reuse the same copy.
        let by_relative = resolve(&media_dir, "after.png", Some(&work))
            .unwrap()
            .unwrap();
        assert_eq!(by_relative.path, by_absolute.path);
        let by_file_url = resolve(
            &media_dir,
            &format!("file://{}", work.join("after.png").display()),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(by_file_url.path, by_absolute.path);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_files_that_are_not_what_they_claim() {
        let root = root();
        std::fs::write(root.join("fake.png"), b"just some text").unwrap();
        std::fs::write(root.join("notes.txt"), b"hi").unwrap();
        assert!(
            resolve(
                &root.join("media"),
                root.join("fake.png").to_str().unwrap(),
                None
            )
            .unwrap()
            .is_none()
        );
        assert!(
            resolve(
                &root.join("media"),
                root.join("notes.txt").to_str().unwrap(),
                None
            )
            .unwrap()
            .is_none()
        );
        assert!(
            resolve(&root.join("media"), "/no/such/shot.png", None)
                .unwrap()
                .is_none()
        );
        // A relative reference without a worktree can't resolve.
        assert!(
            resolve(&root.join("media"), "after.png", None)
                .unwrap()
                .is_none()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn accepts_a_recording_by_its_ftyp_box() {
        let root = root();
        let mut mp4 = b"\0\0\0\x18".to_vec();
        mp4.extend_from_slice(b"ftypisom");
        std::fs::write(root.join("flow.mp4"), &mp4).unwrap();
        let media = resolve(
            &root.join("media"),
            root.join("flow.mp4").to_str().unwrap(),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(media.kind, "video");
        assert!(media.preview_url.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
