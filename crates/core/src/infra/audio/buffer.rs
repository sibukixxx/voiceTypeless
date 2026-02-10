use ringbuf::{
    traits::{Consumer, Observer, Producer, Split},
    HeapCons, HeapProd, HeapRb,
};

const DEFAULT_SAMPLE_RATE: u32 = 16_000;

/// ロックフリーのシングルプロデューサ/シングルコンシューマ リングバッファ。
/// cpal コールバックスレッド (producer) と処理スレッド (consumer) 間で音声サンプルを転送する。
pub struct RingAudioBuffer {
    producer: HeapProd<f32>,
    consumer: HeapCons<f32>,
}

impl RingAudioBuffer {
    /// 指定秒数分のキャパシティを持つリングバッファを生成する (16kHz mono 基準)。
    pub fn new(seconds: f32) -> Self {
        let capacity = (DEFAULT_SAMPLE_RATE as f32 * seconds) as usize;
        Self::with_capacity(capacity)
    }

    /// 指定サンプル数のキャパシティを持つリングバッファを生成する。
    pub fn with_capacity(capacity: usize) -> Self {
        let rb = HeapRb::<f32>::new(capacity);
        let (producer, consumer) = rb.split();
        Self { producer, consumer }
    }

    /// Producer/Consumer に分割して別スレッドで使用可能にする。
    pub fn split(self) -> (RingAudioProducer, RingAudioConsumer) {
        (
            RingAudioProducer { inner: self.producer },
            RingAudioConsumer { inner: self.consumer },
        )
    }
}

/// リングバッファの書き込み側。cpal コールバックスレッドで使用。
pub struct RingAudioProducer {
    inner: HeapProd<f32>,
}

// cpal コールバックから使うため Send が必要
unsafe impl Send for RingAudioProducer {}

impl RingAudioProducer {
    /// サンプルをバッファに追加する。戻り値は実際に書き込まれたサンプル数。
    pub fn push(&mut self, samples: &[f32]) -> usize {
        self.inner.push_slice(samples)
    }
}

/// リングバッファの読み取り側。処理スレッドで使用。
pub struct RingAudioConsumer {
    inner: HeapCons<f32>,
}

unsafe impl Send for RingAudioConsumer {}

impl RingAudioConsumer {
    /// バッファから最大 buf.len() サンプルを読み取る。戻り値は実際に読んだサンプル数。
    pub fn pop(&mut self, buf: &mut [f32]) -> usize {
        self.inner.pop_slice(buf)
    }

    /// 読み取り可能なサンプル数。
    pub fn available(&self) -> usize {
        self.inner.occupied_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop_roundtrip() {
        let buf = RingAudioBuffer::with_capacity(1024);
        let (mut prod, mut cons) = buf.split();

        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let written = prod.push(&input);
        assert_eq!(written, 5);
        assert_eq!(cons.available(), 5);

        let mut output = vec![0.0; 5];
        let read = cons.pop(&mut output);
        assert_eq!(read, 5);
        assert_eq!(output, input);
        assert_eq!(cons.available(), 0);
    }

    #[test]
    fn pop_partial() {
        let buf = RingAudioBuffer::with_capacity(1024);
        let (mut prod, mut cons) = buf.split();

        let input = vec![1.0, 2.0, 3.0];
        prod.push(&input);

        let mut output = vec![0.0; 2];
        let read = cons.pop(&mut output);
        assert_eq!(read, 2);
        assert_eq!(output, vec![1.0, 2.0]);
        assert_eq!(cons.available(), 1);
    }

    #[test]
    fn pop_from_empty() {
        let buf = RingAudioBuffer::with_capacity(1024);
        let (_prod, mut cons) = buf.split();

        let mut output = vec![0.0; 5];
        let read = cons.pop(&mut output);
        assert_eq!(read, 0);
    }

    #[test]
    fn overflow_drops_oldest_not_written() {
        let buf = RingAudioBuffer::with_capacity(4);
        let (mut prod, mut cons) = buf.split();

        // キャパシティ4に5サンプルを書き込もうとする → 4つのみ書かれる
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let written = prod.push(&input);
        assert_eq!(written, 4);

        let mut output = vec![0.0; 4];
        cons.pop(&mut output);
        assert_eq!(output, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn new_with_seconds() {
        let buf = RingAudioBuffer::new(2.0);
        let (mut prod, _cons) = buf.split();
        // 2秒 @ 16kHz = 32000サンプル
        let samples = vec![0.0; 32000];
        let written = prod.push(&samples);
        assert_eq!(written, 32000);
    }
}
