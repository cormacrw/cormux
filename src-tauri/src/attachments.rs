//! Images and files attached in the composer. Each one becomes a file the agent can open:
//! picked and dropped files keep their own path, pasted ones are saved under the app's data
//! folder (never the worktree, so they stay out of git). Images also go to the model directly.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::engines::prompt::{Prompt, PromptImage};
use crate::error::{Error, Result};

/// Images the model is shown directly; larger ones are only passed by path.
const IMAGE_LIMIT_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptAttachment {
    pub name: String,
    /// A file on disk, from the file picker or a drop.
    pub path: Option<String>,
    /// The contents of a pasted file, which has no path.
    pub data_base64: Option<String>,
}

/// The image type for a file name the model can be shown, if any.
pub fn image_media_type(name: &str) -> Option<&'static str> {
    let extension = Path::new(name).extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// A thumbnail for an image picked or dropped by path, as a data URL; None for anything that
/// isn't a readable image of a size the model is shown.
pub fn image_preview(path: &Path) -> Option<String> {
    let media_type = image_media_type(path.to_str()?)?;
    if std::fs::metadata(path).ok()?.len() > IMAGE_LIMIT_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(format!(
        "data:{media_type};base64,{}",
        STANDARD.encode(bytes)
    ))
}

/// Keeps a pasted file's name readable but safe as one path segment.
fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_string();
    if cleaned.is_empty() {
        "attachment".into()
    } else {
        cleaned
    }
}

fn file_for(dir: &Path, attachment: &PromptAttachment) -> Result<PathBuf> {
    if let Some(path) = attachment.path.as_deref().filter(|path| !path.is_empty()) {
        let path = PathBuf::from(path);
        if !path.is_file() {
            return Err(Error::Workspace(format!(
                "{} isn't a file Cormux can attach",
                attachment.name
            )));
        }
        return Ok(path);
    }
    let data = attachment
        .data_base64
        .as_deref()
        .ok_or_else(|| Error::Workspace(format!("{} has no contents", attachment.name)))?;
    let bytes = STANDARD
        .decode(data)
        .map_err(|error| Error::Workspace(format!("{}: {error}", attachment.name)))?;
    let folder = dir.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&folder).map_err(|error| Error::Workspace(error.to_string()))?;
    let path = folder.join(safe_file_name(&attachment.name));
    std::fs::write(&path, bytes).map_err(|error| Error::Workspace(error.to_string()))?;
    Ok(path)
}

/// The prompt for the engine, and the message the conversation shows. Every attachment is
/// listed by path so the agent can open it; images small enough are also shown to the model.
pub fn build_prompt(
    dir: &Path,
    text: &str,
    attachments: &[PromptAttachment],
) -> Result<(Prompt, String)> {
    if attachments.is_empty() {
        return Ok((text.to_string().into(), text.to_string()));
    }
    let mut paths = Vec::new();
    let mut images = Vec::new();
    for attachment in attachments {
        let path = file_for(dir, attachment)?;
        if let Some(media_type) = image_media_type(&attachment.name)
            && std::fs::metadata(&path).is_ok_and(|meta| meta.len() <= IMAGE_LIMIT_BYTES)
        {
            let bytes =
                std::fs::read(&path).map_err(|error| Error::Workspace(error.to_string()))?;
            images.push(PromptImage {
                media_type: media_type.into(),
                data_base64: STANDARD.encode(bytes),
            });
        }
        paths.push(format!("- {}", path.display()));
    }
    let listing = format!("Attached files:\n{}", paths.join("\n"));
    let names = attachments
        .iter()
        .map(|attachment| attachment.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let (engine_text, shown) = if text.is_empty() {
        (listing, format!("Attached: {names}"))
    } else {
        (
            format!("{text}\n\n{listing}"),
            format!("{text}\n\nAttached: {names}"),
        )
    };
    Ok((
        Prompt {
            text: engine_text,
            images,
        },
        shown,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_which_files_the_model_can_see() {
        assert_eq!(image_media_type("Shot.PNG"), Some("image/png"));
        assert_eq!(image_media_type("a.jpeg"), Some("image/jpeg"));
        assert_eq!(image_media_type("notes.pdf"), None);
        assert_eq!(image_media_type("README"), None);
    }

    #[test]
    fn previews_only_images() {
        let root = std::env::temp_dir().join(format!("cormux-preview-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("shot.png");
        std::fs::write(&image, b"png").unwrap();
        let notes = root.join("notes.txt");
        std::fs::write(&notes, b"hi").unwrap();
        assert_eq!(
            image_preview(&image).as_deref(),
            Some("data:image/png;base64,cG5n")
        );
        assert_eq!(image_preview(&notes), None);
        assert_eq!(image_preview(&root.join("gone.png")), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pasted_names_stay_one_safe_path_segment() {
        assert_eq!(safe_file_name("../../etc/passwd"), "-..-etc-passwd");
        assert_eq!(safe_file_name("Screen Shot 1.png"), "Screen Shot 1.png");
        assert_eq!(safe_file_name("..."), "attachment");
    }

    #[test]
    fn builds_a_prompt_with_images_and_file_paths() {
        let root = std::env::temp_dir().join(format!("cormux-attach-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let notes = root.join("notes.txt");
        std::fs::write(&notes, "hello").unwrap();
        let attachments = vec![
            PromptAttachment {
                name: "shot.png".into(),
                path: None,
                data_base64: Some(STANDARD.encode(b"png-bytes")),
            },
            PromptAttachment {
                name: "notes.txt".into(),
                path: Some(notes.display().to_string()),
                data_base64: None,
            },
        ];
        let (prompt, shown) = build_prompt(&root, "Fix the layout", &attachments).unwrap();
        assert_eq!(shown, "Fix the layout\n\nAttached: shot.png, notes.txt");
        assert_eq!(prompt.images.len(), 1);
        assert_eq!(prompt.images[0].media_type, "image/png");
        assert_eq!(prompt.images[0].data_base64, STANDARD.encode(b"png-bytes"));
        assert!(
            prompt
                .text
                .starts_with("Fix the layout\n\nAttached files:\n- ")
        );
        assert!(prompt.text.contains("/shot.png\n"));
        assert!(prompt.text.ends_with(&format!("- {}", notes.display())));

        let missing = PromptAttachment {
            name: "gone.txt".into(),
            path: Some(root.join("gone.txt").display().to_string()),
            data_base64: None,
        };
        assert!(build_prompt(&root, "", &[missing]).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
