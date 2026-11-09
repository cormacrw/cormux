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

/// What starts the line naming a message's attachments where the thread shows it. The browser
/// harness gets it as `ATTACHED_LINE_PREFIX`.
pub const ATTACHED_LINE_PREFIX: &str = "Attached: ";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptAttachment {
    pub name: String,
    /// A file on disk, from the file picker or a drop.
    pub path: Option<String>,
    /// The contents of a pasted file, which has no path.
    pub data_base64: Option<String>,
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
        if let Some(format) = crate::media::sniff_image(&path)
            && std::fs::metadata(&path)
                .is_ok_and(|meta| meta.len() <= crate::media::INLINE_IMAGE_LIMIT)
        {
            let bytes =
                std::fs::read(&path).map_err(|error| Error::Workspace(error.to_string()))?;
            images.push(PromptImage {
                media_type: format.mime.into(),
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
        (listing, format!("{ATTACHED_LINE_PREFIX}{names}"))
    } else {
        (
            format!("{text}\n\n{listing}"),
            format!("{text}\n\n{ATTACHED_LINE_PREFIX}{names}"),
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

/// A thumbnail for an image attached by path, for the composer: one the model is shown inline.
#[tauri::command]
#[specta::specta]
pub async fn image_preview(path: String) -> crate::error::Result<Option<String>> {
    Ok(tokio::task::spawn_blocking(move || crate::media::image_data_url(Path::new(&path))).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    #[test]
    fn previews_only_images() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let image = root.join("shot.png");
        std::fs::write(&image, PNG).unwrap();
        let notes = root.join("notes.txt");
        std::fs::write(&notes, b"hi").unwrap();
        assert_eq!(
            crate::media::image_data_url(&image).as_deref(),
            Some(format!("data:image/png;base64,{}", STANDARD.encode(PNG)).as_str())
        );
        assert_eq!(crate::media::image_data_url(&notes), None);
        let renamed = root.join("renamed.png");
        std::fs::write(&renamed, b"png").unwrap();
        assert_eq!(crate::media::image_data_url(&renamed), None);
        assert_eq!(crate::media::image_data_url(&root.join("gone.png")), None);
    }

    #[test]
    fn pasted_names_stay_one_safe_path_segment() {
        assert_eq!(safe_file_name("../../etc/passwd"), "-..-etc-passwd");
        assert_eq!(safe_file_name("Screen Shot 1.png"), "Screen Shot 1.png");
        assert_eq!(safe_file_name("..."), "attachment");
    }

    #[test]
    fn builds_a_prompt_with_images_and_file_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let notes = root.join("notes.txt");
        std::fs::write(&notes, "hello").unwrap();
        let attachments = vec![
            PromptAttachment {
                name: "shot.png".into(),
                path: None,
                data_base64: Some(STANDARD.encode(PNG)),
            },
            PromptAttachment {
                name: "notes.txt".into(),
                path: Some(notes.display().to_string()),
                data_base64: None,
            },
        ];
        let (prompt, shown) = build_prompt(root, "Fix the layout", &attachments).unwrap();
        assert_eq!(shown, "Fix the layout\n\nAttached: shot.png, notes.txt");
        assert_eq!(prompt.images.len(), 1);
        assert_eq!(prompt.images[0].media_type, "image/png");
        assert_eq!(prompt.images[0].data_base64, STANDARD.encode(PNG));
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
        assert!(build_prompt(root, "", &[missing]).is_err());
    }

    /// Bug: attachments went to the model as images by their extension alone, so JPEG data
    /// saved as `.png` was sent as a PNG and the engine failed the turn.
    #[test]
    fn an_image_whose_bytes_are_not_its_type_is_only_listed() {
        let dir = tempfile::tempdir().unwrap();
        let jpeg_as_png = PromptAttachment {
            name: "x.png".into(),
            path: None,
            data_base64: Some(STANDARD.encode([0xFF, 0xD8, 0xFF, 0xE0])),
        };
        let (prompt, _) = build_prompt(dir.path(), "Look", &[jpeg_as_png]).unwrap();
        assert!(prompt.images.is_empty());
        assert!(prompt.text.ends_with("/x.png"));
    }
}
