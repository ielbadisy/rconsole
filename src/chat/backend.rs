pub trait ChatBackend {
    fn ask(&self, prompt: &str) -> String;
}

#[derive(Debug, Default)]
pub struct PlaceholderBackend;

impl ChatBackend for PlaceholderBackend {
    fn ask(&self, prompt: &str) -> String {
        format!(
            "v1 local backend: I can help explain, plan, or suggest next commands.\nPrompt: {prompt}"
        )
    }
}
