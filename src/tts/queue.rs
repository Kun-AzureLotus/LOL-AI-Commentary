use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex,
    },
};

use crate::narrative_engine::{Emotion, NarrativeMode, Priority};

use super::{TtsConfig, TtsConnectionHint, TtsEngine, TtsError, TtsUtterance};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsPlaybackClass {
    Normal,
    HighConfirmed,
}

impl TtsPlaybackClass {
    pub fn from_commentary(
        mode: NarrativeMode,
        priority: Priority,
        emotion: Emotion,
    ) -> Self {
        if mode == NarrativeMode::ConfirmedEvent
            && (priority == Priority::High || emotion == Emotion::Epic)
        {
            Self::HighConfirmed
        } else {
            Self::Normal
        }
    }

    fn can_preempt(self, other: Self) -> bool {
        self == Self::HighConfirmed && other == Self::Normal
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedCommentary {
    pub text: String,
    pub class: TtsPlaybackClass,
    pub rate: i32,
    pub volume: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueOutcome {
    IgnoredEmpty,
    IgnoredDuplicate,
    Queued,
    PreemptedPending,
    RejectedLowerPriority,
}

#[derive(Debug, Default)]
pub struct TtsQueue {
    pending: VecDeque<QueuedCommentary>,
    last_text: Option<String>,
}

impl TtsQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&mut self, item: QueuedCommentary) -> EnqueueOutcome {
        let text = item.text.trim();
        if text.is_empty() {
            return EnqueueOutcome::IgnoredEmpty;
        }

        if self.last_text.as_deref() == Some(text)
            || self.pending.iter().any(|queued| queued.text == text)
        {
            return EnqueueOutcome::IgnoredDuplicate;
        }

        let item = QueuedCommentary {
            text: text.to_string(),
            class: item.class,
            rate: item.rate,
            volume: item.volume,
        };

        if let Some(front) = self.pending.front() {
            if item.class.can_preempt(front.class) {
                self.pending.clear();
                self.pending.push_back(item);
                return EnqueueOutcome::PreemptedPending;
            }
            if front.class.can_preempt(item.class) || front.class == TtsPlaybackClass::HighConfirmed
            {
                return EnqueueOutcome::RejectedLowerPriority;
            }
            self.pending.clear();
        }

        self.pending.push_back(item);
        EnqueueOutcome::Queued
    }

    pub fn take_next(&mut self) -> Option<QueuedCommentary> {
        self.pending.pop_front()
    }

    pub fn mark_played(&mut self, text: &str) {
        self.last_text = Some(text.trim().to_string());
    }

    pub fn pending_text(&self) -> Option<&str> {
        self.pending.front().map(|item| item.text.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

fn queued_item(
    text: &str,
    class: TtsPlaybackClass,
    rate: i32,
    volume: u16,
) -> QueuedCommentary {
    QueuedCommentary {
        text: text.to_string(),
        class,
        rate,
        volume,
    }
}

struct PlaybackState<E> {
    engine: E,
    queue: TtsQueue,
    config: TtsConfig,
    playing: Option<TtsPlaybackClass>,
    draining: bool,
    tts_status: Option<Arc<AtomicU8>>,
}

#[derive(Clone)]
pub struct TtsPlayback<E> {
    state: Arc<Mutex<PlaybackState<E>>>,
}

impl<E> TtsPlayback<E>
where
    E: TtsEngine + Clone + 'static,
{
    pub fn new(engine: E) -> Self {
        Self::with_config(engine, TtsConfig::default())
    }

    pub fn with_config(engine: E, config: TtsConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(PlaybackState {
                engine,
                queue: TtsQueue::new(),
                config,
                playing: None,
                draining: false,
                tts_status: None,
            })),
        }
    }

    pub fn with_status_hint(self, hint: Arc<AtomicU8>) -> Self {
        self.state.lock().expect("tts playback mutex").tts_status = Some(hint);
        self
    }

    pub fn enqueue(
        &self,
        text: &str,
        class: TtsPlaybackClass,
        emotion: Emotion,
    ) -> EnqueueOutcome {
        let mut state = self.state.lock().expect("tts playback mutex");
        let rate = state.config.rate_for_emotion(emotion);
        let volume = state.config.volume;
        let outcome = state.queue.enqueue(queued_item(
            text,
            class,
            rate,
            volume,
        ));

        let should_interrupt = matches!(
            outcome,
            EnqueueOutcome::Queued | EnqueueOutcome::PreemptedPending
        ) && class == TtsPlaybackClass::HighConfirmed
            && state.playing == Some(TtsPlaybackClass::Normal);

        if should_interrupt {
            state.engine.interrupt();
        }

        outcome
    }

    pub fn stop(&self) {
        let mut state = self.state.lock().expect("tts playback mutex");
        while state.queue.take_next().is_some() {}
        state.engine.interrupt();
        state.playing = None;
    }

    pub fn start_drain_if_needed(&self) {
        let mut state = self.state.lock().expect("tts playback mutex");
        if state.draining || state.queue.is_empty() {
            return;
        }
        state.draining = true;
        drop(state);

        let playback = self.clone();
        tokio::spawn(async move {
            let _ = playback.drain().await;
        });
    }

    pub async fn drain(&self) -> Result<(), TtsError> {
        let mut last_error = None;

        loop {
            let (engine, item) = {
                let mut state = self.state.lock().expect("tts playback mutex");
                match state.queue.take_next() {
                    Some(item) => {
                        state.playing = Some(item.class);
                        (state.engine.clone(), item)
                    }
                    None => {
                        if state.queue.is_empty() {
                            state.draining = false;
                            state.playing = None;
                            break;
                        }
                        continue;
                    }
                }
            };

            let utterance = TtsUtterance {
                text: item.text.clone(),
                rate: item.rate,
                volume: item.volume,
            };
            let result = engine.speak(&utterance).await;
            let mut state = self.state.lock().expect("tts playback mutex");
            state.playing = None;
            report_tts_result(&mut state, &result);
            if result.is_ok() {
                state.queue.mark_played(&item.text);
            } else if let Err(error) = result {
                last_error = Some(error);
            }
        }

        last_error.map_or(Ok(()), Err)
    }
}

fn report_tts_result<E>(state: &mut PlaybackState<E>, result: &Result<(), TtsError>) {
    let Some(hint) = &state.tts_status else {
        return;
    };
    let next = match result {
        Ok(()) => TtsConnectionHint::Connected,
        Err(error) => match TtsConnectionHint::from_error(error) {
            Some(status) => status,
            None => {
                eprintln!("[TTS Error] {error}");
                return;
            }
        },
    };
    let code = next.as_u8();
    let previous = hint.swap(code, Ordering::SeqCst);
    if result.is_err() && previous != code {
        if let Err(error) = result {
            eprintln!("[TTS Error] {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Clone, Default)]
    struct RecordingEngine {
        spoken: Arc<Mutex<Vec<TtsUtterance>>>,
        fail: bool,
    }

    impl TtsEngine for RecordingEngine {
        async fn speak(&self, utterance: &TtsUtterance) -> Result<(), TtsError> {
            if utterance.text.trim().is_empty() {
                return Ok(());
            }
            if self.fail {
                return Err(TtsError::PlaybackFailed);
            }
            self.spoken
                .lock()
                .expect("spoken")
                .push(utterance.clone());
            Ok(())
        }
    }

    #[test]
    fn empty_text_is_not_queued() {
        let mut queue = TtsQueue::new();

        let outcome = queue.enqueue(queued_item("   ", TtsPlaybackClass::Normal, 0, 80));

        assert_eq!(outcome, EnqueueOutcome::IgnoredEmpty);
        assert!(queue.is_empty());
    }

    #[test]
    fn normal_text_is_queued() {
        let mut queue = TtsQueue::new();

        let outcome = queue.enqueue(queued_item(
            "蓝方开始集中。",
            TtsPlaybackClass::Normal,
            0,
            80,
        ));

        assert_eq!(outcome, EnqueueOutcome::Queued);
        assert_eq!(queue.pending_text(), Some("蓝方开始集中。"));
    }

    #[test]
    fn duplicate_text_is_not_queued() {
        let mut queue = TtsQueue::new();
        queue.enqueue(queued_item("蓝方开始集中。", TtsPlaybackClass::Normal, 0, 80));

        let outcome = queue.enqueue(queued_item(
            "蓝方开始集中。",
            TtsPlaybackClass::Normal,
            0,
            80,
        ));

        assert_eq!(outcome, EnqueueOutcome::IgnoredDuplicate);
    }

    #[test]
    fn high_priority_preempts_low_priority_pending() {
        let mut queue = TtsQueue::new();
        queue.enqueue(queued_item(
            "可能成为焦点。",
            TtsPlaybackClass::Normal,
            0,
            80,
        ));

        let outcome = queue.enqueue(queued_item(
            "拿下大龙！",
            TtsPlaybackClass::HighConfirmed,
            3,
            80,
        ));

        assert_eq!(outcome, EnqueueOutcome::PreemptedPending);
        assert_eq!(queue.pending_text(), Some("拿下大龙！"));
    }

    #[test]
    fn visual_warning_cannot_preempt_high_confirmed() {
        let mut queue = TtsQueue::new();
        queue.enqueue(queued_item(
            "拿下大龙！",
            TtsPlaybackClass::HighConfirmed,
            3,
            80,
        ));

        let outcome = queue.enqueue(queued_item(
            "开始集中。",
            TtsPlaybackClass::Normal,
            -2,
            80,
        ));

        assert_eq!(outcome, EnqueueOutcome::RejectedLowerPriority);
        assert_eq!(queue.pending_text(), Some("拿下大龙！"));
    }

    #[tokio::test]
    async fn normal_text_is_played() {
        let engine = RecordingEngine::default();
        let playback = TtsPlayback::new(engine.clone());

        assert_eq!(
            playback.enqueue(
                "这是一句解说。",
                TtsPlaybackClass::Normal,
                Emotion::Excited
            ),
            EnqueueOutcome::Queued
        );
        playback.drain().await.expect("playback should succeed");

        let spoken = engine.spoken.lock().expect("spoken").clone();
        assert_eq!(spoken[0].text, "这是一句解说。");
        assert_eq!(spoken[0].rate, TtsConfig::default().excited_rate);
    }

    #[tokio::test]
    async fn calm_excited_epic_use_different_rates() {
        let engine = RecordingEngine::default();
        let playback = TtsPlayback::new(engine.clone());
        let config = TtsConfig::default();

        playback.enqueue("平静。", TtsPlaybackClass::Normal, Emotion::Calm);
        playback.drain().await.unwrap();
        playback.enqueue("兴奋。", TtsPlaybackClass::Normal, Emotion::Excited);
        playback.drain().await.unwrap();
        playback.enqueue("史诗。", TtsPlaybackClass::HighConfirmed, Emotion::Epic);
        playback.drain().await.unwrap();

        let spoken = engine.spoken.lock().expect("spoken").clone();
        assert_eq!(spoken[0].rate, config.calm_rate);
        assert_eq!(spoken[1].rate, config.excited_rate);
        assert_eq!(spoken[2].rate, config.epic_rate);
        assert_ne!(spoken[0].rate, spoken[1].rate);
        assert_ne!(spoken[1].rate, spoken[2].rate);
    }

    #[tokio::test]
    async fn tts_failure_does_not_panic() {
        let engine = RecordingEngine {
            spoken: Arc::new(Mutex::new(Vec::new())),
            fail: true,
        };
        let playback = TtsPlayback::new(engine);

        playback.enqueue(
            "这是一句解说。",
            TtsPlaybackClass::Normal,
            Emotion::Calm,
        );
        let result = playback.drain().await;

        assert!(matches!(result, Err(TtsError::PlaybackFailed)));
    }

    #[tokio::test]
    async fn empty_text_is_not_played() {
        let engine = RecordingEngine::default();
        let playback = TtsPlayback::new(engine.clone());

        playback.enqueue("   ", TtsPlaybackClass::Normal, Emotion::Calm);
        playback.drain().await.expect("drain empty queue");

        assert!(engine.spoken.lock().expect("spoken").is_empty());
    }

    #[derive(Clone)]
    struct SequenceEngine {
        results: Arc<Mutex<Vec<Result<(), TtsError>>>>,
        spoken: Arc<Mutex<Vec<String>>>,
    }

    impl TtsEngine for SequenceEngine {
        async fn speak(&self, utterance: &TtsUtterance) -> Result<(), TtsError> {
            let next = self
                .results
                .lock()
                .expect("results")
                .remove(0);
            if next.is_ok() {
                self.spoken
                    .lock()
                    .expect("spoken")
                    .push(utterance.text.clone());
            }
            next
        }
    }

    #[tokio::test]
    async fn elevenlabs_failure_skips_utterance_and_continues() {
        let engine = SequenceEngine {
            results: Arc::new(Mutex::new(vec![
                Err(TtsError::QuotaExceeded),
                Ok(()),
                Err(TtsError::Unauthorized),
                Ok(()),
            ])),
            spoken: Arc::new(Mutex::new(Vec::new())),
        };
        let hint = Arc::new(AtomicU8::new(TtsConnectionHint::Unknown.as_u8()));
        let playback = TtsPlayback::new(engine.clone()).with_status_hint(hint.clone());

        playback.enqueue("one", TtsPlaybackClass::Normal, Emotion::Calm);
        let first = playback.drain().await;
        assert!(matches!(first, Err(TtsError::QuotaExceeded)));
        assert_eq!(
            TtsConnectionHint::from_u8(hint.load(Ordering::SeqCst)),
            TtsConnectionHint::QuotaExceeded
        );

        playback.enqueue("two", TtsPlaybackClass::Normal, Emotion::Calm);
        playback.drain().await.expect("successful utterance should play");
        assert_eq!(
            TtsConnectionHint::from_u8(hint.load(Ordering::SeqCst)),
            TtsConnectionHint::Connected
        );

        playback.enqueue("three", TtsPlaybackClass::Normal, Emotion::Calm);
        let second = playback.drain().await;
        assert!(matches!(second, Err(TtsError::Unauthorized)));

        playback.enqueue("four", TtsPlaybackClass::Normal, Emotion::Calm);
        playback.drain().await.expect("pipeline should continue after TTS error");
        assert_eq!(engine.spoken.lock().expect("spoken").as_slice(), ["two", "four"]);
        assert_eq!(
            TtsConnectionHint::from_u8(hint.load(Ordering::SeqCst)),
            TtsConnectionHint::Connected
        );
    }

    #[tokio::test]
    async fn consecutive_quota_errors_keep_a_single_status() {
        let engine = SequenceEngine {
            results: Arc::new(Mutex::new(vec![
                Err(TtsError::QuotaExceeded),
                Err(TtsError::QuotaExceeded),
            ])),
            spoken: Arc::new(Mutex::new(Vec::new())),
        };
        let hint = Arc::new(AtomicU8::new(TtsConnectionHint::Unknown.as_u8()));
        let playback = TtsPlayback::new(engine).with_status_hint(hint.clone());

        playback.enqueue("one", TtsPlaybackClass::Normal, Emotion::Calm);
        let first = playback.drain().await;
        assert!(matches!(first, Err(TtsError::QuotaExceeded)));
        playback.enqueue("two", TtsPlaybackClass::Normal, Emotion::Calm);
        let result = playback.drain().await;

        assert!(matches!(result, Err(TtsError::QuotaExceeded)));
        assert_eq!(
            TtsConnectionHint::from_u8(hint.load(Ordering::SeqCst)),
            TtsConnectionHint::QuotaExceeded
        );
        assert_eq!(
            TtsConnectionHint::QuotaExceeded.status_line(),
            Some("ElevenLabs quota exceeded")
        );
    }

    #[tokio::test]
    async fn mocked_elevenlabs_http_failures_skip_and_continue() {
        async fn skip_then_play(error: TtsError) {
            let engine = SequenceEngine {
                results: Arc::new(Mutex::new(vec![Err(error), Ok(())])),
                spoken: Arc::new(Mutex::new(Vec::new())),
            };
            let playback = TtsPlayback::new(engine.clone());
            playback.enqueue("fail", TtsPlaybackClass::Normal, Emotion::Calm);
            let failed = playback.drain().await;
            assert!(failed.is_err());
            playback.enqueue("ok", TtsPlaybackClass::Normal, Emotion::Calm);
            playback.drain().await.expect("later commentary should still play");
            assert_eq!(engine.spoken.lock().expect("spoken").as_slice(), ["ok"]);
        }

        skip_then_play(TtsError::QuotaExceeded).await;
        skip_then_play(TtsError::Unauthorized).await;
        skip_then_play(TtsError::RateLimited).await;
        skip_then_play(TtsError::Network).await;
        skip_then_play(TtsError::Http { status: 403 }).await;
    }
}
