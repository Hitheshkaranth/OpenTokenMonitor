//! Usage-report export rendering and on-disk writing.
//!
//! The backend already knows how to *build* a [`UsageReport`] (see
//! `commands::export_usage_report`). This module takes that report and renders
//! it to a file the user can open or print:
//!
//! - [`ExportFormat::Csv`]  -> [`render_csv`]
//! - [`ExportFormat::Json`] -> [`render_json`]
//! - [`ExportFormat::Pdf`]  -> [`render_html`]
//!
//! "Pdf" renders HTML on purpose: true PDF generation needs a heavy
//! dependency, whereas the browser's "Print to PDF" affordance already gives
//! the user an openable, printable PDF from HTML. That keeps this an MVP while
//! still fulfilling the "save as PDF" goal.
//!
//! The rendering functions are pure (report in, string out) so they are easy
//! to unit-test without a running Tauri runtime.

use std::collections::HashSet;
use std::fs;

use chrono::Utc;
use serde::Deserialize;
use tauri::{AppHandle, Manager, State};

use crate::alerts::build_alerts;
use crate::usage::models::{ProviderId, UsageReport};

/// Output format selected by the frontend before invoking the export command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Json,
    /// Renders to HTML; the user prints/saves it as a PDF via the browser.
    Pdf,
}

impl ExportFormat {
    /// The file extension used for this format on disk.
    fn extension(self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            // "Pdf" is authored as HTML so it opens directly in a browser and
            // can be "Print to PDF" from there.
            ExportFormat::Pdf => "html",
        }
    }
}

/// Fixed CSV schema for one model breakdown entry per row.
const CSV_HEADER: &str = "provider,model,days,token_type,input_tokens,output_tokens,cache_read_tokens,cache_write_tokens,estimated_cost_usd";

/// Render a usage report as CSV.
///
/// One data row per [`ModelBreakdownEntry`]; `token_type` is `combined`
/// because each entry already carries the four token columns inline. A
/// `#`-prefixed summary section follows the data rows (report window, totals
/// per token type, cost, counts).
pub fn render_csv(report: &UsageReport, days: u32) -> String {
    let mut out = String::new();
    out.push_str(CSV_HEADER);
    out.push('\n');

    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_read: u64 = 0;
    let mut total_cache_write: u64 = 0;
    let mut total_cost: f64 = 0.0;
    let mut providers = HashSet::new();

    for e in &report.model_breakdowns {
        total_input += e.input_tokens;
        total_output += e.output_tokens;
        total_cache_read += e.cache_read_tokens;
        total_cache_write += e.cache_write_tokens;
        total_cost += e.estimated_cost_usd;
        providers.insert(e.provider.as_str());

        let row = [
            csv_field(e.provider.as_str()),
            csv_field(&e.model),
            csv_field(&e.days.to_string()),
            csv_field("combined"),
            csv_field(&e.input_tokens.to_string()),
            csv_field(&e.output_tokens.to_string()),
            csv_field(&e.cache_read_tokens.to_string()),
            csv_field(&e.cache_write_tokens.to_string()),
            csv_field(&fmt_cost(e.estimated_cost_usd)),
        ]
        .join(",");
        out.push_str(&row);
        out.push('\n');
    }

    let total_tokens = total_input + total_output + total_cache_read + total_cache_write;

    out.push('\n');
    out.push_str("# SUMMARY\n");
    out.push_str(&format!("# report_days,{}\n", days.max(1)));
    out.push_str(&format!("# generated_at,{}\n", report.generated_at.to_rfc3339()));
    out.push_str(&format!("# providers,{}\n", providers.len()));
    out.push_str(&format!("# model_entries,{}\n", report.model_breakdowns.len()));
    out.push_str(&format!("# alerts,{}\n", report.alerts.len()));
    out.push_str(&format!("# tokens.input,{}\n", total_input));
    out.push_str(&format!("# tokens.output,{}\n", total_output));
    out.push_str(&format!("# tokens.cache_read,{}\n", total_cache_read));
    out.push_str(&format!("# tokens.cache_write,{}\n", total_cache_write));
    out.push_str(&format!("# tokens.total,{}\n", total_tokens));
    out.push_str(&format!("# cost_usd,{}\n", fmt_cost(total_cost)));

    out
}

