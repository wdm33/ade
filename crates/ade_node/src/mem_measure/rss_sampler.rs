//! RED: process resident-set-size (RSS) sampler.
//!
//! Reads `/proc/self/status` (Linux) to observe THIS process's memory
//! footprint. Observational only: it reads the OS, returns numbers, and
//! influences NO authoritative output. It is the single MEM-MEASURE site that
//! touches the OS for memory facts. RSS values are nondeterministic
//! (allocator / OS / timing) and therefore NEVER enter a replay fingerprint or
//! any authoritative comparison — they are release-tier evidence only. The
//! percentile math IS deterministic given a fixed sample multiset, but the
//! samples themselves are not, so this whole module is RED.

use std::fs;

/// One RSS observation, in kibibytes (the `/proc/self/status` unit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RssSampleKib(pub u64);

/// Current resident set size (`VmRSS`) of this process, in kiB. `None` if the
/// field is unavailable (non-Linux / unreadable `/proc`) — fail-soft, never a
/// panic into a measurement.
pub fn sample_vm_rss_kib() -> Option<RssSampleKib> {
    read_status_field_kib("VmRSS:").map(RssSampleKib)
}

/// Peak resident set size (`VmHWM`, the high-water mark) of this process, in
/// kiB. Monotonic per process; the basis for the peak evidence field.
pub fn sample_vm_hwm_kib() -> Option<RssSampleKib> {
    read_status_field_kib("VmHWM:").map(RssSampleKib)
}

/// Parse a `kB`-suffixed numeric field from `/proc/self/status`. The line shape
/// is `Field:\t<spaces><number> kB`; the value is already in kiB.
fn read_status_field_kib(field: &str) -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix(field) {
            let num = rest.split_whitespace().next()?;
            return num.parse::<u64>().ok();
        }
    }
    None
}

/// Accumulates RSS observations over a measurement window and derives
/// `p50` / `p95` / `peak`. The percentile math is deterministic given the
/// sample multiset (integer nearest-rank, no float); the samples are
/// nondeterministic OS values, so this is RED evidence, never a fingerprint
/// input.
#[derive(Debug, Clone, Default)]
pub struct RssWindow {
    samples: Vec<u64>,
}

impl RssWindow {
    pub fn new() -> Self {
        RssWindow {
            samples: Vec::new(),
        }
    }

    /// Record an explicit sample (used by tests with fixed sample sets).
    pub fn record(&mut self, s: RssSampleKib) {
        self.samples.push(s.0);
    }

    /// Observe `VmRSS` now and record it if available (no-op off-Linux).
    pub fn observe_now(&mut self) {
        if let Some(s) = sample_vm_rss_kib() {
            self.record(s);
        }
    }

    /// Number of samples collected.
    pub fn count(&self) -> usize {
        self.samples.len()
    }

    /// Peak (max) observed RSS in kiB, or `None` if no samples.
    pub fn peak_kib(&self) -> Option<u64> {
        self.samples.iter().copied().max()
    }

    /// `p`-th percentile (0..=100) by integer nearest-rank, or `None` if empty.
    /// rank = ceil(p * n / 100), 1-based; index = rank - 1, clamped to [0, n-1].
    pub fn percentile_kib(&self, p: u8) -> Option<u64> {
        let n = self.samples.len();
        if n == 0 {
            return None;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        let rank = ((p as usize) * n).div_ceil(100); // ceil(p*n/100), integer-only
        let idx = rank.saturating_sub(1).min(n - 1);
        Some(sorted[idx])
    }

    /// Median (p50).
    pub fn p50_kib(&self) -> Option<u64> {
        self.percentile_kib(50)
    }

    /// p95.
    pub fn p95_kib(&self) -> Option<u64> {
        self.percentile_kib(95)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn vm_rss_sample_present_on_linux() {
        let s = sample_vm_rss_kib();
        if cfg!(target_os = "linux") {
            let v = s.expect("VmRSS must be readable on linux");
            assert!(v.0 > 0, "a running process has nonzero RSS");
        }
        // Off-Linux: `None` is acceptable (fail-soft) — no assertion.
    }

    #[test]
    fn percentile_nearest_rank_is_deterministic() {
        let mut w = RssWindow::new();
        for v in [10u64, 20, 30, 40, 50] {
            w.record(RssSampleKib(v));
        }
        // n=5: p50 rank=ceil(2.5)=3 -> idx2 -> 30; p95 rank=ceil(4.75)=5 -> idx4 -> 50.
        assert_eq!(w.p50_kib(), Some(30));
        assert_eq!(w.p95_kib(), Some(50));
        assert_eq!(w.peak_kib(), Some(50));
        assert_eq!(w.p50_kib(), w.p50_kib(), "deterministic across calls");
    }

    #[test]
    fn empty_window_yields_none() {
        let w = RssWindow::new();
        assert_eq!(w.p50_kib(), None);
        assert_eq!(w.p95_kib(), None);
        assert_eq!(w.peak_kib(), None);
        assert_eq!(w.count(), 0);
    }

    #[test]
    fn peak_is_max_of_samples() {
        let mut w = RssWindow::new();
        for v in [7u64, 3, 99, 42] {
            w.record(RssSampleKib(v));
        }
        assert_eq!(w.peak_kib(), Some(99));
        assert_eq!(w.count(), 4);
    }
}
