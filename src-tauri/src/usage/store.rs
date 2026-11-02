use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};

use crate::usage::models::{
    CacheInsights, CostEntry, HistorySample, ModelBreakdownEntry, ProviderId, TrendData,
    TrendPoint, UsageSnapshot, WindowType,
};

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
        store.purge_legacy_rows()?;
        Ok(store)
    }

    /// Delete cost rows the current scanners can never write: unversioned
    /// model keys (`claude-opus`), un-stripped vendor prefixes, and retired
    /// provider ids (`gemini`). Migrations v1/v2 did this once, but an older
    /// build sharing the same database (e.g. a not-yet-updated install) can
    /// write them again, and since a fresh scan writes the versioned key
    /// alongside, the same usage is then counted twice. Running on every open
    /// is cheap and idempotent; the rows are re-derived from local logs.
    fn purge_legacy_rows(&self) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        // Derive the allowlist from ProviderId so a newly added provider's
        // history is never purged by a stale hardcoded list.
        let known = ProviderId::all()
            .iter()
            .map(|p| format!("'{}'", p.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        let removed = conn
            .execute(
                &format!(
                    "DELETE FROM cost_entries
                      WHERE model IN ('claude-opus', 'claude-sonnet', 'claude-haiku')
                         OR model LIKE '%/%'
                         OR provider NOT IN ({known})"
                ),
                [],
            )
            .map_err(|e| e.to_string())?;
        if removed > 0 {
            tracing::info!("[store] purged {removed} legacy cost rows on open");
        }
        Ok(())
    }

    fn init_schema(&self) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
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
        drop(conn);
        self.migrate()
    }

    /// Schema/data migrations, tracked with `PRAGMA user_version`.
    ///
    /// Each step must be safe to skip on a fresh database and safe to run
    /// exactly once on an existing one, and **each step stamps its own version
    /// as soon as it completes**. Bumping the version once at the end instead
    /// means a build carrying the new version constant but not yet the new step
    /// marks the work done without doing it, stranding the database.
    fn migrate(&self) -> Result<(), String> {
        const LATEST_VERSION: i64 = 4;

        let conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;

        if version >= LATEST_VERSION {
            return Ok(());
        }

        if version < 1 {
            // v1 — drop cost rows keyed on model names the normalizers can no
            // longer produce.
            //
            // `normalize_claude_model` used to collapse every Claude id to a
            // bare family name, and the normalizers didn't strip vendor
            // prefixes. Those rows are now orphans: `cost_entries` is keyed
            // (day, provider, model), so a fresh scan writes `claude-opus-4-5`
            // alongside the stale `claude-opus` row instead of replacing it,
            // and the model breakdown counts the same usage twice.
            //
            // Deleting is safe — every row here is re-derived from the local
            // CLI logs on the next refresh.
            let removed = conn
                .execute(
                    "DELETE FROM cost_entries
                      WHERE model IN ('claude-opus', 'claude-sonnet', 'claude-haiku')
                         OR model LIKE '%/%'",
                    [],
                )
                .map_err(|e| e.to_string())?;
            if removed > 0 {
                tracing::info!(
                    "[store] migration v1: dropped {removed} cost rows using legacy model keys"
                );
            }
            conn.execute_batch("PRAGMA user_version = 1")
                .map_err(|e| e.to_string())?;
        }

        if version < 2 {
            // v2 — drop rows belonging to provider ids that no longer exist.
            //
            // The Gemini provider was renamed to Antigravity, but its rows were
            // left behind under provider='gemini'. Every read filters on a
            // current `ProviderId`, so these can never be displayed, refreshed,
            // or corrected — they just sit in the file forever.
            let removed = conn
                .execute(
                    "DELETE FROM cost_entries
                      WHERE provider NOT IN ('claude', 'codex', 'antigravity')",
                    [],
                )
                .map_err(|e| e.to_string())?;
            if removed > 0 {
                tracing::info!(
                    "[store] migration v2: dropped {removed} cost rows for retired providers"
                );
            }
            conn.execute_batch("PRAGMA user_version = 2")
                .map_err(|e| e.to_string())?;
        }

        if version < 3 {
            // v3 — utilization history. `snapshots` only keeps the latest row
            // per provider, so nothing could say how fast a window is filling.
            // One row per window per fresh fetch feeds the time-to-limit
            // forecast; `append_snapshot_history` prunes it to 30 days.
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS snapshot_history (
                   provider TEXT NOT NULL,
                   window_type TEXT NOT NULL,
                   ts INTEGER NOT NULL,
                   utilization REAL NOT NULL,
                   used INTEGER,
                   resets_at INTEGER,
                   PRIMARY KEY(provider, window_type, ts)
                 );
                 PRAGMA user_version = 3;",
            )
            .map_err(|e| e.to_string())?;
        }

        if version < 4 {
            // v4 — Codex rows stored `input_tokens` including the cached part
            // while `cache_read_tokens` held that same cached part, so every
            // token total (input + cache_read + ...) counted cached input
            // twice. New scans store the uncached remainder; this corrects rows
            // a refresh won't rewrite (it only rescans recent days). Runs in
            // one transaction with the version stamp so it can't apply twice.
            conn.execute_batch(
                "BEGIN;
                 UPDATE cost_entries
                    SET input_tokens = MAX(input_tokens - cache_read_tokens, 0)
                  WHERE provider = 'codex';
                 PRAGMA user_version = 4;
                 COMMIT;",
            )
            .map_err(|e| e.to_string())?;
        }

        debug_assert_eq!(
            conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .map_err(|e| e.to_string())?,
            LATEST_VERSION,
            "every migration step must stamp its own user_version"
        );
        Ok(())
    }

    pub fn save_snapshot(&self, snapshot: &UsageSnapshot) -> Result<(), String> {
        let payload = serde_json::to_string(snapshot).map_err(|e| e.to_string())?;
        let fetched = snapshot.fetched_at.timestamp_millis();
        let conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        conn.execute(
            "INSERT INTO snapshots(provider, payload, fetched_at) VALUES (?1, ?2, ?3)
            ON CONFLICT(provider) DO UPDATE SET payload=excluded.payload, fetched_at=excluded.fetched_at",
            params![snapshot.provider.as_str(), payload, fetched],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_snapshot(&self, provider: ProviderId) -> Result<Option<UsageSnapshot>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT payload FROM snapshots WHERE provider = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query(params![provider.as_str()])
            .map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let payload: String = row.get(0).map_err(|e| e.to_string())?;
            let snapshot: UsageSnapshot =
                serde_json::from_str(&payload).map_err(|e| e.to_string())?;
            return Ok(Some(snapshot));
        }
        Ok(None)
    }

    pub fn get_all_snapshots(&self) -> Result<Vec<UsageSnapshot>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT payload FROM snapshots")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let payload: String = row.get(0).map_err(|e| e.to_string())?;
            match serde_json::from_str::<UsageSnapshot>(&payload) {
                Ok(snapshot) => out.push(snapshot),
                Err(e) => tracing::warn!("[store] skipping unreadable snapshot row: {e}"),
            }
        }
        Ok(out)
    }

    /// Append one history row per window of a freshly fetched snapshot, then
    /// drop this provider's rows older than 30 days. A repeat of the same
    /// `fetched_at` is ignored.
    pub fn append_snapshot_history(&self, snapshot: &UsageSnapshot) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        let ts = snapshot.fetched_at.timestamp();
        for window in &snapshot.windows {
            conn.execute(
                "INSERT OR IGNORE INTO snapshot_history
                   (provider, window_type, ts, utilization, used, resets_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    snapshot.provider.as_str(),
                    window.window_type.as_str(),
                    ts,
                    window.utilization,
                    window.used.map(|v| v as i64),
                    window.resets_at.map(|d| d.timestamp()),
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        let cutoff = chrono::Utc::now().timestamp() - 30 * 24 * 60 * 60;
        conn.execute(
            "DELETE FROM snapshot_history WHERE provider = ?1 AND ts < ?2",
            params![snapshot.provider.as_str(), cutoff],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// History rows for one provider window at or after `since_unix`, oldest first.
    pub fn get_snapshot_history(
        &self,
        provider: ProviderId,
        window_type: WindowType,
        since_unix: i64,
    ) -> Result<Vec<HistorySample>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT ts, utilization, used, resets_at
                 FROM snapshot_history
                 WHERE provider = ?1 AND window_type = ?2 AND ts >= ?3
                 ORDER BY ts ASC",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query(params![provider.as_str(), window_type.as_str(), since_unix])
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            out.push(HistorySample {
                ts: row.get(0).map_err(|e| e.to_string())?,
                utilization: row.get(1).map_err(|e| e.to_string())?,
                used: row
                    .get::<_, Option<i64>>(2)
                    .map_err(|e| e.to_string())?
                    .map(|v| v.max(0) as u64),
                resets_at: row.get(3).map_err(|e| e.to_string())?,
            });
        }
        Ok(out)
    }

    pub fn save_cost_entries(&self, entries: &[CostEntry]) -> Result<(), String> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
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

    /// Cost rows from the last `days` calendar days (UTC, today included),
    /// oldest first. This is a calendar window, not "the last N days that had
    /// activity": the latter let a "7d" view reach back weeks for an idle
    /// provider, so token totals and spend covered different periods.
    pub fn get_cost_history(
        &self,
        provider: ProviderId,
        days: u32,
    ) -> Result<Vec<CostEntry>, String> {
        let since_day = (chrono::Utc::now().date_naive()
            - chrono::Duration::days(i64::from(days.max(1)) - 1))
        .format("%Y-%m-%d")
        .to_string();

        let conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT day, model, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, estimated_cost_usd
                 FROM cost_entries
                 WHERE provider = ?1 AND day >= ?2
                 ORDER BY day DESC, model ASC",
            )
            .map_err(|e| e.to_string())?;

        let mut rows = stmt
            .query(params![provider.as_str(), since_day])
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
            let point = by_day
                .entry(entry.date.clone())
                .or_insert_with(|| TrendPoint {
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

    pub fn get_model_breakdown(
        &self,
        provider: ProviderId,
        days: u32,
    ) -> Result<Vec<ModelBreakdownEntry>, String> {
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
                    cache_savings_usd: 0.0,
                });
            slot.input_tokens = slot.input_tokens.saturating_add(entry.input_tokens);
            slot.output_tokens = slot.output_tokens.saturating_add(entry.output_tokens);
            slot.cache_read_tokens = slot
                .cache_read_tokens
                .saturating_add(entry.cache_read_tokens);
            slot.cache_write_tokens = slot
                .cache_write_tokens
                .saturating_add(entry.cache_write_tokens);
            slot.total_tokens = slot.total_tokens.saturating_add(total_tokens);
            slot.estimated_cost_usd += entry.estimated_cost_usd;
        }

        let mut out: Vec<_> = by_model.into_values().collect();
        for entry in &mut out {
            entry.cache_savings_usd =
                crate::pricing::cache_savings_usd(provider, &entry.model, entry.cache_read_tokens);
        }
        out.sort_by(|a, b| {
            b.estimated_cost_usd
                .partial_cmp(&a.estimated_cost_usd)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.total_tokens.cmp(&a.total_tokens))
                .then(a.model.cmp(&b.model))
        });
        Ok(out)
    }

    /// Cache payoff for `provider` as of `today` (a UTC day, matching how cost
    /// rows are keyed): this month with vs without caching, plus the last 7 days'
    /// hit rate, prompt volume and blended rates.
    pub fn get_cache_insights(
        &self,
        provider: ProviderId,
        today: chrono::NaiveDate,
    ) -> Result<CacheInsights, String> {
        use chrono::Datelike;

        let month_start = today.with_day(1).unwrap_or(today);
        let week_start = today - chrono::Duration::days(6);
        let since = month_start.min(week_start).format("%Y-%m-%d").to_string();

        let conn = self
            .conn
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT day, model, input_tokens, cache_read_tokens, cache_write_tokens, estimated_cost_usd
                 FROM cost_entries
                 WHERE provider = ?1 AND day >= ?2",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query(params![provider.as_str(), since])
            .map_err(|e| e.to_string())?;

        // Claude and Codex rows store cache reads/writes beside the uncached
        // input (Codex since store migration v4); Antigravity counts cached
        // tokens inside input.
        let claude = provider == ProviderId::Claude;
        let cache_beside_input = provider != ProviderId::Antigravity;
        let per_m = |tokens: u64, rate: f64| tokens as f64 / 1_000_000.0 * rate;

        let mut month_cost = 0.0f64;
        let mut month_uncached = 0.0f64;
        let mut daily = [0u64; 7];
        let (mut hits, mut prompt_7d) = (0u64, 0u64);
        let (mut rated_tokens, mut w_in, mut w_read, mut w_write) = (0f64, 0f64, 0f64, 0f64);

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let day: String = row.get(0).map_err(|e| e.to_string())?;
            let model: String = row.get(1).map_err(|e| e.to_string())?;
            let input: u64 = row.get(2).map_err(|e| e.to_string())?;
            let read: u64 = row.get(3).map_err(|e| e.to_string())?;
            let write: u64 = row.get(4).map_err(|e| e.to_string())?;
            let cost: f64 = row.get(5).map_err(|e| e.to_string())?;
            let Ok(date) = chrono::NaiveDate::parse_from_str(&day, "%Y-%m-%d") else {
                continue;
            };
            let rates = crate::pricing::cache_rates(provider, &model);
            let prompt = if cache_beside_input {
                input.saturating_add(read).saturating_add(write)
            } else {
                input
            };

            if date >= month_start && date <= today {
                month_cost += cost;
                // Undo the cache: reads and writes billed at the plain input rate.
                let delta = rates.map_or(0.0, |(r_in, r_read, r_write)| {
                    per_m(read, r_in - r_read)
                        + if claude {
                            per_m(write, r_in - r_write)
                        } else {
                            0.0
                        }
                });
                month_uncached += cost + delta;
            }

            if date >= week_start && date <= today {
                let slot = (date - week_start).num_days() as usize;
                daily[slot] = daily[slot].saturating_add(prompt);
                hits = hits.saturating_add(read);
                prompt_7d = prompt_7d.saturating_add(prompt);
                if let Some((r_in, r_read, r_write)) = rates {
                    let t = prompt as f64;
                    rated_tokens += t;
                    w_in += t * r_in;
                    w_read += t * r_read;
                    w_write += t * r_write;
                }
            }
        }

        let rates_per_m = (rated_tokens > 0.0).then(|| {
            (
                w_in / rated_tokens,
                w_read / rated_tokens,
                w_write / rated_tokens,
            )
        });
        Ok(CacheInsights {
            provider,
            month_cost_usd: month_cost,
            month_uncached_cost_usd: month_uncached,
            hit_ratio_7d: (prompt_7d > 0).then(|| hits as f64 / prompt_7d as f64),
            break_even_hit_ratio: rates_per_m
                .map(|(i, r, w)| crate::pricing::cache_break_even_ratio(i, r, w)),
            daily_prompt_tokens_7d: daily.to_vec(),
            rates_per_m,
        })
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

    #[test]
    fn migration_v1_drops_legacy_model_keys_and_keeps_versioned_ones() {
        let store = temp_store("migration-v1");

        let entry = |model: &str| CostEntry {
            date: day_ago(1),
            provider: ProviderId::Claude,
            model: model.to_string(),
            input_tokens: 10,
            output_tokens: 5,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            estimated_cost_usd: 1.0,
        };

        store
            .save_cost_entries(&[
                entry("claude-opus"),             // legacy collapsed key
                entry("anthropic/claude-opus-5"), // legacy un-stripped vendor prefix
                entry("claude-opus-4-5"),         // current key — must survive
            ])
            .expect("seed rows");

        // Re-running migrations is a no-op once user_version is current, so
        // rewind it to simulate a store written by the previous release.
        {
            let conn = store.conn.lock().expect("lock");
            conn.execute_batch("PRAGMA user_version = 0")
                .expect("rewind version");
        }
        store.migrate().expect("migrate");

        let breakdown = store
            .get_model_breakdown(ProviderId::Claude, 30)
            .expect("load breakdown");
        let models: Vec<&str> = breakdown.iter().map(|b| b.model.as_str()).collect();
        assert_eq!(models, vec!["claude-opus-4-5"]);

        // And it must not re-run and delete anything on the next open.
        store.migrate().expect("migrate again");
        assert_eq!(
            store
                .get_model_breakdown(ProviderId::Claude, 30)
                .expect("reload")
                .len(),
            1
        );
    }

    #[test]
    fn migration_v2_drops_rows_for_retired_providers() {
        let store = temp_store("migration-v2");

        // `gemini` predates the Antigravity rename and is no longer a
        // ProviderId, so this row is unreachable through every read path.
        {
            let conn = store.conn.lock().expect("lock");
            conn.execute(
                "INSERT INTO cost_entries(day, provider, model, input_tokens, output_tokens,
                    cache_read_tokens, cache_write_tokens, estimated_cost_usd)
                 VALUES ('2026-09-04', 'gemini', 'gemini-2.5-pro', 10, 5, 0, 0, 1.0)",
                [],
            )
            .expect("seed retired-provider row");
            conn.execute_batch("PRAGMA user_version = 0")
                .expect("rewind version");
        }

        store.migrate().expect("migrate");

        let remaining: i64 = {
            let conn = store.conn.lock().expect("lock");
            conn.query_row(
                "SELECT COUNT(*) FROM cost_entries WHERE provider = 'gemini'",
                [],
                |row| row.get(0),
            )
            .expect("count")
        };
        assert_eq!(remaining, 0);
    }

    #[test]
    fn migration_v4_stops_double_counting_codex_cached_input() {
        let store = temp_store("migration-v4");
        let row = |provider: ProviderId| CostEntry {
            date: day_ago(1),
            provider,
            model: "gpt-5".to_string(),
            // Legacy Codex shape: input includes the 200 cached tokens.
            input_tokens: 1_000,
            output_tokens: 50,
            cache_read_tokens: 200,
            cache_write_tokens: 0,
            estimated_cost_usd: 1.0,
        };
        store
            .save_cost_entries(&[row(ProviderId::Codex), row(ProviderId::Claude)])
            .expect("seed rows");
        {
            let conn = store.conn.lock().expect("lock");
            conn.execute_batch("PRAGMA user_version = 3")
                .expect("rewind version");
        }
        store.migrate().expect("migrate");
        store.migrate().expect("migrate again is a no-op");

        let codex = store.get_cost_history(ProviderId::Codex, 30).unwrap();
        assert_eq!(codex[0].input_tokens, 800);
        assert_eq!(codex[0].cache_read_tokens, 200);
        // Claude rows were already stored uncached-only; leave them alone.
        let claude = store.get_cost_history(ProviderId::Claude, 30).unwrap();
        assert_eq!(claude[0].input_tokens, 1_000);
    }

    fn sample_entry(
        day: &str,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
        cost: f64,
    ) -> CostEntry {
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

    /// UTC calendar day `n` days before today, in the store's `day` format.
    fn day_ago(n: i64) -> String {
        (chrono::Utc::now().date_naive() - chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    }

    #[test]
    fn cost_history_keeps_every_model_row_in_window() {
        let store = temp_store("distinct-days");
        let entries = vec![
            sample_entry(&day_ago(1), "gpt-5", 10, 5, 1.0),
            sample_entry(&day_ago(1), "gpt-5-mini", 4, 2, 0.2),
            sample_entry(&day_ago(0), "gpt-5", 8, 4, 0.8),
        ];
        store
            .save_cost_entries(&entries)
            .expect("save cost entries");

        let history = store
            .get_cost_history(ProviderId::Codex, 2)
            .expect("load cost history");
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn cost_history_uses_calendar_days_not_active_days() {
        let store = temp_store("calendar-window");
        // Only two active days, but one is outside a 7-day window. The old
        // "last N distinct days" query returned both.
        let entries = vec![
            sample_entry(&day_ago(20), "gpt-5", 100, 50, 10.0),
            sample_entry(&day_ago(6), "gpt-5", 10, 5, 1.0),
            sample_entry(&day_ago(7), "gpt-5", 1, 1, 0.1),
        ];
        store
            .save_cost_entries(&entries)
            .expect("save cost entries");

        let history = store
            .get_cost_history(ProviderId::Codex, 7)
            .expect("load cost history");
        let days: Vec<String> = history.into_iter().map(|e| e.date).collect();
        assert_eq!(days, vec![day_ago(6)]);
    }

    #[test]
    fn model_breakdown_aggregates_rows_per_model() {
        let store = temp_store("breakdown");
        let entries = vec![
            sample_entry(&day_ago(1), "gpt-5", 10, 5, 1.0),
            sample_entry(&day_ago(0), "gpt-5", 12, 6, 1.2),
            sample_entry(&day_ago(0), "gpt-5-mini", 4, 2, 0.2),
        ];
        store
            .save_cost_entries(&entries)
            .expect("save cost entries");

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
            sample_entry(&day_ago(1), "gpt-5", 10, 5, 1.0),
            sample_entry(&day_ago(1), "gpt-5-mini", 4, 2, 0.2),
        ];
        store
            .save_cost_entries(&entries)
            .expect("save cost entries");

        let trend = store
            .get_usage_trends(ProviderId::Codex, 30)
            .expect("load usage trends");
        assert_eq!(trend.points.len(), 1);
        assert_eq!(trend.points[0].date, day_ago(1));
        assert_eq!(trend.points[0].total_tokens, 21);
        assert!((trend.points[0].cost_usd - 1.2).abs() < f64::EPSILON);
    }

    fn history_snapshot(secs_ago: i64, utilization: f64) -> UsageSnapshot {
        use crate::usage::models::{
            DataProvenance, DataSource, UsageUnit, UsageWindow, WindowAccuracy,
        };
        UsageSnapshot {
            provider: ProviderId::Claude,
            windows: vec![UsageWindow {
                window_type: WindowType::FiveHour,
                utilization,
                used: None,
                limit: None,
                remaining: None,
                resets_at: None,
                reset_countdown_secs: None,
                unit: UsageUnit::Percent,
                accuracy: WindowAccuracy::PercentOnly,
                note: None,
            }],
            credits: None,
            plan: None,
            fetched_at: chrono::Utc::now() - chrono::Duration::seconds(secs_ago),
            source: DataSource::Oauth,
            provenance: DataProvenance::Official,
            stale: false,
        }
    }

    #[test]
    fn snapshot_history_round_trips_and_ignores_duplicate_ts() {
        let store = temp_store("history-roundtrip");
        let older = history_snapshot(120, 10.0);
        let mut duplicate = older.clone();
        duplicate.windows[0].utilization = 99.0;
        store.append_snapshot_history(&older).unwrap();
        store.append_snapshot_history(&duplicate).unwrap();
        store
            .append_snapshot_history(&history_snapshot(0, 20.0))
            .unwrap();

        let rows = store
            .get_snapshot_history(ProviderId::Claude, WindowType::FiveHour, 0)
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].utilization, 10.0);
        assert_eq!(rows[1].utilization, 20.0);
        assert!(store
            .get_snapshot_history(ProviderId::Claude, WindowType::SevenDay, 0)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn snapshot_history_prunes_rows_older_than_30_days() {
        let store = temp_store("history-prune");
        store
            .append_snapshot_history(&history_snapshot(40 * 24 * 3600, 5.0))
            .unwrap();
        store
            .append_snapshot_history(&history_snapshot(0, 7.0))
            .unwrap();
        let rows = store
            .get_snapshot_history(ProviderId::Claude, WindowType::FiveHour, 0)
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].utilization, 7.0);
    }

    #[test]
    fn reopening_purges_legacy_rows_written_after_migration() {
        let path = std::env::temp_dir().join(format!(
            "otm-store-purge-{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let store = UsageStore::open(&path).unwrap();
        let entry = |provider: ProviderId, model: &str| CostEntry {
            date: day_ago(1),
            provider,
            model: model.to_string(),
            input_tokens: 10,
            output_tokens: 5,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            estimated_cost_usd: 1.0,
        };
        // Simulate an older build writing a collapsed key after migrations ran.
        store
            .save_cost_entries(&[
                entry(ProviderId::Claude, "claude-opus"),
                entry(ProviderId::Claude, "claude-opus-5"),
            ])
            .unwrap();
        {
            let conn = store.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO cost_entries VALUES ('2026-09-04','gemini','g',1,1,0,0,1.0)",
                [],
            )
            .unwrap();
        }
        drop(store);

        let store = UsageStore::open(&path).unwrap();
        let models: Vec<String> = store
            .get_cost_history(ProviderId::Claude, 30)
            .unwrap()
            .into_iter()
            .map(|e| e.model)
            .collect();
        assert_eq!(models, vec!["claude-opus-5".to_string()]);
        let gemini: i64 = store
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM cost_entries WHERE provider = 'gemini'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(gemini, 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cache_insights_split_month_and_week() {
        let store = temp_store("cache-insights");
        let row = |day: &str, input, read, write, cost| CostEntry {
            date: day.to_string(),
            provider: ProviderId::Claude,
            model: "claude-haiku-4-5".to_string(),
            input_tokens: input,
            output_tokens: 0,
            cache_read_tokens: read,
            cache_write_tokens: write,
            estimated_cost_usd: cost,
        };
        store
            .save_cost_entries(&[
                // Last month, inside the 7-day window only.
                row("2026-09-30", 0, 1_000_000, 0, 0.10),
                // This month.
                row("2026-10-02", 1_000_000, 3_000_000, 0, 1.30),
                // Older than both windows.
                row("2026-08-01", 5_000_000, 0, 0, 5.00),
            ])
            .unwrap();

        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 3).unwrap();
        let ci = store.get_cache_insights(ProviderId::Claude, today).unwrap();

        // Month: $1.30 with caching; 3M reads at full input adds 3 x $0.90.
        assert!((ci.month_cost_usd - 1.30).abs() < 1e-9);
        assert!((ci.month_uncached_cost_usd - 4.00).abs() < 1e-9);
        // Week: 4M reads over 5M prompt tokens.
        assert!((ci.hit_ratio_7d.unwrap() - 0.8).abs() < 1e-9);
        assert_eq!(
            ci.daily_prompt_tokens_7d,
            vec![0, 0, 0, 1_000_000, 0, 4_000_000, 0]
        );
        assert!((ci.break_even_hit_ratio.unwrap() - 0.25 / 1.15).abs() < 1e-9);
    }
}
