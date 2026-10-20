/// What a user turn sends an engine: the message, and any images to show the model with it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prompt {
    pub text: String,
    pub images: Vec<PromptImage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptImage {
    /// `image/png`, `image/jpeg`, `image/gif` or `image/webp`.
    pub media_type: String,
    pub data_base64: String,
}

impl From<String> for Prompt {
    fn from(text: String) -> Self {
        Self {
            text,
            images: Vec::new(),
        }
    }
}

impl From<&str> for Prompt {
    fn from(text: &str) -> Self {
        text.to_string().into()
    }
}

impl Prompt {
    /// Messages queued while a thread was paused, sent as one turn.
    pub fn merge(prompts: Vec<Prompt>) -> Prompt {
        let mut texts = Vec::new();
        let mut images = Vec::new();
        for prompt in prompts {
            texts.push(prompt.text);
            images.extend(prompt.images);
        }
        Prompt {
            text: texts.join("\n\n"),
            images,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merging_queued_prompts_keeps_every_image() {
        let image = PromptImage {
            media_type: "image/png".into(),
            data_base64: "aGk=".into(),
        };
        let merged = Prompt::merge(vec![
            Prompt {
                text: "first".into(),
                images: vec![image.clone()],
            },
            "second".into(),
        ]);
        assert_eq!(merged.text, "first\n\nsecond");
        assert_eq!(merged.images, vec![image]);
    }
}
