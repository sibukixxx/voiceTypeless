use serde::Serialize;

/// ジョブ状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
    Canceled,
}

/// ジョブ種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Transcribe,
    Rewrite,
    Deliver,
}

/// ジョブメタデータ
#[derive(Debug, Clone, Serialize)]
pub struct JobInfo {
    pub job_id: String,
    pub session_id: String,
    pub segment_id: Option<String>,
    pub kind: JobKind,
    pub status: JobStatus,
    pub created_at: String,
    pub error: Option<String>,
}

impl JobInfo {
    pub fn new(
        job_id: String,
        session_id: String,
        segment_id: Option<String>,
        kind: JobKind,
        now: String,
    ) -> Self {
        Self {
            job_id,
            session_id,
            segment_id,
            kind,
            status: JobStatus::Queued,
            created_at: now,
            error: None,
        }
    }
}
