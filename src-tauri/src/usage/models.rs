use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Claude,
    Codex,
    Gemini,
}

impl ProviderId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
        }
    }

    pub fn all() -> [ProviderId; 3] {
        [ProviderId::Claude, ProviderId::Codex, ProviderId::Gemini]
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowType {
    FiveHour,
    SevenDay,
    Daily,
    Monthly,
    Session,
    Weekly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    Oauth,
    Cookie,
    Cli,
    LocalLog,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DataProvenance {
    Official,
    Internal,
    #[default]
    DerivedLocal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UsageUnit {
    #[default]
    Tokens,
    Requests,
    Percent,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WindowAccuracy {
    #[default]
    Exact,
    Approximate,
    PercentOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageWindow {
    pub window_type: WindowType,
    pub utilization: f64,
    pub used: Option<u64>,
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
    pub resets_at: Option<DateTime<Utc>>,
    pub reset_countdown_secs: Option<i64>,
    #[serde(default)]
    pub unit: UsageUnit,
    #[serde(default)]
    pub accuracy: WindowAccuracy,
    #[serde(default)]
    pub note: Option<String>,
}

impl UsageWindow {
    pub fn exact(
        window_type: WindowType,
        used: u64,
        limit: u64,
        resets_at: Option<DateTime<Utc>>,
        unit: UsageUnit,
    ) -> Self {
        Self::build(
            window_type,
            used,
            limit,
            resets_at,
            unit,
            WindowAccuracy::Exact,
            None,
        )
    }

    pub fn approximate(
        window_type: WindowType,
        used: u64,
        limit: u64,
        resets_at: Option<DateTime<Utc>>,
        unit: UsageUnit,
        note: impl Into<String>,
    ) -> Self {
        Self::build(
            window_type,
            used,
            limit,
            resets_at,
            unit,
            WindowAccuracy::Approximate,
            Some(note.into()),
        )
    }

    pub fn percent(
        window_type: WindowType,
        utilization: f64,
        resets_at: Option<DateTime<Utc>>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            window_type,
            utilization: utilization.clamp(0.0, 100.0),
            used: None,
            limit: None,
            remaining: None,
            resets_at,
            reset_countdown_secs: resets_at.map(|r| (r - Utc::now()).num_seconds()),
            unit: UsageUnit::Percent,
            accuracy: WindowAccuracy::PercentOnly,
            note: Some(note.into()),
        }
    }

    fn build(
        window_type: WindowType,
        used: u64,
        limit: u64,
        resets_at: Option<DateTime<Utc>>,
        unit: UsageUnit,
        accuracy: WindowAccuracy,
        note: Option<String>,
    ) -> Self {
        let utilization = if limit == 0 {
            0.0
        } else {
            (used as f64 * 100.0) / limit as f64
        };
        let remaining = Some(limit.saturating_sub(used));
        let reset_countdown_secs = resets_at.map(|r| (r - Utc::now()).num_seconds());
        Self {
            window_type,
            utilization,
            used: Some(used),
            limit: Some(limit),
            remaining,
            resets_at,
            reset_countdown_secs,
            unit,
            accuracy,
            note,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreditsInfo {
    pub balance_usd: Option<f64>,
    pub spent_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanInfo {
    pub tier: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSnapshot {
    pub provider: ProviderId,
    pub windows: Vec<UsageWindow>,
    pub credits: Option<CreditsInfo>,
    pub plan: Option<PlanInfo>,
    pub fetched_at: DateTime<Utc>,
    pub source: DataSource,
    #[serde(default)]
    pub provenance: DataProvenance,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub date: String,
    pub provider: ProviderId,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    pub date: String,
    pub cost_usd: f64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendData {
    pub provider: ProviderId,
    pub days: u32,
    pub points: Vec<TrendPoint>,
    pub total_cost_usd: f64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBreakdownEntry {
    pub provider: ProviderId,
    pub model: String,
    pub days: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentActivityEntry {
    pub provider: ProviderId,
    pub prompt: String,
    pub response: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub session_id: Option<String>,
    pub terminal_label: Option<String>,
    pub cwd: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Warning,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAlert {
    pub provider: ProviderId,
    pub window_type: WindowType,
    pub utilization: f64,
    pub threshold_percent: u8,
    pub severity: AlertSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    pub generated_at: DateTime<Utc>,
    pub snapshots: Vec<UsageSnapshot>,
    pub alerts: Vec<UsageAlert>,
    pub model_breakdowns: Vec<ModelBreakdownEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderHealth {
    Active,
    Waiting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider: ProviderId,
    pub health: ProviderHealth,
    pub message: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefreshCadence {
    Manual,
    Every30s,
    Every1m,
    Every2m,
    Every5m,
    Every15m,
}

impl RefreshCadence {
    pub fn seconds(self) -> Option<u64> {
        match self {
            Self::Manual => None,
            Self::Every30s => Some(30),
            Self::Every1m => Some(60),
            Self::Every2m => Some(120),
            Self::Every5m => Some(300),
            Self::Every15m => Some(900),
        }
    }
}
