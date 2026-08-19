use std::future::Future;

use super::TtsError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtsUtterance {
    pub text: String,
    pub rate: i32,
    pub volume: u16,
}

impl TtsUtterance {
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            rate: 0,
            volume: 80,
        }
    }
}

pub trait TtsEngine: Send + Sync {
    fn speak(&self, utterance: &TtsUtterance) -> impl Future<Output = Result<(), TtsError>> + Send;

    fn interrupt(&self) {}
}
