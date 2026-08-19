use std::{
    sync::{
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{
    commentary_runtime::{run_commentary_pipeline, CommentaryRuntimeConfig},
    tts::{LocalTtsEngine, TtsPlayback},
};

mod config;
mod connection;
mod i18n;
mod theme;
mod ui;

pub use config::{
    apply_start, apply_started, apply_stop, apply_stop_requested, env_llm_api_key, env_llm_model,
    friendly_llm_startup_error, style_tts_emotion, validate_custom_style_prompt, validate_volume,
    wrap_style_instruction, AiConnectionHint, CommentaryStyle, ConnectionProvider, GameType,
    CommentaryLanguage, LauncherConfig, LauncherStatus, ObsConnectionHint, UiLanguage, UiTheme,
    DEFAULT_CUSTOM_STYLE_PROMPT, MAX_CUSTOM_STYLE_PROMPT_CHARS,
    LLM_ENV_HINT,
    OPENROUTER_BASE_URL, SYSTEM_DEFAULT_VOICE, TEST_VOICE_TEXT,
};
pub use connection::{
    check_startup_requirements, play_test_voice, play_test_voice_text, resolve_llm_config,
    test_llm_connection,
};

pub const STOP_JOIN_TIMEOUT: Duration = Duration::from_secs(8);
pub const CLOSE_JOIN_TIMEOUT: Duration = Duration::from_secs(8);
const PIPELINE_ALREADY_RUNNING: &str = "commentary runtime already running";
const PIPELINE_DID_NOT_EXIT: &str =
    "Pipeline did not fully exit before timeout. Click Stop again before starting.";

static ACTIVE_COMMENTARY_RUNTIMES: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static LIFECYCLE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn run() {
    ui::run_launcher();
}

pub fn active_commentary_runtime_count() -> usize {
    ACTIVE_COMMENTARY_RUNTIMES.load(Ordering::SeqCst)
}

struct RuntimeGuard;

impl RuntimeGuard {
    fn acquire() -> Result<Self, String> {
        ACTIVE_COMMENTARY_RUNTIMES
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| Self)
            .map_err(|_| PIPELINE_ALREADY_RUNNING.to_string())
    }
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        ACTIVE_COMMENTARY_RUNTIMES.store(0, Ordering::SeqCst);
    }
}

pub struct PipelineSession {
    stop: Arc<AtomicBool>,
    tts: Option<TtsPlayback<LocalTtsEngine>>,
    join: Option<JoinHandle<()>>,
    obs_hint: Arc<AtomicU8>,
    ai_hint: Arc<AtomicU8>,
}

impl PipelineSession {
    pub fn start(config: LauncherConfig, session_api_key: Option<String>) -> Result<Self, String> {
        let llm = check_startup_requirements(&config, session_api_key.as_deref())?;
        config.save_to_disk()?;
        let style_instruction = config.style_instruction()?;

        let tts_config = config.to_tts_config();
        let tts = TtsPlayback::with_config(
            LocalTtsEngine::with_config(tts_config.clone()),
            tts_config.clone(),
        );
        let stop = Arc::new(AtomicBool::new(false));
        let obs_hint = Arc::new(AtomicU8::new(ObsConnectionHint::Unknown.as_u8()));
        let ai_hint = Arc::new(AtomicU8::new(AiConnectionHint::Connected.as_u8()));
        let runtime_config = CommentaryRuntimeConfig {
            tts: tts_config,
            style: config.style,
            style_instruction: Some(style_instruction),
            output_language: config.commentary_language.prompt_language(),
            llm: Some(llm),
            obs_hint: obs_hint.clone(),
            ai_hint: ai_hint.clone(),
        };
        let session_tts = tts.clone();

        Self::spawn(stop, Some(tts), obs_hint, ai_hint, move |session_stop| {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("[Launcher Error] {error}");
                    return;
                }
            };
            if let Err(error) =
                runtime.block_on(run_commentary_pipeline(runtime_config, session_tts, session_stop))
            {
                eprintln!("[Launcher Error] {error}");
            }
        })
    }

    fn spawn<F>(
        stop: Arc<AtomicBool>,
        tts: Option<TtsPlayback<LocalTtsEngine>>,
        obs_hint: Arc<AtomicU8>,
        ai_hint: Arc<AtomicU8>,
        runner: F,
    ) -> Result<Self, String>
    where
        F: FnOnce(Arc<AtomicBool>) + Send + 'static,
    {
        let guard = RuntimeGuard::acquire()?;
        let thread_stop = stop.clone();
        let join = thread::Builder::new()
            .name("commentary-pipeline".into())
            .spawn(move || {
                let _guard = guard;
                runner(thread_stop);
            })
            .map_err(|error| error.to_string())?;

        Ok(Self {
            stop,
            tts,
            join: Some(join),
            obs_hint,
            ai_hint,
        })
    }

    pub fn obs_hint(&self) -> ObsConnectionHint {
        ObsConnectionHint::from_u8(self.obs_hint.load(Ordering::SeqCst))
    }

    pub fn ai_hint(&self) -> AiConnectionHint {
        AiConnectionHint::from_u8(self.ai_hint.load(Ordering::SeqCst))
    }

    pub fn has_exited(&self) -> bool {
        self.join.as_ref().map(|handle| handle.is_finished()).unwrap_or(true)
    }

    fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(tts) = &self.tts {
            tts.stop();
        }
    }

    pub fn stop(&mut self) -> Result<(), String> {
        self.stop_with_timeout(STOP_JOIN_TIMEOUT)
    }

    pub fn stop_on_close(&mut self) -> Result<(), String> {
        self.stop_with_timeout(CLOSE_JOIN_TIMEOUT)
    }

    pub fn stop_with_timeout(&mut self, timeout: Duration) -> Result<(), String> {
        self.request_stop();
        self.wait_for_exit(timeout)
    }

    fn wait_for_exit(&mut self, timeout: Duration) -> Result<(), String> {
        let Some(handle) = self.join.as_mut() else {
            return Ok(());
        };
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if handle.is_finished() {
                return self.take_join();
            }
            thread::sleep(Duration::from_millis(20));
        }
        if handle.is_finished() {
            return self.take_join();
        }
        Err(PIPELINE_DID_NOT_EXIT.to_string())
    }

    fn take_join(&mut self) -> Result<(), String> {
        match self.join.take() {
            Some(handle) => handle
                .join()
                .map_err(|_| "commentary pipeline thread panicked".to_string()),
            None => Ok(()),
        }
    }
}

