//! Budget configuration and spend forecasting.
//!
//! Budgets are persisted alongside the other secrets in the shared
//! `secrets.json` store, encoded as a JSON array under the `"budgets"` key.
//! [`compute_forecast`] projects future spend from the historical cost
//! history so the UI can warn before a budget is blown.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::usage::models::ProviderId;
use crate::usage::store::UsageStore;

use crate::AppState;

const BUDGETS_STORE_KEY: &str = "budgets";

/// A per-provider spend budget over a rolling window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    pub provider: ProviderId,
    pub amount_usd: f64,
    pub period_days: u32,
}

/// The result of projecting spend against an optional budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastResult {
    pub provider: ProviderId,
    pub period_days: u32,
    pub spend_to_date_usd: f64,
    pub budget_usd: Option<f64>,
    pub remaining_usd: Option<f64>,
    pub daily_average_usd: f64,
    pub projected_spend_usd: f64,
    pub projected_exceeds_budget: bool,
    pub utilization_percent: f64,
}

fn budgets_store_path(app: &AppHandle) -> PathBuf {
    let mut path = crate::secrets_store_path(app);
    path.set_file_name("budgets.json");
    path
}

pub(crate) fn load_budgets(app: &AppHandle) -> Vec<BudgetConfig> {
    let Ok(store) = app.store(budgets_store_path(app)) else {
        return Vec::new();
    };
    match store
        .get(BUDGETS_STORE_KEY)
        .and_then(|v| v.as_str().map(str::to_string))
    {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => Vec::new(),
    }
}

fn save_budgets(app: &AppHandle, budgets: &[BudgetConfig]) -> Result<(), String> {
    let store = app
        .store(budgets_store_path(app))
        .map_err(|e| e.to_string())?;
    let json = serde_json::to_string(budgets).map_err(|e| e.to_string())?;
    store.set(BUDGETS_STORE_KEY, serde_json::Value::String(json));
    store.save().map_err(|e| e.to_string())
}

fn is_valid_provider(provider: ProviderId) -> bool {
    ProviderId::all().contains(&provider)
}

fn upsert_budget(
    budgets: Vec<BudgetConfig>,
    provider: ProviderId,
    amount_usd: f64,
    period_days: u32,
) -> Vec<BudgetConfig> {
    let mut budgets = budgets;
    if let Some(existing) = budgets.iter_mut().find(|b| b.provider == provider) {
        existing.amount_usd = amount_usd;
        existing.period_days = period_days;
    } else {
        budgets.push(BudgetConfig {
            provider,
            amount_usd,
            period_days,
        });
    }
    budgets
}

/// Project spend for `provider` over `days`, optionally against `budget`.
pub fn compute_forecast(
    store: &UsageStore,
    budget: Option<&BudgetConfig>,
    provider: ProviderId,
    days: u32,
) -> ForecastResult {
    let effective_days = days.max(1);

    let spend_to_date_usd = store
        .get_cost_history(provider, effective_days)
        .map(|entries| entries.iter().map(|e| e.estimated_cost_usd).sum())
        .unwrap_or(0.0);

    let budget_usd = budget.map(|b| b.amount_usd);
    let remaining_usd = budget_usd.map(|b| {
        if b > spend_to_date_usd {
            b - spend_to_date_usd
        } else {
            0.0
        }
    });

    let daily_average_usd = spend_to_date_usd / effective_days as f64;

    let forecast_window = budget
        .map(|b| b.period_days.max(1))
        .unwrap_or_else(|| effective_days.max(1));
    let projected_spend_usd = daily_average_usd * forecast_window as f64;

    let projected_exceeds_budget = budget_usd.map(|b| projected_spend_usd > b).unwrap_or(false);

    let utilization_percent = match budget_usd {
        Some(b) if b > 0.0 => (spend_to_date_usd / b) * 100.0,
        _ => 0.0,
    };

    ForecastResult {
        provider,
        period_days: effective_days,
        spend_to_date_usd,
        budget_usd,
        remaining_usd,
        daily_average_usd,
        projected_spend_usd,
        projected_exceeds_budget,
        utilization_percent,
    }
}

#[tauri::command]
pub async fn get_budgets(
    app: AppHandle,
    #[allow(unused_variables)] state: State<'_, AppState>,
) -> Result<Vec<BudgetConfig>, String> {
    Ok(load_budgets(&app))
}

#[tauri::command]
pub async fn set_budget(
    app: AppHandle,
    #[allow(unused_variables)] state: State<'_, AppState>,
    provider: ProviderId,
    amount_usd: f64,
    period_days: u32,
) -> Result<(), String> {
    if !is_valid_provider(provider) {
        return Err(format!("invalid provider: {}", provider.as_str()));
    }
    let budgets = upsert_budget(load_budgets(&app), provider, amount_usd, period_days);
    save_budgets(&app, &budgets)
}

#[tauri::command]
pub async fn get_forecast(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: ProviderId,
    days: u32,
) -> Result<ForecastResult, String> {
    if !is_valid_provider(provider) {
        return Err(format!("invalid provider: {}", provider.as_str()));
    }
    let budget = load_budgets(&app)
        .into_iter()
        .find(|b| b.provider == provider);
    Ok(compute_forecast(
        &state.store,
        budget.as_ref(),
        provider,
        days,
    ))
}
