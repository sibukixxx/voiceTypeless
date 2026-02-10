use serde::Serialize;
use std::sync::Mutex;

/// ローカルメトリクス収集器
pub struct Metrics {
    counters: Mutex<MetricsCounters>,
    latencies: Mutex<Vec<LatencyRecord>>,
}

#[derive(Debug, Default)]
struct MetricsCounters {
    sessions_started: u64,
    segments_transcribed: u64,
    segments_rewritten: u64,
    segments_delivered: u64,
    errors_permission: u64,
    errors_device: u64,
    errors_stt: u64,
    errors_rewrite: u64,
    errors_internal: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LatencyRecord {
    pub phase: String,
    pub duration_ms: u64,
    pub timestamp: String,
}

/// メトリクスサマリー（UIに返す用）
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSummary {
    pub sessions_started: u64,
    pub segments_transcribed: u64,
    pub segments_rewritten: u64,
    pub segments_delivered: u64,
    pub error_counts: ErrorCounts,
    pub avg_latency_ms: AvgLatency,
    pub recent_latencies: Vec<LatencyRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorCounts {
    pub permission: u64,
    pub device: u64,
    pub stt: u64,
    pub rewrite: u64,
    pub internal: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AvgLatency {
    pub transcribe: Option<f64>,
    pub rewrite: Option<f64>,
    pub deliver: Option<f64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            counters: Mutex::new(MetricsCounters::default()),
            latencies: Mutex::new(Vec::new()),
        }
    }

    pub fn inc_sessions_started(&self) {
        self.counters.lock().unwrap().sessions_started += 1;
    }

    pub fn inc_segments_transcribed(&self) {
        self.counters.lock().unwrap().segments_transcribed += 1;
    }

    pub fn inc_segments_rewritten(&self) {
        self.counters.lock().unwrap().segments_rewritten += 1;
    }

    pub fn inc_segments_delivered(&self) {
        self.counters.lock().unwrap().segments_delivered += 1;
    }

    pub fn inc_error(&self, code: &str) {
        let mut c = self.counters.lock().unwrap();
        match code {
            "E_PERMISSION" => c.errors_permission += 1,
            "E_DEVICE" => c.errors_device += 1,
            "E_STT_UNAVAILABLE" | "E_TIMEOUT" => c.errors_stt += 1,
            "E_REWRITE" => c.errors_rewrite += 1,
            _ => c.errors_internal += 1,
        }
    }

    pub fn record_latency(&self, phase: &str, duration_ms: u64) {
        let record = LatencyRecord {
            phase: phase.to_string(),
            duration_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let mut latencies = self.latencies.lock().unwrap();
        latencies.push(record);
        // 最新1000件のみ保持
        if latencies.len() > 1000 {
            let excess = latencies.len() - 1000;
            latencies.drain(0..excess);
        }
    }

    pub fn summary(&self) -> MetricsSummary {
        let c = self.counters.lock().unwrap();
        let latencies = self.latencies.lock().unwrap();

        let avg = |phase: &str| -> Option<f64> {
            let vals: Vec<f64> = latencies
                .iter()
                .filter(|r| r.phase == phase)
                .map(|r| r.duration_ms as f64)
                .collect();
            if vals.is_empty() {
                None
            } else {
                Some(vals.iter().sum::<f64>() / vals.len() as f64)
            }
        };

        let recent: Vec<LatencyRecord> = latencies
            .iter()
            .rev()
            .take(20)
            .cloned()
            .collect();

        MetricsSummary {
            sessions_started: c.sessions_started,
            segments_transcribed: c.segments_transcribed,
            segments_rewritten: c.segments_rewritten,
            segments_delivered: c.segments_delivered,
            error_counts: ErrorCounts {
                permission: c.errors_permission,
                device: c.errors_device,
                stt: c.errors_stt,
                rewrite: c.errors_rewrite,
                internal: c.errors_internal,
            },
            avg_latency_ms: AvgLatency {
                transcribe: avg("transcribe"),
                rewrite: avg("rewrite"),
                deliver: avg("deliver"),
            },
            recent_latencies: recent,
        }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counters() {
        let m = Metrics::new();
        m.inc_sessions_started();
        m.inc_sessions_started();
        m.inc_segments_transcribed();
        m.inc_error("E_DEVICE");
        m.inc_error("E_PERMISSION");
        m.inc_error("E_INTERNAL");

        let s = m.summary();
        assert_eq!(s.sessions_started, 2);
        assert_eq!(s.segments_transcribed, 1);
        assert_eq!(s.error_counts.device, 1);
        assert_eq!(s.error_counts.permission, 1);
        assert_eq!(s.error_counts.internal, 1);
    }

    #[test]
    fn test_latency_recording() {
        let m = Metrics::new();
        m.record_latency("transcribe", 120);
        m.record_latency("transcribe", 80);
        m.record_latency("rewrite", 200);

        let s = m.summary();
        assert!((s.avg_latency_ms.transcribe.unwrap() - 100.0).abs() < f64::EPSILON);
        assert!((s.avg_latency_ms.rewrite.unwrap() - 200.0).abs() < f64::EPSILON);
        assert!(s.avg_latency_ms.deliver.is_none());
        assert_eq!(s.recent_latencies.len(), 3);
    }

    #[test]
    fn test_latency_cap() {
        let m = Metrics::new();
        for i in 0..1100 {
            m.record_latency("transcribe", i);
        }
        let latencies = m.latencies.lock().unwrap();
        assert_eq!(latencies.len(), 1000);
    }
}