impl Drop for PipelineSession {
    fn drop(&mut self) {
        self.request_stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lifecycle_lock() -> std::sync::MutexGuard<'static, ()> {
        LIFECYCLE_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn dummy_session() -> PipelineSession {
        PipelineSession::spawn(
            Arc::new(AtomicBool::new(false)),
            None,
            Arc::new(AtomicU8::new(0)),
            Arc::new(AtomicU8::new(0)),
            |stop| {
                while !stop.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(10));
                }
            },
        )
        .expect("dummy pipeline should start")
    }

    #[test]
    fn start_reaches_single_running_runtime() {
        let _lock = lifecycle_lock();
        let mut session = dummy_session();
        assert_eq!(active_commentary_runtime_count(), 1);
        session.stop().expect("stop dummy pipeline");
        assert_eq!(active_commentary_runtime_count(), 0);
        assert!(session.has_exited());
    }

    #[test]
    fn start_while_running_is_rejected() {
        let _lock = lifecycle_lock();
        let mut session = dummy_session();
        let second = PipelineSession::spawn(
            Arc::new(AtomicBool::new(false)),
            None,
            Arc::new(AtomicU8::new(0)),
            Arc::new(AtomicU8::new(0)),
            |_| {},
        );
        match second {
            Ok(extra) => {
                extra.request_stop();
                panic!("second commentary runtime should be rejected");
            }
            Err(error) => assert_eq!(error, PIPELINE_ALREADY_RUNNING),
        }
        assert_eq!(active_commentary_runtime_count(), 1);
        session.stop().expect("stop dummy pipeline");
    }

    #[test]
    fn stop_waits_until_pipeline_exits() {
        let _lock = lifecycle_lock();
        let mut session = PipelineSession::spawn(
            Arc::new(AtomicBool::new(false)),
            None,
            Arc::new(AtomicU8::new(0)),
            Arc::new(AtomicU8::new(0)),
            |stop| {
                while !stop.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(10));
                }
                thread::sleep(Duration::from_millis(80));
            },
        )
        .expect("dummy pipeline should start");

        let started = Instant::now();
        session.stop().expect("stop should wait for exit");
        assert!(started.elapsed() >= Duration::from_millis(80));
        assert_eq!(active_commentary_runtime_count(), 0);
        assert!(session.has_exited());
    }

    #[test]
    fn stop_timeout_does_not_claim_exit() {
        let _lock = lifecycle_lock();
        let mut session = PipelineSession::spawn(
            Arc::new(AtomicBool::new(false)),
            None,
            Arc::new(AtomicU8::new(0)),
            Arc::new(AtomicU8::new(0)),
            |_| {
                thread::sleep(Duration::from_millis(120));
            },
        )
        .expect("dummy pipeline should start");

        let error = session
            .stop_with_timeout(Duration::from_millis(20))
            .expect_err("timeout should be reported");
        assert_eq!(error, PIPELINE_DID_NOT_EXIT);
        assert_eq!(active_commentary_runtime_count(), 1);
        assert!(!session.has_exited());

        session
            .wait_for_exit(Duration::from_millis(500))
            .expect("pipeline should exit after the delayed runner finishes");
        assert_eq!(active_commentary_runtime_count(), 0);
    }

    #[test]
    fn stop_then_start_has_only_one_runtime() {
        let _lock = lifecycle_lock();
        let mut first = dummy_session();
        assert_eq!(active_commentary_runtime_count(), 1);
        first.stop().expect("stop first pipeline");
        assert_eq!(active_commentary_runtime_count(), 0);

        let mut second = dummy_session();
        assert_eq!(active_commentary_runtime_count(), 1);
        second.stop().expect("stop second pipeline");
        assert_eq!(active_commentary_runtime_count(), 0);
    }
}
