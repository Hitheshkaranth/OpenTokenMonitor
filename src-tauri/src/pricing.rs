//! Per-model cost tables and cost computations.
//!
//! All rates are USD per **1 million tokens**, taken from each provider's
//! public pricing page. Edit this file in one place when prices change —
//! the scanners delegate every cost calculation here.
//!
//! Pricing source-of-truth links (verify before bumping rates):
//! - Anthropic:  <https://www.anthropic.com/pricing>
//! - OpenAI:     <https://openai.com/api/pricing/>
//! - Google AI:  <https://ai.google.dev/pricing>
//!
//! Rates last reviewed: **2026-04** (Q1 2026 published list prices). Prices
//! reflect the standard / ≤128K-context tier; long-context surcharges are not
//! tracked because we don't capture context size from local logs.
//!
//! ### Conventions
//!
//! - Claude rate tuple: `(input, cache_read, cache_write, output)`
//! - Codex rate tuple:  `(input, cached_input, output)`
//! - Gemini rate tuple: `(input, output)` — caching surfaced separately when
//!   needed; today we don't subtract cached portions for Gemini because the
//!   local session files don't always tag cached tokens reliably.
//!
//! ### Adding a new model
//!
//! 1. Add a row to the matching table (Claude / Codex / Gemini).
//! 2. Add a normalization branch to the matching `normalize_*_model` function
//!    in `usage_scanners.rs` so log entries map onto the table key.
//! 3. Update the model picker in any UI that shows per-model breakdowns.

// ─────────────────────────────────────────────────────────────────────────────
// Anthropic — Claude
// ─────────────────────────────────────────────────────────────────────────────

/// Per-1M-token rates for Claude models, in USD.
///
/// Returned tuple: `(input, cache_read, cache_write, output)`.
///
/// Tiers (Q1 2026):
/// - Opus 4 / 4.5 / 4.7  →  $15 / $1.50 / $18.75 / $75
/// - Sonnet 4 / 4.5 / 4.6 →  $3  / $0.30 / $3.75  / $15
/// - Haiku 4.5            →  $1  / $0.10 / $1.25  / $5
///
/// Falls back to Sonnet rates for unrecognized "claude-*" entries so cost
/// estimates degrade conservatively rather than silently dropping to zero.
pub fn claude_rates(model: &str) -> Option<(f64, f64, f64, f64)> {
    let m = model.to_ascii_lowercase();

    if m.contains("opus") {
        return Some((15.00, 1.50, 18.75, 75.00));
    }
    if m.contains("sonnet") {
        return Some((3.00, 0.30, 3.75, 15.00));
    }
    if m.contains("haiku") {
        // Haiku 4.5 (current). Earlier Haiku 3 / 3.5 used $0.25 / $1.25 — if
        // we ever need legacy support, branch on the version digit here.
        return Some((1.00, 0.10, 1.25, 5.00));
    }
    None
}

/// Compute Claude usage cost in USD given the four token streams.
pub fn claude_cost_usd(
    model: &str,
    input: u64,
    cache_read: u64,
    cache_create: u64,
    output: u64,
) -> f64 {
    let Some((in_per_m, read_per_m, create_per_m, out_per_m)) = claude_rates(model) else {
        return 0.0;
    };
    per_million(input, in_per_m)
        + per_million(cache_read, read_per_m)
        + per_million(cache_create, create_per_m)
        + per_million(output, out_per_m)
}

// ─────────────────────────────────────────────────────────────────────────────
// OpenAI — Codex / GPT-5 / GPT-4.1 / o-series
// ─────────────────────────────────────────────────────────────────────────────

/// Per-1M-token rates for OpenAI models, in USD.
///
/// Returned tuple: `(input, cached_input, output)`.
///
/// Tiers (Q1 2026, OpenAI public list):
/// - gpt-5         →  $1.25 / $0.125 / $10.00
/// - gpt-5-mini    →  $0.25 / $0.025 / $2.00
/// - gpt-5-nano    →  $0.05 / $0.005 / $0.40
/// - gpt-4.1       →  $2.00 / $0.50  / $8.00
/// - gpt-4.1-mini  →  $0.40 / $0.10  / $1.60
/// - gpt-4.1-nano  →  $0.10 / $0.025 / $0.40
/// - o3            →  $2.00 / $0.50  / $8.00
/// - o3-mini       →  $1.10 / $0.55  / $4.40
/// - o4-mini       →  $1.10 / $0.275 / $4.40
pub fn codex_rates(model: &str) -> Option<(f64, f64, f64)> {
    let m = model.to_ascii_lowercase();

    // GPT-5 family — match the most specific suffix first.
    if m.contains("gpt-5") && m.contains("nano") {
        return Some((0.05, 0.005, 0.40));
    }
    if m.contains("gpt-5") && m.contains("mini") {
        return Some((0.25, 0.025, 2.00));
    }
    if m.contains("gpt-5") {
        return Some((1.25, 0.125, 10.00));
    }

    // GPT-4.1 family.
    if m.contains("gpt-4.1") && m.contains("nano") {
        return Some((0.10, 0.025, 0.40));
    }
    if m.contains("gpt-4.1") && m.contains("mini") {
        return Some((0.40, 0.10, 1.60));
    }
    if m.contains("gpt-4.1") {
        return Some((2.00, 0.50, 8.00));
    }

    // Reasoning (o-series). Order matters — match "o4-mini" / "o3-mini"
    // before "o4" / "o3".
    if m.contains("o4-mini") {
        return Some((1.10, 0.275, 4.40));
    }
    if m.contains("o3-mini") {
        return Some((1.10, 0.55, 4.40));
    }
    if m.contains("o3") {
        return Some((2.00, 0.50, 8.00));
    }

    None
}

