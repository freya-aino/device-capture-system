//! Hub-side measurements for `/status`: throughput of the fanned-in stream and the process's CPU use.
//!
//! The proxy forwards frames inside `zmq::proxy_steerable`, which gives no byte counts, so a counting
//! subscriber reads the proxy's internal endpoint (the same one the relays use). Counting is one
//! `recv_msg` per part, no copy of the payload. A sampler thread turns the running totals into
//! per-second rates once a second, and reads the process CPU time the same way, so one `GET /status`
//! answers with the rates of the last completed second.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle, sleep, spawn};
use std::time::{Duration, Instant};

use anyhow::{Error, Result};
use zmq::{Context, SUB};

/// Rates over the last completed sampling window, plus running totals since start.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rates {
    pub frames_per_s: f64,
    pub mbit_per_s: f64,
    /// Process CPU time over wall time in the window, in percent of one core (so 200 = two cores).
    /// `None` where the process CPU time is unavailable.
    pub cpu_percent: Option<f64>,
}

#[derive(Debug, Default)]
struct Totals {
    frames: AtomicU64,
    bytes: AtomicU64,
}

/// Shared handle to the hub's counters. Cheap to clone; the status server holds one.
#[derive(Debug, Clone)]
pub struct HubMetrics {
    started: Instant,
    totals: Arc<Totals>,
    rates: Arc<Mutex<Rates>>,
}

impl Default for HubMetrics {
    fn default() -> Self {
        HubMetrics {
            started: Instant::now(),
            totals: Arc::new(Totals::default()),
            rates: Arc::new(Mutex::new(Rates::default())),
        }
    }
}

