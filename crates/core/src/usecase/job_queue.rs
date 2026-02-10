use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinHandle;

use crate::domain::job::{JobInfo, JobKind, JobStatus};

/// ジョブキュー: Tokioタスクの発行・追跡・キャンセル
pub struct JobQueue {
    jobs: Arc<Mutex<HashMap<String, JobEntry>>>,
}

struct JobEntry {
    info: JobInfo,
    cancel_tx: Option<oneshot::Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// ジョブを登録し、キャンセルトークンのReceiverを返す
    pub async fn enqueue(
        &self,
        session_id: String,
        segment_id: Option<String>,
        kind: JobKind,
    ) -> (String, oneshot::Receiver<()>) {
        let job_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let info = JobInfo::new(job_id.clone(), session_id, segment_id, kind, now);
        let (cancel_tx, cancel_rx) = oneshot::channel();

        let entry = JobEntry {
            info,
            cancel_tx: Some(cancel_tx),
            handle: None,
        };

        let mut jobs = self.jobs.lock().await;
        jobs.insert(job_id.clone(), entry);

        (job_id, cancel_rx)
    }

    /// ジョブのJoinHandleを設定（spawn後に呼ぶ）
    pub async fn set_handle(&self, job_id: &str, handle: JoinHandle<()>) {
        let mut jobs = self.jobs.lock().await;
        if let Some(entry) = jobs.get_mut(job_id) {
            entry.handle = Some(handle);
        }
    }

    /// ジョブをRunning状態に変更
    pub async fn mark_running(&self, job_id: &str) {
        let mut jobs = self.jobs.lock().await;
        if let Some(entry) = jobs.get_mut(job_id) {
            entry.info.status = JobStatus::Running;
        }
    }

    /// ジョブをDone状態に変更
    pub async fn mark_done(&self, job_id: &str) {
        let mut jobs = self.jobs.lock().await;
        if let Some(entry) = jobs.get_mut(job_id) {
            entry.info.status = JobStatus::Done;
            entry.cancel_tx = None;
            entry.handle = None;
        }
    }

    /// ジョブをFailed状態に変更
    pub async fn mark_failed(&self, job_id: &str, error: String) {
        let mut jobs = self.jobs.lock().await;
        if let Some(entry) = jobs.get_mut(job_id) {
            entry.info.status = JobStatus::Failed;
            entry.info.error = Some(error);
            entry.cancel_tx = None;
            entry.handle = None;
        }
    }

    /// ジョブをキャンセル
    pub async fn cancel(&self, job_id: &str) -> bool {
        let mut jobs = self.jobs.lock().await;
        if let Some(entry) = jobs.get_mut(job_id) {
            if entry.info.status == JobStatus::Queued || entry.info.status == JobStatus::Running {
                entry.info.status = JobStatus::Canceled;
                // キャンセルシグナルを送信
                if let Some(tx) = entry.cancel_tx.take() {
                    let _ = tx.send(());
                }
                // タスクをabort
                if let Some(handle) = entry.handle.take() {
                    handle.abort();
                }
                return true;
            }
        }
        false
    }

    /// セッション内の全ジョブをキャンセル
    pub async fn cancel_session(&self, session_id: &str) -> Vec<String> {
        let mut jobs = self.jobs.lock().await;
        let mut canceled = vec![];

        for (job_id, entry) in jobs.iter_mut() {
            if entry.info.session_id == session_id
                && (entry.info.status == JobStatus::Queued
                    || entry.info.status == JobStatus::Running)
            {
                entry.info.status = JobStatus::Canceled;
                if let Some(tx) = entry.cancel_tx.take() {
                    let _ = tx.send(());
                }
                if let Some(handle) = entry.handle.take() {
                    handle.abort();
                }
                canceled.push(job_id.clone());
            }
        }

        canceled
    }

    /// ジョブ情報を取得
    pub async fn get_job(&self, job_id: &str) -> Option<JobInfo> {
        let jobs = self.jobs.lock().await;
        jobs.get(job_id).map(|e| e.info.clone())
    }

    /// 完了済みジョブを削除（メモリ解放）
    pub async fn cleanup_completed(&self) {
        let mut jobs = self.jobs.lock().await;
        jobs.retain(|_, entry| {
            !matches!(
                entry.info.status,
                JobStatus::Done | JobStatus::Failed | JobStatus::Canceled
            )
        });
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enqueue_and_get() {
        let queue = JobQueue::new();
        let (job_id, _cancel_rx) = queue
            .enqueue("s1".to_string(), Some("seg1".to_string()), JobKind::Transcribe)
            .await;

        let info = queue.get_job(&job_id).await.unwrap();
        assert_eq!(info.status, JobStatus::Queued);
        assert_eq!(info.kind, JobKind::Transcribe);
        assert_eq!(info.session_id, "s1");
    }

    #[tokio::test]
    async fn test_lifecycle() {
        let queue = JobQueue::new();
        let (job_id, _cancel_rx) = queue
            .enqueue("s1".to_string(), None, JobKind::Rewrite)
            .await;

        queue.mark_running(&job_id).await;
        assert_eq!(
            queue.get_job(&job_id).await.unwrap().status,
            JobStatus::Running
        );

        queue.mark_done(&job_id).await;
        assert_eq!(
            queue.get_job(&job_id).await.unwrap().status,
            JobStatus::Done
        );
    }

    #[tokio::test]
    async fn test_cancel() {
        let queue = JobQueue::new();
        let (job_id, _cancel_rx) = queue
            .enqueue("s1".to_string(), None, JobKind::Deliver)
            .await;

        queue.mark_running(&job_id).await;
        let canceled = queue.cancel(&job_id).await;
        assert!(canceled);
        assert_eq!(
            queue.get_job(&job_id).await.unwrap().status,
            JobStatus::Canceled
        );
    }

    #[tokio::test]
    async fn test_cancel_session() {
        let queue = JobQueue::new();
        let (j1, _) = queue
            .enqueue("s1".to_string(), None, JobKind::Transcribe)
            .await;
        let (j2, _) = queue
            .enqueue("s1".to_string(), None, JobKind::Rewrite)
            .await;
        let (j3, _) = queue
            .enqueue("s2".to_string(), None, JobKind::Transcribe)
            .await;

        let canceled = queue.cancel_session("s1").await;
        assert_eq!(canceled.len(), 2);
        assert!(canceled.contains(&j1));
        assert!(canceled.contains(&j2));
        assert_eq!(
            queue.get_job(&j3).await.unwrap().status,
            JobStatus::Queued
        );
    }

    #[tokio::test]
    async fn test_mark_failed() {
        let queue = JobQueue::new();
        let (job_id, _) = queue
            .enqueue("s1".to_string(), None, JobKind::Transcribe)
            .await;

        queue.mark_failed(&job_id, "STT timeout".to_string()).await;
        let info = queue.get_job(&job_id).await.unwrap();
        assert_eq!(info.status, JobStatus::Failed);
        assert_eq!(info.error.as_deref(), Some("STT timeout"));
    }

    #[tokio::test]
    async fn test_cleanup() {
        let queue = JobQueue::new();
        let (j1, _) = queue
            .enqueue("s1".to_string(), None, JobKind::Transcribe)
            .await;
        let (j2, _) = queue
            .enqueue("s1".to_string(), None, JobKind::Rewrite)
            .await;

        queue.mark_done(&j1).await;
        queue.cleanup_completed().await;

        assert!(queue.get_job(&j1).await.is_none());
        assert!(queue.get_job(&j2).await.is_some());
    }
}
