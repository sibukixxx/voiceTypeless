use tokio::sync::mpsc;

use crate::domain::stt::{
    AudioSegment, PipelineEvent, PipelineState, SttContext, SttEngine, Transcript,
};

/// STT パイプラインラッパー。
///
/// SttEngine の呼び出しを状態遷移イベントで包み、
/// `PipelineEvent` を mpsc チャンネル経由で通知する。
pub struct SttPipeline {
    engine: Box<dyn SttEngine>,
    event_tx: mpsc::UnboundedSender<PipelineEvent>,
    state: PipelineState,
}

impl SttPipeline {
    pub fn new(
        engine: Box<dyn SttEngine>,
        event_tx: mpsc::UnboundedSender<PipelineEvent>,
    ) -> Self {
        Self {
            engine,
            event_tx,
            state: PipelineState::Idle,
        }
    }

    /// 現在の状態を返す。
    pub fn state(&self) -> PipelineState {
        self.state
    }

    /// エンジン名を返す。
    pub fn engine_name(&self) -> &str {
        self.engine.name()
    }

    /// 状態を遷移してイベントを発行する。
    fn transition(&mut self, new_state: PipelineState) {
        if self.state != new_state {
            self.state = new_state;
            let _ = self.event_tx.send(PipelineEvent::StateChanged(new_state));
        }
    }

    /// VU メータレベルを通知する。
    pub fn notify_audio_level(&self, rms: f32) {
        let _ = self.event_tx.send(PipelineEvent::AudioLevel(rms));
    }

    /// Listening 状態に遷移する（マイク開始時）。
    pub fn start_listening(&mut self) {
        self.transition(PipelineState::Listening);
    }

    /// Capturing 状態に遷移する（VAD speech 検出時）。
    pub fn start_capturing(&mut self) {
        self.transition(PipelineState::Capturing);
    }

    /// Idle 状態に遷移する（マイク停止時）。
    pub fn stop(&mut self) {
        self.transition(PipelineState::Idle);
    }

    /// セグメントを文字起こしする。
    ///
    /// Processing 状態に遷移 → engine.transcribe() → 結果イベント発行 → Listening に復帰。
    pub async fn transcribe(
        &mut self,
        audio: &AudioSegment,
        ctx: &SttContext,
    ) -> Option<Transcript> {
        self.transition(PipelineState::Processing);

        match self.engine.transcribe(audio, ctx).await {
            Ok(transcript) => {
                let _ = self
                    .event_tx
                    .send(PipelineEvent::FinalTranscript(transcript.clone()));
                self.transition(PipelineState::Listening);
                Some(transcript)
            }
            Err(e) => {
                let recoverable = e.recoverable;
                let _ = self.event_tx.send(PipelineEvent::Error(e));
                if recoverable {
                    self.transition(PipelineState::Listening);
                } else {
                    self.transition(PipelineState::Idle);
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::stt::SttError;

    // ── テスト用モックエンジン ──────────────────────────────

    struct MockEngine {
        result: Result<Transcript, SttError>,
    }

    #[async_trait::async_trait]
    impl SttEngine for MockEngine {
        async fn transcribe(
            &self,
            _audio: &AudioSegment,
            _ctx: &SttContext,
        ) -> Result<Transcript, SttError> {
            match &self.result {
                Ok(t) => Ok(t.clone()),
                Err(e) => Err(SttError {
                    kind: e.kind,
                    detail: e.detail.clone(),
                    recoverable: e.recoverable,
                }),
            }
        }

        fn supports_partial(&self) -> bool {
            false
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    fn make_transcript(text: &str) -> Transcript {
        Transcript {
            text: text.into(),
            confidence: None,
            is_partial: false,
            tokens: None,
            timings: None,
        }
    }

    #[tokio::test]
    async fn pipeline_state_transitions_on_success() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let engine = MockEngine {
            result: Ok(make_transcript("hello")),
        };
        let mut pipeline = SttPipeline::new(Box::new(engine), tx);

        assert_eq!(pipeline.state(), PipelineState::Idle);

        pipeline.start_listening();
        assert_eq!(pipeline.state(), PipelineState::Listening);

        pipeline.start_capturing();
        assert_eq!(pipeline.state(), PipelineState::Capturing);

        let audio = AudioSegment::new(vec![0.0; 16000], 16000, chrono::Utc::now());
        let ctx = SttContext::default();
        let result = pipeline.transcribe(&audio, &ctx).await;

        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "hello");
        assert_eq!(pipeline.state(), PipelineState::Listening);

        // イベント確認: Listening, Capturing, Processing, FinalTranscript, Listening
        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }
        assert!(events.len() >= 4);
        assert!(matches!(events[0], PipelineEvent::StateChanged(PipelineState::Listening)));
        assert!(matches!(events[1], PipelineEvent::StateChanged(PipelineState::Capturing)));
        assert!(matches!(events[2], PipelineEvent::StateChanged(PipelineState::Processing)));
        assert!(matches!(events[3], PipelineEvent::FinalTranscript(_)));
        assert!(matches!(events[4], PipelineEvent::StateChanged(PipelineState::Listening)));
    }

    #[tokio::test]
    async fn pipeline_recoverable_error_returns_to_listening() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let engine = MockEngine {
            result: Err(SttError::no_speech()),
        };
        let mut pipeline = SttPipeline::new(Box::new(engine), tx);

        pipeline.start_listening();
        let audio = AudioSegment::new(vec![0.0; 16000], 16000, chrono::Utc::now());
        let result = pipeline.transcribe(&audio, &SttContext::default()).await;

        assert!(result.is_none());
        assert_eq!(pipeline.state(), PipelineState::Listening);

        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }
        // Error event should be present, followed by return to Listening
        let has_error = events.iter().any(|e| matches!(e, PipelineEvent::Error(_)));
        assert!(has_error);
    }

    #[tokio::test]
    async fn pipeline_fatal_error_goes_idle() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let engine = MockEngine {
            result: Err(SttError::engine_not_available("gone")),
        };
        let mut pipeline = SttPipeline::new(Box::new(engine), tx);

        pipeline.start_listening();
        let audio = AudioSegment::new(vec![0.0; 16000], 16000, chrono::Utc::now());
        let result = pipeline.transcribe(&audio, &SttContext::default()).await;

        assert!(result.is_none());
        assert_eq!(pipeline.state(), PipelineState::Idle);

        let mut events = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }
        let has_error = events.iter().any(|e| matches!(e, PipelineEvent::Error(_)));
        assert!(has_error);
        // Last state change should be Idle
        let last_state = events.iter().rev().find_map(|e| match e {
            PipelineEvent::StateChanged(s) => Some(*s),
            _ => None,
        });
        assert_eq!(last_state, Some(PipelineState::Idle));
    }

    #[tokio::test]
    async fn pipeline_audio_level_event() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let engine = MockEngine {
            result: Ok(make_transcript("test")),
        };
        let pipeline = SttPipeline::new(Box::new(engine), tx);

        pipeline.notify_audio_level(0.42);

        let ev = rx.try_recv().unwrap();
        match ev {
            PipelineEvent::AudioLevel(level) => {
                assert!((level - 0.42).abs() < f32::EPSILON);
            }
            _ => panic!("Expected AudioLevel event"),
        }
    }

    #[test]
    fn pipeline_engine_name() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let engine = MockEngine {
            result: Ok(make_transcript("test")),
        };
        let pipeline = SttPipeline::new(Box::new(engine), tx);
        assert_eq!(pipeline.engine_name(), "mock");
    }
}
