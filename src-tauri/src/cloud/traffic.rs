//! Drive HTTP payload bytes consumed by the client; not wire or system-wide traffic.
use serde::Serialize;
use std::{
    collections::VecDeque,
    io::{self, Read},
    sync::{Arc, LazyLock, Mutex},
    time::Instant,
};

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub uploaded: u64,
    pub downloaded: u64,
    pub upload_rate: f64,
    pub download_rate: f64,
}
#[derive(Default)]
struct Meter {
    totals: [u64; 2],
    buckets: VecDeque<(u64, [u64; 2])>,
}
impl Meter {
    fn prune(&mut self, second: u64) {
        while self
            .buckets
            .front()
            .is_some_and(|(s, _)| second.saturating_sub(*s) >= 3)
        {
            self.buckets.pop_front();
        }
    }
    fn record(&mut self, second: u64, direction: usize, bytes: u64) {
        self.prune(second);
        self.totals[direction] = self.totals[direction].saturating_add(bytes);
        if self.buckets.back().is_none_or(|(s, _)| *s != second) {
            self.buckets.push_back((second, [0; 2]));
        }
        let values = &mut self.buckets.back_mut().unwrap().1;
        values[direction] = values[direction].saturating_add(bytes);
    }
    fn sample(&mut self, second: u64) -> Sample {
        self.prune(second);
        let mut rates = [0.0; 2];
        for (_, values) in &self.buckets {
            for i in 0..2 {
                rates[i] += values[i] as f64 / 3.0;
            }
        }
        Sample {
            uploaded: self.totals[0],
            downloaded: self.totals[1],
            upload_rate: rates[0],
            download_rate: rates[1],
        }
    }
}
static START: LazyLock<Instant> = LazyLock::new(Instant::now);
static METER: LazyLock<Arc<Mutex<Meter>>> =
    LazyLock::new(|| Arc::new(Mutex::new(Meter::default())));
pub fn sample() -> Sample {
    METER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .sample(START.elapsed().as_secs())
}
pub struct Counting<R> {
    inner: R,
    direction: usize,
    meter: Arc<Mutex<Meter>>,
    progress: Option<crate::progress::Reporter>,
}
impl<R: Read> Read for Counting<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let length = crate::resources::chunk(self.direction, buffer.len());
        let n = self.inner.read(&mut buffer[..length])?;
        crate::resources::pace(self.direction, n)?;
        if n > 0 {
            if let Some(p) = &self.progress {
                p.bytes(n);
            }
            self.meter.lock().unwrap_or_else(|e| e.into_inner()).record(
                START.elapsed().as_secs(),
                self.direction,
                n as u64,
            );
        }
        Ok(n)
    }
}
pub fn download<R: Read>(inner: R) -> Counting<R> {
    Counting {
        inner,
        direction: 1,
        meter: METER.clone(),
        progress: crate::progress::current(),
    }
}
pub fn upload(bytes: Vec<u8>) -> reqwest::blocking::Body {
    let length = bytes.len() as u64;
    if let Some(p) = crate::progress::current() {
        p.body(Some(length));
    }
    reqwest::blocking::Body::sized(
        Counting {
            inner: io::Cursor::new(bytes),
            direction: 0,
            meter: METER.clone(),
            progress: crate::progress::current(),
        },
        length,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rates_expire_but_totals_survive_and_directions_are_independent() {
        let mut m = Meter::default();
        m.record(10, 0, 600);
        m.record(11, 1, 300);
        let s = m.sample(11);
        assert_eq!((s.upload_rate, s.download_rate), (200.0, 100.0));
        assert_eq!(m.sample(13).upload_rate, 0.0);
        let s = m.sample(14);
        assert_eq!((s.uploaded, s.downloaded), (600, 300));
        assert_eq!(s.download_rate, 0.0);
        for second in 15..10000 {
            m.record(second, 0, 1);
        }
        assert!(m.buckets.len() <= 3);
    }
    #[test]
    fn counts_only_consumed_bytes_including_partial_reads() {
        let meter = Arc::new(Mutex::new(Meter::default()));
        let mut reader = Counting {
            inner: io::Cursor::new(vec![1; 10]),
            direction: 1,
            meter: meter.clone(),
            progress: None,
        };
        assert_eq!(meter.lock().unwrap().totals, [0, 0]);
        assert_eq!(reader.read(&mut [0; 4]).unwrap(), 4);
        assert_eq!(meter.lock().unwrap().totals, [0, 4]);
        reader.read_to_end(&mut Vec::new()).unwrap();
        assert_eq!(reader.read(&mut [0; 4]).unwrap(), 0);
        assert_eq!(meter.lock().unwrap().totals, [0, 10]);
    }
}
