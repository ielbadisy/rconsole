#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptKind {
    User,
    Chat,
    R,
    Codex,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptEntry {
    pub label: String,
    pub kind: TranscriptKind,
    pub content: String,
}

#[derive(Debug, Default)]
pub struct Transcript {
    entries: Vec<TranscriptEntry>,
}

impl Transcript {
    pub fn push(&mut self, entry: TranscriptEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[TranscriptEntry] {
        &self.entries
    }
}

impl TranscriptEntry {
    pub fn new(label: impl Into<String>, kind: TranscriptKind, content: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            kind,
            content: content.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Transcript, TranscriptEntry, TranscriptKind};

    #[test]
    fn stores_entries_in_order() {
        let mut transcript = Transcript::default();
        transcript.push(TranscriptEntry::new(
            "[system]",
            TranscriptKind::System,
            "ready",
        ));
        transcript.push(TranscriptEntry::new(
            "[chat]",
            TranscriptKind::Chat,
            "hello",
        ));

        assert_eq!(transcript.entries().len(), 2);
        assert_eq!(transcript.entries()[0].content, "ready");
        assert_eq!(transcript.entries()[1].content, "hello");
    }
}