/// Render a usage report as pretty-printed JSON.
pub fn render_json(report: &UsageReport) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
}

/// Render a usage report as a minimal, self-contained HTML document.
pub fn render_html(report: &UsageReport) -> String {
    let generated_at = report.generated_at.to_rfc3339();
    let report_days = report.model_breakdowns.iter().map(|e| e.days).max().unwrap_or(0).max(1);

    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_read: u64 = 0;
    let mut total_cache_write: u64 = 0;
    let mut total_cost: f64 = 0.0;
    let mut providers = HashSet::new();
    for e in &report.model_breakdowns {
        total_input += e.input_tokens;
        total_output += e.output_tokens;
        total_cache_read += e.cache_read_tokens;
        total_cache_write += e.cache_write_tokens;
        total_cost += e.estimated_cost_usd;
        providers.insert(e.provider.as_str());
    }
    let total_tokens = total_input + total_output + total_cache_read + total_cache_write;

    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n");
    out.push_str("<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str("<title>OpenTokenMonitor Usage Report</title>\n");
    out.push_str("<style>\n");
    out.push_str("body{font-family:-apple-system,Segoe UI,Roboto,Helvetica,Arial,sans-serif;margin:2rem;color:#1a1a1a;background:#fafafa;}\n");
    out.push_str("h1{font-size:1.5rem;margin-bottom:0;}\n");
    out.push_str(".meta{color:#666;margin:.25rem 0 1.5rem;}\n");
    out.push_str(".summary{display:flex;gap:.75rem;flex-wrap:wrap;margin-bottom:1.75rem;}\n");
    out.push_str(".card{background:#fff;border:1px solid #e2e2e2;border-radius:8px;padding:.75rem 1rem;min-width:110px;}\n");
    out.push_str(".card .n{font-size:1.25rem;font-weight:600;}\n");
    out.push_str(".card .l{color:#666;font-size:.8rem;text-transform:uppercase;letter-spacing:.03em;}\n");
    out.push_str("table{border-collapse:collapse;width:100%;margin-bottom:1.75rem;background:#fff;}\n");
    out.push_str("th,td{border:1px solid #e2e2e2;padding:.5rem .6rem;text-align:left;font-size:.9rem;}\n");
    out.push_str("th{background:#f0f0f0;}\n");
    out.push_str("tr:nth-child(even){background:#fafafa;}\n");
    out.push_str(".sev-warning{color:#9a7300;font-weight:600;}\n");
    out.push_str(".sev-high{color:#c25b09;font-weight:600;}\n");
    out.push_str(".sev-critical{color:#c0392b;font-weight:700;}\n");
    out.push_str(".foot{color:#888;font-size:.8rem;margin-top:2rem;}\n");
    out.push_str("</style>\n</head>\n<body>\n");

    out.push_str("<h1>OpenTokenMonitor Usage Report</h1>\n");
    out.push_str(&format!(
        "<p class=\"meta\">Generated {generated_at} over the last {report_days} day(s).</p>\n"
    ));

    out.push_str("<div class=\"summary\">\n");
    card(&mut out, &format!("${:.4}", total_cost), "Cost (USD)");
    card(&mut out, &total_tokens.to_string(), "Total tokens");
    card(&mut out, &providers.len().to_string(), "Providers");
    card(&mut out, &report.model_breakdowns.len().to_string(), "Models");
    card(&mut out, &report.alerts.len().to_string(), "Alerts");
    out.push_str("</div>\n");

    out.push_str("<h2>Token usage by model</h2>\n");
    out.push_str("<table>\n<thead>\n<tr>\n");
    for h in [
        "Provider", "Model", "Days", "Input", "Output", "Cache read", "Cache write", "Total", "Cost (USD)",
    ] {
        out.push_str(&format!("<th>{}</th>\n", esc(h)));
    }
    out.push_str("</tr>\n</thead>\n<tbody>\n");
    if report.model_breakdowns.is_empty() {
        out.push_str("<tr><td colspan=\"9\" style=\"text-align:center;color:#888\">No usage data for this period.</td></tr>\n");
    }
    for e in &report.model_breakdowns {
        let total = e.input_tokens + e.output_tokens + e.cache_read_tokens + e.cache_write_tokens;
        out.push_str("<tr>\n");
        for cell in [
            esc(e.provider.as_str()),
            esc(&e.model),
            e.days.to_string(),
            e.input_tokens.to_string(),
            e.output_tokens.to_string(),
            e.cache_read_tokens.to_string(),
            e.cache_write_tokens.to_string(),
            total.to_string(),
            fmt_cost(e.estimated_cost_usd),
        ] {
            out.push_str(&format!("<td>{}</td>\n", cell));
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</tbody>\n</table>\n");

    if !report.alerts.is_empty() {
        out.push_str("<h2>Usage alerts</h2>\n");
        out.push_str("<table>\n<thead>\n<tr>\n");
        for h in ["Provider", "Window", "Utilization", "Threshold", "Severity", "Message"] {
            out.push_str(&format!("<th>{}</th>\n", esc(h)));
        }
        out.push_str("</tr>\n</thead>\n<tbody>\n");
        for a in &report.alerts {
            let (label, cls) = match a.severity {
                crate::usage::models::AlertSeverity::Warning => ("Warning", "sev-warning"),
                crate::usage::models::AlertSeverity::High => ("High", "sev-high"),
                crate::usage::models::AlertSeverity::Critical => ("Critical", "sev-critical"),
            };
            out.push_str("<tr>\n");
            for cell in [
                esc(a.provider.as_str()),
                esc(window_label(a.window_type)),
                format!("{:.0}%", a.utilization),
                format!("{}%", a.threshold_percent),
                format!("<span class=\"{}\">{}</span>", cls, esc(label)),
                esc(&a.message),
            ] {
                out.push_str(&format!("<td>{}</td>\n", cell));
            }
            out.push_str("</tr>\n");
        }
        out.push_str("</tbody>\n</table>\n");
    }

    out.push_str("<p class=\"foot\">Open this file in a browser and use Print → Save as PDF to export it.</p>\n");
    out.push_str("</body>\n</html>\n");

    out
}

fn card(out: &mut String, number: &str, label: &str) {
    out.push_str("<div class=\"card\">\n");
    out.push_str(&format!("<div class=\"n\">{}</div>\n", esc(number)));
    out.push_str(&format!("<div class=\"l\">{}</div>\n", esc(label)));
    out.push_str("</div>\n");
}

/// Build the export command.
///
/// Reuses the same report construction as `commands::export_usage_report`,
/// renders it in the requested format, and writes it under
/// `<app_data_dir>/exports/`. Returns the absolute path to the written file.
#[tauri::command]
pub async fn export_report(
    app: AppHandle,
    days: u32,
    format: ExportFormat,
) -> Result<String, String> {
    let state: State<'_, crate::AppState> = app.state();

    let snapshots = state.store.get_all_snapshots()?;
    let mut model_breakdowns = Vec::new();
    for provider in ProviderId::all() {
        model_breakdowns.extend(state.store.get_model_breakdown(provider, days.max(1))?);
    }

    let report = UsageReport {
        generated_at: Utc::now(),
        alerts: build_alerts(&snapshots),
        snapshots,
        model_breakdowns,
    };

    let ext = format.extension();
    let content: String = match format {
        ExportFormat::Csv => render_csv(&report, days),
        ExportFormat::Json => render_json(&report),
        ExportFormat::Pdf => render_html(&report),
    };

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    let exports_dir = data_dir.join("exports");
    fs::create_dir_all(&exports_dir)
        .map_err(|e| format!("failed to create exports dir: {e}"))?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    // Provider-agnostic base name: the report aggregates all providers, so it
    // is not tied to any single one.
    let filename = format!("usage-report-{timestamp}.{ext}");
    let file_path = exports_dir.join(&filename);

    fs::write(&file_path, content)
        .map_err(|e| format!("failed to write report file: {e}"))?;

    Ok(file_path.to_string_lossy().to_string())
}

fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        let mut quoted = String::with_capacity(value.len() + 2);
        quoted.push('"');
        for c in value.chars() {
            if c == '"' {
                quoted.push_str("\"\"");
            } else {
                quoted.push(c);
            }
        }
        quoted.push('"');
        quoted
    } else {
        value.to_string()
    }
}

/// Format a cost without scientific notation and with a stable precision.
fn fmt_cost(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    format!("{:.6}", value)
}

/// Escape a string for safe inclusion in HTML text/attributes.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Human-friendly window-type label for the HTML report.
fn window_label(window_type: crate::usage::models::WindowType) -> &'static str {
    match window_type {
        crate::usage::models::WindowType::FiveHour => "5h window",
        crate::usage::models::WindowType::SevenDay => "7d window",
        crate::usage::models::WindowType::Daily => "daily window",
        crate::usage::models::WindowType::Monthly => "monthly window",
        crate::usage::models::WindowType::Session => "session window",
        crate::usage::models::WindowType::Weekly => "weekly window",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::models::ModelBreakdownEntry;

    fn sample_report() -> UsageReport {
        UsageReport {
            generated_at: Utc::now(),
            alerts: vec![],
            snapshots: vec![],
            model_breakdowns: vec![
                ModelBreakdownEntry {
                    provider: ProviderId::Claude,
                    model: "gpt-4".into(),
                    days: 7,
                    input_tokens: 1000,
                    output_tokens: 500,
                    cache_read_tokens: 200,
                    cache_write_tokens: 100,
                    total_tokens: 1800,
                    estimated_cost_usd: 1.2345,
                    cache_savings_usd: 0.0,
                },
                ModelBreakdownEntry {
                    provider: ProviderId::Codex,
                    model: "o1, preview".into(),
                    days: 30,
                    input_tokens: 10,
                    output_tokens: 20,
                    cache_read_tokens: 0,
                    cache_write_tokens: 5,
                    total_tokens: 35,
                    estimated_cost_usd: 0.0,
                    cache_savings_usd: 0.0,
                },
            ],
        }
    }

    #[test]
    fn csv_header_and_one_row_per_entry() {
        let report = sample_report();
        let csv = render_csv(&report, 7);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], CSV_HEADER);
        let data_rows = lines
            .iter()
            .filter(|l| l.starts_with("claude") || l.starts_with("codex"))
            .count();
        assert_eq!(data_rows, report.model_breakdowns.len());
    }

    #[test]
    fn csv_quotes_fields_containing_commas() {
        let csv = render_csv(&sample_report(), 7);
        assert!(csv.contains("\"o1, preview\""));
    }

    #[test]
    fn csv_carries_the_token_columns() {
        let csv = render_csv(&sample_report(), 7);
        // Four consecutive token columns for the first entry.
        assert!(csv.contains("1000,500,200,100"));
        // Summary totals are present.
        assert!(csv.contains("# tokens.total,1835"));
        assert!(csv.contains("# cost_usd,"));
    }

    #[test]
    fn json_is_valid_and_pretty() {
        let report = sample_report();
        let json = render_json(&report);
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("render_json must produce valid JSON");
        // serde_json renders the Utc instant with a `Z` suffix, so normalize the
        // offset before comparing against the rfc3339 form.
        let generated_at =
            value["generated_at"].as_str().expect("generated_at is a string");
        assert_eq!(
            generated_at.replace("Z", "+00:00"),
            report.generated_at.to_rfc3339()
        );
        assert!(json.contains('\n') && json.contains("  "));
    }

    #[test]
    fn html_contains_report_content() {
        let html = render_html(&sample_report());
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("gpt-4"));
        assert!(html.contains("o1"));
        assert!(html.contains("Open this file in a browser"));
    }

    #[test]
    fn html_escapes_html_special_chars() {
        let mut report = sample_report();
        report.model_breakdowns[1].model = "evil<b>x\"y\"".into();
        let html = render_html(&report);
        assert!(html.contains("evil&lt;b&gt;"));
        assert!(!html.contains("evil<b>"));
    }
}