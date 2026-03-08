use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

use crate::usage::models::{CostEntry, ProviderId, TrendData, TrendPoint, UsageSnapshot};

#[derive(Clone)]
pub struct UsageStore {
    conn: Arc<Mutex<Connection>>,
}

impl UsageStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|_| "store lock poisoned".to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS snapshots (
              provider TEXT PRIMARY KEY,
              payload TEXT NOT NULL,
              fetched_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cost_entries (
              day TEXT NOT NULL,
              provider TEXT NOT NULL,
              model TEXT NOT NULL,
              input_tokens INTEGER NOT NULL,
              output_tokens INTEGER NOT NULL,
              cache_read_tokens INTEGER NOT NULL,
              cache_write_tokens INTEGER NOT NULL,
              estimated_cost_usd REAL NOT NULL,
              PRIMARY KEY(day, provider, model)
            );
            ",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save_snapshot(&self, snapshot: &UsageSnapshot) -> Result<(), String> {
        let payload = serde_json::to_string(snapshot).map_err(|e| e.to_string())?;
        let fetched = snapshot.fetched_at.timestamp_millis();
        let conn = self.conn.lock().map_err(|_| "store lock poisoned".to_string())?;
        conn.execute(
            "INSERT INTO snapshots(provider, payload, fetched_at) VALUES (?1, ?2, ?3)
            ON CONFLICT(provider) DO UPDATE SET payload=excluded.payload, fetched_at=excluded.fetched_at",
            params![snapshot.provider.as_str(), payload, fetched],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_snapshot(&self, provider: ProviderId) -> Result<Option<UsageSnapshot>, String> {
        let conn = self.conn.lock().map_err(|_| "store lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT payload FROM snapshots WHERE provider = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![provider.as_str()]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let payload: String = row.get(0).map_err(|e| e.to_string())?;
            let snapshot: UsageSnapshot = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
            return Ok(Some(snapshot));
        }
        Ok(None)
    }

    pub fn get_all_snapshots(&self) -> Result<Vec<UsageSnapshot>, String> {
        let conn = self.conn.lock().map_err(|_| "store lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT payload FROM snapshots")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let payload: String = row.get(0).map_err(|e| e.to_string())?;
            if let Ok(snapshot) = serde_json::from_str::<UsageSnapshot>(&payload) {
                out.push(snapshot);
            }
        }
        Ok(out)
    }

    pub fn save_cost_entries(&self, entries: &[CostEntry]) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|_| "store lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for entry in entries {
            tx.execute(
                "INSERT INTO cost_entries(day, provider, model, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, estimated_cost_usd)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(day, provider, model) DO UPDATE SET
                    input_tokens=excluded.input_tokens,
                    output_tokens=excluded.output_tokens,
                    cache_read_tokens=excluded.cache_read_tokens,
                    cache_write_tokens=excluded.cache_write_tokens,
                    estimated_cost_usd=excluded.estimated_cost_usd",
                params![
                    entry.date,
                    entry.provider.as_str(),
                    entry.model,
                    entry.input_tokens,
                    entry.output_tokens,
                    entry.cache_read_tokens,
                    entry.cache_write_tokens,
                    entry.estimated_cost_usd
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_cost_history(&self, provider: ProviderId, days: u32) -> Result<Vec<CostEntry>, String> {
        let conn = self.conn.lock().map_err(|_| "store lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT day, model, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, estimated_cost_usd
                 FROM cost_entries
                 WHERE provider = ?1
                 ORDER BY day DESC
                 LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query(params![provider.as_str(), days.max(1)])
            .map_err(|e| e.to_string())?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            out.push(CostEntry {
                date: row.get(0).map_err(|e| e.to_string())?,
                provider,
                model: row.get(1).map_err(|e| e.to_string())?,
                input_tokens: row.get(2).map_err(|e| e.to_string())?,
                output_tokens: row.get(3).map_err(|e| e.to_string())?,
                cache_read_tokens: row.get(4).map_err(|e| e.to_string())?,
                cache_write_tokens: row.get(5).map_err(|e| e.to_string())?,
                estimated_cost_usd: row.get(6).map_err(|e| e.to_string())?,
            });
        }
        out.reverse();
        Ok(out)
    }

    pub fn get_usage_trends(&self, provider: ProviderId, days: u32) -> Result<TrendData, String> {
        let entries = self.get_cost_history(provider, days)?;
        let mut points = Vec::with_capacity(entries.len());
        let mut total_cost = 0.0f64;
        let mut total_tokens = 0u64;

        for entry in entries {
            let tokens = entry
                .input_tokens
                .saturating_add(entry.output_tokens)
                .saturating_add(entry.cache_read_tokens)
                .saturating_add(entry.cache_write_tokens);
            total_cost += entry.estimated_cost_usd;
            total_tokens = total_tokens.saturating_add(tokens);
            points.push(TrendPoint {
                date: entry.date,
                cost_usd: entry.estimated_cost_usd,
                total_tokens: tokens,
            });
        }

        Ok(TrendData {
            provider,
            days,
            points,
            total_cost_usd: total_cost,
            total_tokens,
        })
    }
}