impl HubMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn uptime(&self) -> Duration {
        self.started.elapsed()
    }

    pub fn frames(&self) -> u64 {
        self.totals.frames.load(Ordering::Relaxed)
    }

    pub fn bytes(&self) -> u64 {
        self.totals.bytes.load(Ordering::Relaxed)
    }

    pub fn rates(&self) -> Rates {
        *self.rates.lock().expect("rates lock poisoned")
    }

    /// Records one forwarded frame of `bytes` (all parts together), for tests that need a stream
    /// without a socket.
    #[cfg(test)]
    pub fn record_frame(&self, bytes: u64) {
        self.totals.frames.fetch_add(1, Ordering::Relaxed);
        self.totals.bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Subscribes to `endpoint` (the proxy's internal fan-out) and counts every frame and byte that
    /// passes. Returns once the socket is connected; the counting runs on its own thread until the
    /// context is terminated.
    pub fn count_stream(&self, context: &Context, endpoint: &str) -> Result<(), Error> {
        let socket = context.socket(SUB)?;
        socket.set_subscribe(b"")?;
        // The counter does almost nothing per frame, so a modest queue is enough; if it ever fell
        // behind, PUB drops frames for this subscriber only and the counts under-report.
        socket.set_rcvhwm(1000)?;
        socket.connect(endpoint)?;
        let totals = self.totals.clone();
        spawn(move || {
            loop {
                let mut bytes = 0u64;
                loop {
                    match socket.recv_msg(0) {
                        Ok(part) => bytes += part.len() as u64,
                        // ETERM: the context is shutting down; anything else is unexpected but not
                        // worth killing the hub over.
                        Err(_) => return,
                    }
                    if !socket.get_rcvmore().unwrap_or(false) {
                        break;
                    }
                }
                totals.frames.fetch_add(1, Ordering::Relaxed);
                totals.bytes.fetch_add(bytes, Ordering::Relaxed);
            }
        });
        Ok(())
    }

    /// Starts the sampler: every `window` it replaces `rates()` with the frame rate, bit rate and CPU
    /// share of the window that just ended.
    pub fn start_sampler(&self, window: Duration) -> JoinHandle<()> {
        let totals = self.totals.clone();
        let rates = self.rates.clone();
        spawn(move || {
            let mut last = Sample::take(&totals);
            loop {
                sleep(window);
                let now = Sample::take(&totals);
                let next = now.rates_since(&last);
                *rates.lock().expect("rates lock poisoned") = next;
                last = now;
            }
        })
    }
}

/// One reading of the counters and the process clock.
#[derive(Debug, Clone, Copy)]
struct Sample {
    at: Instant,
    frames: u64,
    bytes: u64,
    cpu: Option<Duration>,
}

impl Sample {
    fn take(totals: &Totals) -> Self {
        Sample {
            at: Instant::now(),
            frames: totals.frames.load(Ordering::Relaxed),
            bytes: totals.bytes.load(Ordering::Relaxed),
            cpu: process_cpu_time(),
        }
    }

    fn rates_since(&self, earlier: &Sample) -> Rates {
        let secs = self
            .at
            .saturating_duration_since(earlier.at)
            .as_secs_f64()
            .max(f64::EPSILON);
        let cpu_percent = match (self.cpu, earlier.cpu) {
            (Some(now), Some(before)) => {
                Some(now.saturating_sub(before).as_secs_f64() / secs * 100.0)
            }
            _ => None,
        };
        Rates {
            frames_per_s: self.frames.saturating_sub(earlier.frames) as f64 / secs,
            mbit_per_s: self.bytes.saturating_sub(earlier.bytes) as f64 * 8.0 / 1e6 / secs,
            cpu_percent,
        }
    }
}

/// User plus system CPU time of this process, all threads included.
#[cfg(unix)]
pub fn process_cpu_time() -> Option<Duration> {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: `usage` is a valid, writable rusage and RUSAGE_SELF needs no other state.
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    if rc != 0 {
        return None;
    }
    let to_duration = |t: libc::timeval| {
        Duration::from_secs(t.tv_sec.max(0) as u64) + Duration::from_micros(t.tv_usec.max(0) as u64)
    };
    Some(to_duration(usage.ru_utime) + to_duration(usage.ru_stime))
}

#[cfg(not(unix))]
pub fn process_cpu_time() -> Option<Duration> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use zmq::PUB;

    #[test]
    fn rates_since_divides_deltas_by_the_window() {
        let earlier = Sample {
            at: Instant::now(),
            frames: 100,
            bytes: 1_000_000,
            cpu: Some(Duration::from_millis(500)),
        };
        let later = Sample {
            at: earlier.at + Duration::from_secs(2),
            frames: 160,
            bytes: 1_000_000 + 2_500_000,
            cpu: Some(Duration::from_millis(1500)),
        };
        let rates = later.rates_since(&earlier);
        assert!((rates.frames_per_s - 30.0).abs() < 1e-9);
        assert!((rates.mbit_per_s - 10.0).abs() < 1e-9);
        assert!((rates.cpu_percent.unwrap() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn rates_since_reports_no_cpu_without_a_process_clock() {
        let earlier = Sample {
            at: Instant::now(),
            frames: 0,
            bytes: 0,
            cpu: None,
        };
        let later = Sample {
            at: earlier.at + Duration::from_secs(1),
            frames: 1,
            bytes: 1,
            cpu: Some(Duration::from_secs(1)),
        };
        assert_eq!(later.rates_since(&earlier).cpu_percent, None);
    }

    #[test]
    fn process_cpu_time_is_available_and_monotonic_on_unix() {
        let first = process_cpu_time().expect("getrusage works on unix");
        // Burn a little CPU so the clock has a chance to advance.
        let mut x = 0u64;
        for i in 0..5_000_000u64 {
            x = x.wrapping_mul(31).wrapping_add(i);
        }
        std::hint::black_box(x);
        let second = process_cpu_time().unwrap();
        assert!(second >= first);
    }

    #[test]
    fn count_stream_counts_frames_and_all_parts_bytes() {
        let context = Context::new();
        let endpoint = "inproc://metrics-test";
        let publisher = context.socket(PUB).unwrap();
        publisher.bind(endpoint).unwrap();

        let metrics = HubMetrics::new();
        metrics.count_stream(&context, endpoint).unwrap();
        // inproc subscriptions propagate asynchronously; give the SUB a moment.
        sleep(Duration::from_millis(100));

        for _ in 0..5 {
            publisher
                .send_multipart([&b"header"[..], &b"seq"[..], &[0u8; 1000][..]], 0)
                .unwrap();
        }

        let deadline = Instant::now() + Duration::from_secs(3);
        while metrics.frames() < 5 && Instant::now() < deadline {
            sleep(Duration::from_millis(10));
        }
        assert_eq!(metrics.frames(), 5);
        assert_eq!(metrics.bytes(), 5 * (6 + 3 + 1000));
    }

    #[test]
    fn sampler_publishes_rates_of_the_last_window() {
        let metrics = HubMetrics::new();
        let _sampler = metrics.start_sampler(Duration::from_millis(100));
        // A steady stream of one 125 kB frame every 10 ms, until the sampler has closed a window
        // that saw some of it (its first reading may predate the first frame).
        let deadline = Instant::now() + Duration::from_secs(3);
        while metrics.rates().frames_per_s == 0.0 && Instant::now() < deadline {
            metrics.record_frame(125_000);
            sleep(Duration::from_millis(10));
        }
        let rates = metrics.rates();
        // About 100 frames/s and 100 Mbit/s nominal; allow a wide margin for scheduling jitter.
        assert!(rates.frames_per_s > 20.0, "{rates:?}");
        assert!(rates.mbit_per_s > 2.0, "{rates:?}");
        assert!(rates.cpu_percent.is_some());
    }
}