/// Compute Codex/OpenAI cost in USD. `cached_input` is subtracted from
/// `input` so the rate-discounted portion is billed correctly.
pub fn codex_cost_usd(model: &str, input: u64, cached_input: u64, output: u64) -> f64 {
    let Some((in_per_m, cached_per_m, out_per_m)) = codex_rates(model) else {
        return 0.0;
    };
    let non_cached = input.saturating_sub(cached_input);
    per_million(non_cached, in_per_m)
        + per_million(cached_input, cached_per_m)
        + per_million(output, out_per_m)
}

// ─────────────────────────────────────────────────────────────────────────────
// Google — Gemini
// ─────────────────────────────────────────────────────────────────────────────

/// Per-1M-token rates for Gemini models, in USD.
///
/// Returned tuple: `(input, output)`. Cached-token discounts exist on
/// Gemini's API but local session files don't reliably distinguish cached
/// reads, so we don't apply them here.
///
/// Tiers (Q1 2026, Google AI public list, ≤128K-context tier):
/// - gemini-2.5-pro       →  $1.25 / $10.00
/// - gemini-2.5-flash     →  $0.30 / $2.50
/// - gemini-2.5-flash-lite→  $0.10 / $0.40
/// - gemini-2.0-flash     →  $0.10 / $0.40
/// - gemini-2.0-flash-lite→  $0.075/ $0.30
/// - gemini-1.5-pro       →  $1.25 / $5.00
/// - gemini-1.5-flash     →  $0.075/ $0.30
///
/// Unknown models fall back to a Flash-tier estimate so we don't silently
/// undercount usage for newly-released models.
pub fn gemini_rates(model: &str) -> (f64, f64) {
    let m = model.to_ascii_lowercase();

    // 2.5 family.
    if m.contains("2.5") && m.contains("pro") {
        return (1.25, 10.00);
    }
    if m.contains("2.5") && m.contains("flash-lite") {
        return (0.10, 0.40);
    }
    if m.contains("2.5") && m.contains("flash") {
        return (0.30, 2.50);
    }

    // 2.0 family.
    if m.contains("2.0") && m.contains("flash-lite") {
        return (0.075, 0.30);
    }
    if m.contains("2.0") && m.contains("flash") {
        return (0.10, 0.40);
    }

    // 1.5 family.
    if m.contains("1.5") && m.contains("pro") {
        return (1.25, 5.00);
    }
    if m.contains("1.5") && m.contains("flash") {
        return (0.075, 0.30);
    }

    // Unknown / unspecified — use a Flash-tier default rather than $0 so cost
    // estimates remain non-zero for novel model strings.
    (0.15, 0.60)
}

/// Compute Gemini cost in USD. The `_cached` argument is accepted but
/// currently ignored — see [`gemini_rates`] for rationale.
pub fn gemini_cost_usd(model: &str, input: u64, _cached: u64, output: u64) -> f64 {
    let (in_per_m, out_per_m) = gemini_rates(model);
    per_million(input, in_per_m) + per_million(output, out_per_m)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a token count + per-million rate into a USD amount.
///
/// Centralizing this keeps the unit conversion (and any future numerical
/// guards) in exactly one spot.
#[inline]
fn per_million(tokens: u64, usd_per_1m: f64) -> f64 {
    (tokens as f64 / 1_000_000.0) * usd_per_1m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_opus_uses_premium_rates() {
        let cost = claude_cost_usd("claude-opus-4-7", 1_000_000, 0, 0, 1_000_000);
        // input 1M @ $15  + output 1M @ $75  = $90.
        assert!((cost - 90.0).abs() < 0.0001);
    }

    #[test]
    fn claude_sonnet_uses_mid_tier() {
        let cost = claude_cost_usd("claude-sonnet-4-6", 1_000_000, 0, 0, 1_000_000);
        // input 1M @ $3 + output 1M @ $15 = $18.
        assert!((cost - 18.0).abs() < 0.0001);
    }

    #[test]
    fn codex_gpt5_nano_picks_nano_first() {
        // Order-sensitive: "gpt-5-nano" must NOT match the generic "gpt-5"
        // branch, which would give 25× the price.
        let (i, _, o) = codex_rates("gpt-5-nano").unwrap();
        assert_eq!(i, 0.05);
        assert_eq!(o, 0.40);
    }

    #[test]
    fn codex_caching_discount_applied() {
        // 1M input, half cached, no output. Cost should be:
        // 500k @ $1.25/M + 500k @ $0.125/M = $0.625 + $0.0625 = $0.6875
        let cost = codex_cost_usd("gpt-5", 1_000_000, 500_000, 0);
        assert!((cost - 0.6875).abs() < 0.0001);
    }

    #[test]
    fn gemini_unknown_falls_back_to_flash_estimate() {
        let (i, o) = gemini_rates("gemini-9000-mystery");
        assert_eq!(i, 0.15);
        assert_eq!(o, 0.60);
    }

    #[test]
    fn gemini_25_flash_lite_beats_pro_match() {
        // "flash-lite" must take precedence over the generic "flash" branch.
        let (i, o) = gemini_rates("gemini-2.5-flash-lite");
        assert_eq!(i, 0.10);
        assert_eq!(o, 0.40);
    }
}
