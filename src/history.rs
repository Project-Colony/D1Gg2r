use rusqlite::{Connection, params};
use std::path::PathBuf;

use crate::metrics::Snapshot;

/// Stored point for a single metric at a given time.
#[derive(Clone, Debug)]
pub struct HistoryPoint {
    pub timestamp: f64,
    pub cpu: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub net_rx: u64,
    pub net_tx: u64,
}

/// Persistent error state for the history subsystem.
#[derive(Debug, Clone)]
pub enum HistoryError {
    /// Database could not be opened or initialized.
    InitFailed(String),
    /// A write (INSERT/DELETE) failed.
    WriteFailed(String),
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryError::InitFailed(e) => write!(f, "History DB init failed: {e}"),
            HistoryError::WriteFailed(e) => write!(f, "History write failed: {e}"),
        }
    }
}

pub struct History {
    conn: Option<Connection>,
    /// How many seconds of history to keep (default: 24h)
    retention_secs: f64,
    /// Timestamp of last prune operation
    last_prune_time: f64,
    /// Last error encountered, exposed to the UI for user feedback.
    pub last_error: Option<HistoryError>,
}

impl History {
    pub fn open() -> Self {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Set restrictive permissions on the DB directory (Unix only)
        #[cfg(unix)]
        if let Some(parent) = path.parent() {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }

        let conn = match Connection::open(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[digger] Failed to open history database: {e}");
                return Self {
                    conn: None,
                    retention_secs: 86400.0,
                    last_prune_time: 0.0,
                    last_error: Some(HistoryError::InitFailed(e.to_string())),
                };
            }
        };

        if let Err(e) = conn.execute_batch(
            "PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            CREATE TABLE IF NOT EXISTS snapshots (
                timestamp REAL PRIMARY KEY,
                cpu REAL NOT NULL,
                mem_used INTEGER NOT NULL,
                mem_total INTEGER NOT NULL,
                net_rx INTEGER NOT NULL,
                net_tx INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_ts ON snapshots(timestamp);",
        ) {
            eprintln!("[digger] Failed to initialize history tables: {e}");
            return Self {
                conn: None,
                retention_secs: 86400.0,
                last_prune_time: 0.0,
                last_error: Some(HistoryError::InitFailed(e.to_string())),
            };
        }

        // Set restrictive permissions on the DB file itself (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }

