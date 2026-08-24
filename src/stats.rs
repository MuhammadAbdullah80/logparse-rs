//! Rollup accumulators. Everything here is O(distinct keys) in memory, never
//! O(lines), so the tool streams arbitrarily large logs.

use std::collections::HashMap;
use std::io::{self, Write};

use crate::parse::Entry;

/// Default number of rows each "top N" table prints when `--top` is not given.
pub const DEFAULT_TOP_N: usize = 10;

/// Running totals over every line seen so far.
#[derive(Default)]
pub struct Summary {
    /// Rows per table; `None` means use `DEFAULT_TOP_N`.
    top_n: Option<usize>,
    total: u64,
    malformed: u64,
    bytes: u64,
    /// Counts indexed by status class: index 0 is 1xx, index 4 is 5xx.
    by_class: [u64; 5],
    per_path: HashMap<String, PathStat>,
    per_host: HashMap<String, u64>,
}

/// Per-path request count and cumulative duration, for the slowest-paths table.
#[derive(Default)]
struct PathStat {
    hits: u64,
    total_ms: u64,
    timed_hits: u64,
}

impl PathStat {
    /// Mean duration across the hits that carried a timing field.
    fn mean_ms(&self) -> f64 {
        if self.timed_hits == 0 {
            0.0
        } else {
            self.total_ms as f64 / self.timed_hits as f64
        }
    }
}

impl Summary {
    /// Sets how many rows each table prints.
    pub fn set_top_n(&mut self, n: usize) {
        self.top_n = Some(n);
    }

    /// Rows to print per table.
    fn top(&self) -> usize {
        self.top_n.unwrap_or(DEFAULT_TOP_N)
    }

    /// Folds one parsed entry into the totals.
    pub fn record(&mut self, e: &Entry) {
        self.total += 1;
        self.bytes += e.bytes;
        if let Some(slot) = class_index(e.status) {
            self.by_class[slot] += 1;
        }

        let stat = self.per_path.entry(e.path.clone()).or_default();
        stat.hits += 1;
        if let Some(ms) = e.duration_ms {
            stat.total_ms += ms;
            stat.timed_hits += 1;
        }

        *self.per_host.entry(e.host.clone()).or_insert(0) += 1;
    }

    /// Counts a line the parser could not understand.
    pub fn record_malformed(&mut self) {
        self.malformed += 1;
    }

    /// Writes the human-readable rollup.
    pub fn report(&self, out: &mut impl Write) -> io::Result<()> {
        writeln!(out, "requests   {}", self.total)?;
        writeln!(out, "malformed  {}", self.malformed)?;
        writeln!(out, "bytes      {}", human_bytes(self.bytes))?;
        writeln!(out)?;

        writeln!(out, "by status class")?;
        for (i, count) in self.by_class.iter().enumerate() {
            if *count > 0 {
                let pct = 100.0 * *count as f64 / self.total.max(1) as f64;
                writeln!(out, "  {}xx  {:>8}  {:>5.1}%", i + 1, count, pct)?;
            }
        }

        writeln!(out, "\nslowest paths (mean)")?;
        let mut paths: Vec<_> = self.per_path.iter().collect();
        paths.sort_by(|a, b| {
            b.1.mean_ms()
                .partial_cmp(&a.1.mean_ms())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (path, stat) in paths.iter().take(self.top()).filter(|(_, s)| s.timed_hits > 0) {
            writeln!(out, "  {:>8.1}ms  {:>6} hits  {}", stat.mean_ms(), stat.hits, path)?;
        }

        writeln!(out, "\nnoisiest clients")?;
        let mut hosts: Vec<_> = self.per_host.iter().collect();
        hosts.sort_by(|a, b| b.1.cmp(a.1));
        for (host, hits) in hosts.iter().take(self.top()) {
            writeln!(out, "  {:>8}  {}", hits, host)?;
        }
        Ok(())
    }
}

/// Maps an HTTP status to its class bucket, or `None` if it is out of range.
fn class_index(status: u16) -> Option<usize> {
    match status {
        100..=599 => Some((status / 100) as usize - 1),
        _ => None,
    }
}

/// Formats a byte count with a binary unit suffix.
fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, status: u16, ms: Option<u64>) -> Entry {
        Entry {
            host: "10.0.0.1".into(),
            path: path.into(),
            status,
            bytes: 100,
            duration_ms: ms,
        }
    }

    #[test]
    fn buckets_status_classes() {
        assert_eq!(class_index(200), Some(1));
        assert_eq!(class_index(404), Some(3));
        assert_eq!(class_index(503), Some(4));
        assert_eq!(class_index(42), None);
    }

    #[test]
    fn averages_only_timed_hits() {
        let mut s = Summary::default();
        s.record(&entry("/a", 200, Some(100)));
        s.record(&entry("/a", 200, None));
        assert_eq!(s.per_path["/a"].hits, 2);
        assert_eq!(s.per_path["/a"].mean_ms(), 100.0);
    }

    #[test]
    fn formats_byte_counts() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(2048), "2.0 KiB");
    }
}
