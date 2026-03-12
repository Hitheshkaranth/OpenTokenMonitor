use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

use crate::usage::models::{CostEntry, ModelBreakdownEntry, ProviderId, TrendData, TrendPoint, UsageSnapshot};

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
                   AND day IN (
                     SELECT day
                     FROM (
                       SELECT DISTINCT day
                       FROM cost_entries
                       WHERE provider = ?1
                       ORDER BY day DESC
                       LIMIT ?2
                     )
                   )
                 ORDER BY day DESC, model ASC",
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
        let mut by_day = std::collections::BTreeMap::<String, TrendPoint>::new();
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
            let point = by_day.entry(entry.date.clone()).or_insert_with(|| TrendPoint {
                date: entry.date,
                cost_usd: 0.0,
                total_tokens: 0,
            });
            point.cost_usd += entry.estimated_cost_usd;
            point.total_tokens = point.total_tokens.saturating_add(tokens);
        }

        let points = by_day.into_values().collect();

        Ok(TrendData {
            provider,
            days,
            points,
            total_cost_usd: total_cost,
            total_tokens,
        })
    }

    pub fn get_model_breakdown(&self, provider: ProviderId, days: u32) -> Result<Vec<ModelBreakdownEntry>, String> {
        let effective_days = days.max(1);
        let entries = self.get_cost_history(provider, effective_days)?;
        let mut by_model = std::collections::BTreeMap::<String, ModelBreakdownEntry>::new();

        for entry in entries {
            let total_tokens = entry
                .input_tokens
                .saturating_add(entry.output_tokens)
                .saturating_add(entry.cache_read_tokens)
                .saturating_add(entry.cache_write_tokens);
            let slot = by_model
                .entry(entry.model.clone())
                .or_insert_with(|| ModelBreakdownEntry {
                    provider,
                    model: entry.model.clone(),
                    days: effective_days,
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                    total_tokens: 0,
                    estimated_cost_usd: 0.0,
                });
            slot.input_tokens = slot.input_tokens.saturating_add(entry.input_tokens);
            slot.output_tokens = slot.output_tokens.saturating_add(entry.output_tokens);
            slot.cache_read_tokens = slot.cache_read_tokens.saturating_add(entry.cache_read_tokens);
            slot.cache_write_tokens = slot.cache_write_tokens.saturating_add(entry.cache_write_tokens);
            slot.total_tokens = slot.total_tokens.saturating_add(total_tokens);
            slot.estimated_cost_usd += entry.estimated_cost_usd;
        }

        let mut out: Vec<_> = by_model.into_values().collect();
        out.sort_by(|a, b| {
            b.estimated_cost_usd
                .partial_cmp(&a.estimated_cost_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.total_tokens.cmp(&a.total_tokens))
                .then(a.model.cmp(&b.model))
        });
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> UsageStore {
        let path = std::env::temp_dir().join(format!(
            "otm-store-{name}-{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ));
        UsageStore::open(&path).expect("open temp store")
    }

    fn sample_entry(day: &str, model: &str, input_tokens: u64, output_tokens: u64, cost: f64) -> CostEntry {
        CostEntry {
            date: day.to_string(),
            provider: ProviderId::Codex,
            model: model.to_string(),
            input_tokens,
            output_tokens,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            estimated_cost_usd: cost,
        }
    }

    #[test]
    fn cost_history_limits_by_distinct_days_not_rows() {
        let store = temp_store("distinct-days");
        let entries = vec![
            sample_entry("2026-03-01", "gpt-5", 10, 5, 1.0),
            sample_entry("2026-03-01", "gpt-5-mini", 4, 2, 0.2),
            sample_entry("2026-03-02", "gpt-5", 8, 4, 0.8),
        ];
        store.save_cost_entries(&entries).expect("save cost entries");

        let history = store
            .get_cost_history(ProviderId::Codex, 2)
            .expect("load cost history");
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn model_breakdown_aggregates_rows_per_model() {
        let store = temp_store("breakdown");
        let entries = vec![
            sample_entry("2026-03-01", "gpt-5", 10, 5, 1.0),
            sample_entry("2026-03-02", "gpt-5", 12, 6, 1.2),
            sample_entry("2026-03-02", "gpt-5-mini", 4, 2, 0.2),
        ];
        store.save_cost_entries(&entries).expect("save cost entries");

        let breakdown = store
            .get_model_breakdown(ProviderId::Codex, 30)
            .expect("load model breakdown");
        assert_eq!(breakdown.len(), 2);
        assert_eq!(breakdown[0].model, "gpt-5");
        assert_eq!(breakdown[0].input_tokens, 22);
        assert_eq!(breakdown[0].output_tokens, 11);
    }

    #[test]
    fn usage_trends_aggregate_multiple_models_into_one_day() {
        let store = temp_store("trends");
        let entries = vec![
            sample_entry("2026-03-01", "gpt-5", 10, 5, 1.0),
            sample_entry("2026-03-01", "gpt-5-mini", 4, 2, 0.2),
        ];
        store.save_cost_entries(&entries).expect("save cost entries");

        let trend = store
            .get_usage_trends(ProviderId::Codex, 30)
            .expect("load usage trends");
        assert_eq!(trend.points.len(), 1);
        assert_eq!(trend.points[0].date, "2026-03-01");
        assert_eq!(trend.points[0].total_tokens, 21);
        assert!((trend.points[0].cost_usd - 1.2).abs() < f64::EPSILON);
    }
}