        Self {
            conn: Some(conn),
            retention_secs: 86400.0,
            last_prune_time: 0.0,
            last_error: None,
        }
    }

    fn db_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("digger")
            .join("history.db")
    }

    /// Returns true if the history backend is operational.
    pub fn is_available(&self) -> bool {
        self.conn.is_some()
    }

    pub fn record(&mut self, snap: &Snapshot) {
        self.record_batch(&[snap]);
    }

    /// Opt #11: Batch INSERT multiple snapshots in a single transaction.
    pub fn record_batch(&mut self, snapshots: &[&Snapshot]) {
        let Some(conn) = &self.conn else { return };
        if snapshots.is_empty() { return; }

        let result = conn.execute_batch("BEGIN");
        if let Err(e) = result {
            eprintln!("[digger] Failed to begin transaction: {e}");
            self.last_error = Some(HistoryError::WriteFailed(e.to_string()));
            return;
        }

        let mut any_error = false;
        for snap in snapshots {
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO snapshots (timestamp, cpu, mem_used, mem_total, net_rx, net_tx)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    snap.timestamp,
                    snap.cpu_usage_global,
                    snap.memory_used,
                    snap.memory_total,
                    snap.net_rx_bytes,
                    snap.net_tx_bytes,
                ],
            ) {
                eprintln!("[digger] Failed to record snapshot: {e}");
                self.last_error = Some(HistoryError::WriteFailed(e.to_string()));
                any_error = true;
                break;
            }
        }

        let _ = if any_error {
            conn.execute_batch("ROLLBACK")
        } else {
            conn.execute_batch("COMMIT")
        };

        if !any_error {
            // Clear error on success
            if self.last_error.is_some() {
                self.last_error = None;
            }
        }

        // Prune old data every 60 seconds (time-based, not write-count-based)
        if let Some(last) = snapshots.last() {
            if last.timestamp - self.last_prune_time >= 60.0 {
                self.last_prune_time = last.timestamp;
                let cutoff = last.timestamp - self.retention_secs;
                if let Err(e) = conn.execute(
                    "DELETE FROM snapshots WHERE timestamp < ?1",
                    params![cutoff],
                ) {
                    eprintln!("[digger] Failed to prune old history: {e}");
                    self.last_error = Some(HistoryError::WriteFailed(e.to_string()));
                }
            }
        }
    }

    pub fn load_range(&self, from: f64, to: f64) -> Vec<HistoryPoint> {
        let Some(conn) = &self.conn else { return Vec::new() };

        let mut stmt = match conn.prepare(
            "SELECT timestamp, cpu, mem_used, mem_total, net_rx, net_tx
             FROM snapshots WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp ASC",
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[digger] Failed to prepare history query: {e}");
                return Vec::new();
            }
        };

        let result = stmt.query_map(params![from, to], |row| {
            Ok(HistoryPoint {
                timestamp: row.get(0)?,
                cpu: row.get(1)?,
                mem_used: row.get(2)?,
                mem_total: row.get(3)?,
                net_rx: row.get(4)?,
                net_tx: row.get(5)?,
            })
        });
        match result {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                eprintln!("[digger] Failed to load history: {e}");
                Vec::new()
            }
        }
    }

    /// Load history with DB-side downsampling using bucket averaging.
    /// Returns at most `max_points` data points by grouping into time buckets.
    pub fn load_range_downsampled(&self, from: f64, to: f64, max_points: usize) -> Vec<HistoryPoint> {
        let Some(conn) = &self.conn else { return Vec::new() };
        if max_points == 0 {
            return Vec::new();
        }

        let bucket_size = (to - from) / max_points as f64;
        if bucket_size <= 0.0 {
            return self.load_range(from, to);
        }

        // Use SQL to bucket and average
        let mut stmt = match conn.prepare(
            "SELECT
                AVG(timestamp), AVG(cpu),
                CAST(AVG(mem_used) AS INTEGER), CAST(AVG(mem_total) AS INTEGER),
                CAST(AVG(net_rx) AS INTEGER), CAST(AVG(net_tx) AS INTEGER)
             FROM snapshots
             WHERE timestamp >= ?1 AND timestamp <= ?2
             GROUP BY CAST((timestamp - ?1) / ?3 AS INTEGER)
             ORDER BY 1 ASC",
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[digger] Failed to prepare downsampled query: {e}");
                return self.load_range(from, to);
            }
        };

        let result = stmt.query_map(params![from, to, bucket_size], |row| {
            Ok(HistoryPoint {
                timestamp: row.get(0)?,
                cpu: row.get(1)?,
                mem_used: row.get(2)?,
                mem_total: row.get(3)?,
                net_rx: row.get(4)?,
                net_tx: row.get(5)?,
            })
        });
        match result {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                eprintln!("[digger] Failed to load downsampled history: {e}");
                Vec::new()
            }
        }
    }

    pub fn load_last_n_seconds_downsampled(&self, seconds: f64, max_points: usize) -> Vec<HistoryPoint> {
        let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
        self.load_range_downsampled(now - seconds, now, max_points)
    }

    /// Export history within a time range to CSV format.
    /// Opt #12: Streams rows directly from the query to avoid loading all into memory.
    pub fn export_csv(&self, from: f64, to: f64) -> String {
        let Some(conn) = &self.conn else { return String::new() };

        let mut out = String::from("timestamp,cpu_percent,mem_used_bytes,mem_total_bytes,net_rx_bytes,net_tx_bytes\n");
        let mut stmt = match conn.prepare(
            "SELECT timestamp, cpu, mem_used, mem_total, net_rx, net_tx
             FROM snapshots WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp ASC",
        ) {
            Ok(s) => s,
            Err(_) => return out,
        };

        let rows = stmt.query_map(params![from, to], |row| {
            Ok((
                row.get::<_, f64>(0)?,
                row.get::<_, f32>(1)?,
                row.get::<_, u64>(2)?,
                row.get::<_, u64>(3)?,
                row.get::<_, u64>(4)?,
                row.get::<_, u64>(5)?,
            ))
        });

        if let Ok(rows) = rows {
            for row in rows.flatten() {
                use std::fmt::Write;
                let _ = writeln!(out, "{},{:.2},{},{},{},{}", row.0, row.1, row.2, row.3, row.4, row.5);
            }
        }
        out
    }

    /// Export history within a time range to JSON format.
    /// Opt #12: Streams rows directly from the query.
    pub fn export_json(&self, from: f64, to: f64) -> String {
        let Some(conn) = &self.conn else { return String::from("[]") };

        let mut stmt = match conn.prepare(
            "SELECT timestamp, cpu, mem_used, mem_total, net_rx, net_tx
             FROM snapshots WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp ASC",
        ) {
            Ok(s) => s,
            Err(_) => return String::from("[]"),
        };

        let rows = stmt.query_map(params![from, to], |row| {
            Ok((
                row.get::<_, f64>(0)?,
                row.get::<_, f32>(1)?,
                row.get::<_, u64>(2)?,
                row.get::<_, u64>(3)?,
                row.get::<_, u64>(4)?,
                row.get::<_, u64>(5)?,
            ))
        });

        let mut out = String::from("[\n");
        let mut first = true;
        if let Ok(rows) = rows {
            for row in rows.flatten() {
                use std::fmt::Write;
                if !first { out.push_str(",\n"); }
                first = false;
                let _ = write!(
                    out,
                    r#"  {{"timestamp":{:.3},"cpu":{:.2},"mem_used":{},"mem_total":{},"net_rx":{},"net_tx":{}}}"#,
                    row.0, row.1, row.2, row.3, row.4, row.5,
                );
            }
        }
        out.push_str("\n]");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_db() -> History {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS snapshots (
                timestamp REAL PRIMARY KEY,
                cpu REAL NOT NULL,
                mem_used INTEGER NOT NULL,
                mem_total INTEGER NOT NULL,
                net_rx INTEGER NOT NULL,
                net_tx INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_ts ON snapshots(timestamp);",
        ).unwrap();
        History {
            conn: Some(conn),
            retention_secs: 86400.0,
            last_prune_time: 0.0,
            last_error: None,
        }
    }

    fn make_snapshot(ts: f64, cpu: f32) -> Snapshot {
        use std::sync::Arc;
        Snapshot {
            timestamp: ts,
            cpu_usage_per_core: vec![cpu],
            cpu_usage_global: cpu,
            cpu_name: String::new(),
            cpu_core_count: 1,
            cpu_frequency_mhz: 0,
            memory_used: 4_000_000_000,
            memory_total: 8_000_000_000,
            swap_used: 0,
            swap_total: 0,
            disks: vec![],
            disk_io: crate::metrics::DiskIoSnapshot { read_bytes: 0, write_bytes: 0 },
            net_rx_bytes: 1000,
            net_tx_bytes: 2000,
            net_interfaces: vec![],
            temperatures: vec![],
            processes: vec![],
            gpu: crate::gpu::GpuSnapshot::default(),
            uptime_secs: 3600,
            process_count: 100,
            sys_info: Arc::new(crate::metrics::SystemInfo {
                os_name: String::new(),
                os_version: String::new(),
                kernel_version: String::new(),
                hostname: String::new(),
            }),
            load_avg: [0.0, 0.0, 0.0],
        }
    }

    #[test]
    fn test_record_and_load() {
        let mut db = make_test_db();
        let snap = make_snapshot(1000.0, 42.5);
        db.record(&snap);
        assert!(db.last_error.is_none());

        let points = db.load_range(999.0, 1001.0);
        assert_eq!(points.len(), 1);
        assert!((points[0].cpu - 42.5).abs() < 0.01);
        assert_eq!(points[0].mem_used, 4_000_000_000);
    }

    #[test]
    fn test_load_empty() {
        let db = make_test_db();
        let points = db.load_range(0.0, 1000.0);
        assert!(points.is_empty());
    }

    #[test]
    fn test_pruning() {
        let mut db = make_test_db();
        db.retention_secs = 50.0;

        // Record snapshots spanning 120 seconds to trigger time-based pruning
        for i in 0..120 {
            let snap = make_snapshot(1000.0 + i as f64, 50.0);
            db.record(&snap);
        }
        // Pruning triggers at ~60s intervals.
        // At ts=1060: cutoff = 1060 - 50 = 1010, removes 1000..1009 (10 points)
        // At ts=1119: cutoff = 1119 - 50 = 1069, removes 1010..1068 (59 more points)
        // Should have ~51 points remaining (1069..1119)
        let points = db.load_range(0.0, 2000.0);
        assert!(points.len() < 120, "expected fewer than 120 points after pruning, got {}", points.len());
        assert!(!points.is_empty());
    }

    #[test]
    fn test_export_csv() {
        let mut db = make_test_db();
        db.record(&make_snapshot(1000.0, 55.0));
        db.record(&make_snapshot(1001.0, 60.0));

        let csv = db.export_csv(999.0, 1002.0);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 rows
        assert!(lines[0].starts_with("timestamp"));
        assert!(lines[1].contains("55.00"));
    }

    #[test]
    fn test_export_json() {
        let mut db = make_test_db();
        db.record(&make_snapshot(1000.0, 55.0));

        let json = db.export_json(999.0, 1002.0);
        assert!(json.starts_with('['));
        assert!(json.contains("\"cpu\":55.00"));
    }

    #[test]
    fn test_unavailable_db_graceful() {
        let db = History {
            conn: None,
            retention_secs: 86400.0,
            last_prune_time: 0.0,
            last_error: Some(HistoryError::InitFailed("test".into())),
        };
        assert!(!db.is_available());
        assert!(db.load_range(0.0, 1000.0).is_empty());
    }
}
