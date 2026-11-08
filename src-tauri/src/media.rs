//! The image and video files a thread shows, by extension. `src/lib/media.ts` lists the same
//! extensions for the UI; a test keeps the two equal.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Image,
    Video,
}

impl MediaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaFormat {
    pub kind: MediaKind,
    /// The extension a copy is saved under: `jpeg` files become `jpg`.
    pub canonical: &'static str,
    pub mime: &'static str,
}

const FORMATS: &[(&str, MediaFormat)] = &[
    ("png", image("png", "image/png")),
    ("jpg", image("jpg", "image/jpeg")),
    ("jpeg", image("jpg", "image/jpeg")),
    ("gif", image("gif", "image/gif")),
    ("webp", image("webp", "image/webp")),
    ("mp4", video("mp4", "video/mp4")),
    ("webm", video("webm", "video/webm")),
    ("mov", video("mov", "video/quicktime")),
    ("m4v", video("m4v", "video/x-m4v")),
];

const fn image(canonical: &'static str, mime: &'static str) -> MediaFormat {
    MediaFormat {
        kind: MediaKind::Image,
        canonical,
        mime,
    }
}

const fn video(canonical: &'static str, mime: &'static str) -> MediaFormat {
    MediaFormat {
        kind: MediaKind::Video,
        canonical,
        mime,
    }
}

/// The format of a file name or path, by its extension in any case.
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

    /// The quoted items of `const <name> = [...]` in `src/lib/media.ts`.
    fn ui_list(name: &str) -> Vec<String> {
        let source = include_str!("../../src/lib/media.ts");
        let start = source
            .find(&format!("const {name} = ["))
            .unwrap_or_else(|| panic!("{name} is gone from media.ts"));
        let list = &source[start..];
        let list = &list[list.find('[').unwrap() + 1..list.find(']').unwrap()];
        list.split(',')
            .map(|item| item.trim().trim_matches('\'').to_string())
            .filter(|item| !item.is_empty())
            .collect()
    }

    #[test]
    fn the_ui_shows_the_same_files() {
        assert_eq!(ui_list("IMAGE_EXTENSIONS"), extensions(MediaKind::Image));
        assert_eq!(ui_list("VIDEO_EXTENSIONS"), extensions(MediaKind::Video));
    }

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

    #[test]
    fn the_agent_is_told_every_kind_of_file() {
        let told = instruction();
        assert!(told.contains("(png, jpg, jpeg, gif, webp; mp4, webm, mov, m4v)"));
    }
}
