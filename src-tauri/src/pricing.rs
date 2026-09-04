//! Per-model cost tables and cost computations.
//!
//! All rates are USD per **1 million tokens**, taken from each provider's
//! public pricing page. Edit this file in one place when prices change —
//! the scanners delegate every cost calculation here.
//!
//! Pricing source-of-truth links (verify before bumping rates):
//! - Anthropic:  <https://www.anthropic.com/pricing>
//! - OpenAI:     <https://developers.openai.com/api/docs/pricing>
//! - Google AI:  <https://ai.google.dev/gemini-api/docs/pricing>
//!
//! Rates last reviewed: **2026-09**. Prices reflect the standard / ≤200K-context
//! tier; long-context surcharges are not tracked because we don't capture
//! context size from local logs.
//!
//! ### Conventions
//!
//! - Claude rate tuple: `(input, cache_read, cache_write, output)`
//! - Codex rate tuple:  `(input, cached_input, output)`
//! - Antigravity rate tuple: `(input, cached_input, output)`
//!
//! Every table normalizes `.` to `-` before matching, so `opus-4.5` and
//! `opus-4-5` resolve identically no matter how the log spelled the id.
//!
//! ### Adding a new model
//!
//! 1. Add a row to the matching table (Claude / Codex / Antigravity).
//! 2. Confirm `normalize_*_model` in `usage_scanners.rs` preserves enough of the
//!    id for the table to match it — those helpers strip vendor prefixes and
//!    snapshot dates but deliberately keep version digits.
//! 3. Update the model picker in any UI that shows per-model breakdowns.

// ─────────────────────────────────────────────────────────────────────────────
// Anthropic — Claude
// ─────────────────────────────────────────────────────────────────────────────

/// Per-1M-token rates for Claude models, in USD.
///
/// Returned tuple: `(input, cache_read, cache_write, output)`.
///
/// Anthropic's convention is a 5-minute cache write at 1.25× input and a cache
/// read at 0.1× input. Claude Fable 5.1 is the one exception — its cache reads
/// are $0.25/MTok (0.025×), not $1.00.
///
/// Tiers (Sept 2026):
/// - Fable / Mythos 5.x     →  $10   / $50
/// - Opus 4.5 / 4.6 / 4.7 / 4.8 / 5  →  $5 / $25
/// - Opus 4 / 4.1 / 3 (legacy)       →  $15 / $75
/// - Sonnet 5                        →  $2 / $10
/// - Sonnet 3.5 / 3.7 / 4 / 4.5 / 4.6 → $3 / $15
/// - Haiku 4.5              →  $1    / $5
/// - Haiku 3.5              →  $0.80 / $4
/// - Haiku 3                →  $0.25 / $1.25
///
/// Unrecognized ids return `None` so cost stays at $0 rather than being
/// silently attributed to the wrong tier.
pub fn claude_rates(model: &str) -> Option<(f64, f64, f64, f64)> {
    let m = model.to_ascii_lowercase().replace('.', "-");

    // Rate constants, named so the ordering below reads as policy rather than
    // as a wall of magic numbers.
    const FABLE_5_1: (f64, f64, f64, f64) = (10.00, 0.25, 12.50, 50.00);
    const FABLE: (f64, f64, f64, f64) = (10.00, 1.00, 12.50, 50.00);
    const OPUS_CURRENT: (f64, f64, f64, f64) = (5.00, 0.50, 6.25, 25.00);
    const OPUS_LEGACY: (f64, f64, f64, f64) = (15.00, 1.50, 18.75, 75.00);
    const SONNET_CURRENT: (f64, f64, f64, f64) = (2.00, 0.20, 2.50, 10.00);
    const SONNET_LEGACY: (f64, f64, f64, f64) = (3.00, 0.30, 3.75, 15.00);
    const HAIKU_CURRENT: (f64, f64, f64, f64) = (1.00, 0.10, 1.25, 5.00);
    const HAIKU_3_5: (f64, f64, f64, f64) = (0.80, 0.08, 1.00, 4.00);
    const HAIKU_3: (f64, f64, f64, f64) = (0.25, 0.03, 0.30, 1.25);

    // Fable / Mythos share a tier and a price; 5.1 differs only on cache reads.
    if m.contains("fable") || m.contains("mythos") {
        if m.contains("5-1") {
            return Some(FABLE_5_1);
        }
        return Some(FABLE);
    }

    // Claude 3.x used a `claude-3-<size>` / `claude-3-5-<size>` id shape, which
    // the 4.x-era branches below would misread. Handle that era first.
    if m.starts_with("claude-3") {
        if m.contains("opus") {
            return Some(OPUS_LEGACY);
        }
        if m.contains("sonnet") {
            return Some(SONNET_LEGACY);
        }
        if m.contains("3-5-haiku") {
            return Some(HAIKU_3_5);
        }
        if m.contains("haiku") {
            return Some(HAIKU_3);
        }
        return None;
    }

    if m.contains("opus") {
        // Opus 4.0 / 4.1 predate the November 2025 price cut and still bill at
        // $15/$75. Everything at 4.5 and above — including versions newer than
        // this table — is on the $5/$25 tier.
        if m.contains("opus-4-0") || m.contains("opus-4-1") || m.ends_with("opus-4") {
            return Some(OPUS_LEGACY);
        }
        return Some(OPUS_CURRENT);
    }

    if m.contains("sonnet") {
        // Sonnet 5 dropped to $2/$10; every earlier Sonnet stayed at $3/$15.
        if m.contains("sonnet-4-0")
            || m.contains("sonnet-4-5")
            || m.contains("sonnet-4-6")
            || m.ends_with("sonnet-4")
        {
            return Some(SONNET_LEGACY);
        }
        return Some(SONNET_CURRENT);
    }

    if m.contains("haiku") {
        // Most specific first: "haiku-3-5" also contains "haiku-3".
        if m.contains("haiku-3-5") {
            return Some(HAIKU_3_5);
        }
        if m.contains("haiku-3") {
            return Some(HAIKU_3);
        }
        return Some(HAIKU_CURRENT);
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
// OpenAI — Codex / GPT-6 / GPT-5.x / GPT-4.1 / o-series
// ─────────────────────────────────────────────────────────────────────────────

/// Per-1M-token rates for OpenAI models, in USD.
///
/// Returned tuple: `(input, cached_input, output)`.
///
/// Tiers (Sept 2026, OpenAI public list):
/// - gpt-6-astra    →  $10.00 / $1.00  / $50.00
/// - gpt-5.6-sol    →  $4.00  / $0.40  / $20.00
/// - gpt-5.6-terra  →  $2.00  / $0.20  / $12.00
/// - gpt-5.6-luna   →  $0.20  / $0.02  / $1.20
/// - gpt-5.5-pro    →  $30.00 / $3.00  / $180.00
/// - gpt-5.5        →  $5.00  / $0.50  / $30.00
/// - gpt-5.3-codex  →  $1.75  / $0.175 / $14.00
/// - gpt-5.2-pro    →  $21.00 / $2.10  / $168.00
/// - gpt-5.2        →  $1.75  / $0.175 / $14.00
/// - gpt-5.1 / gpt-5 →  $1.25 / $0.125 / $10.00
/// - gpt-5-mini     →  $0.25  / $0.025 / $2.00
/// - gpt-5-nano     →  $0.05  / $0.005 / $0.40
/// - gpt-4.1        →  $2.00  / $0.50  / $8.00
/// - gpt-4.1-mini   →  $0.40  / $0.10  / $1.60
/// - gpt-4.1-nano   →  $0.10  / $0.025 / $0.40
/// - o3             →  $2.00  / $0.50  / $8.00
/// - o3-mini        →  $1.10  / $0.55  / $4.40
/// - o4-mini        →  $1.10  / $0.275 / $4.40
///
/// OpenAI publishes no cached-input rate for the `-pro` models; we assume the
/// house-standard 0.1× so a cached prompt isn't billed at full price.
pub fn codex_rates(model: &str) -> Option<(f64, f64, f64)> {
    let m = model.to_ascii_lowercase();

    if m.contains("gpt-6") {
        return Some((10.00, 1.00, 50.00));
    }

    // GPT-5.6 variants are named, not sized — match the name.
    if m.contains("gpt-5.6") {
        if m.contains("sol") {
            return Some((4.00, 0.40, 20.00));
        }
        if m.contains("terra") {
            return Some((2.00, 0.20, 12.00));
        }
        if m.contains("luna") {
            return Some((0.20, 0.02, 1.20));
        }
    }

    if m.contains("gpt-5.5") {
        if m.contains("pro") {
            return Some((30.00, 3.00, 180.00));
        }
        return Some((5.00, 0.50, 30.00));
    }

    // gpt-5.3-codex is the current Codex CLI default.
    if m.contains("gpt-5.3") {
        return Some((1.75, 0.175, 14.00));
    }

    if m.contains("gpt-5.2") {
        if m.contains("pro") {
            return Some((21.00, 2.10, 168.00));
        }
        return Some((1.75, 0.175, 14.00));
    }

    if m.contains("gpt-5.1") {
        return Some((1.25, 0.125, 10.00));
    }

    // GPT-5 family — match the most specific suffix first, or "gpt-5-nano"
    // would fall into the generic branch and bill at 25× its real rate.
    if m.contains("gpt-5") && m.contains("nano") {
        return Some((0.05, 0.005, 0.40));
    }
    if m.contains("gpt-5") && m.contains("mini") {
        return Some((0.25, 0.025, 2.00));
    }
    if m.contains("gpt-5") {
        // Includes gpt-5-codex, which bills at plain GPT-5 rates.
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
// Google — Antigravity
// ─────────────────────────────────────────────────────────────────────────────

/// Per-1M-token rates for Antigravity models, in USD.
///
/// Returned tuple: `(input, cached_input, output)`. Google publishes a real
/// cached-input rate per model (uniformly 0.1× today, but tabulated rather than
/// derived so a future divergence is a one-line edit).
///
/// Tiers (Sept 2026, ≤200K-context tier):
/// - 3.6 / 3.7 / 3.8 Flash  →  $0.75  / $0.075  / $3.75
/// - 3.5 Flash              →  $1.50  / $0.15   / $9.00
/// - 3.5 Flash-Lite         →  $0.30  / $0.03   / $2.50
/// - 3.1 Pro                →  $2.00  / $0.20   / $12.00
/// - 3.1 Flash-Lite         →  $0.25  / $0.025  / $1.50
/// - 2.5 Pro                →  $1.25  / $0.125  / $10.00
/// - 2.5 Flash              →  $0.30  / $0.03   / $2.50
/// - 2.5 Flash-Lite         →  $0.10  / $0.01   / $0.40
/// - 2.0 Flash              →  $0.10  / $0.01   / $0.40
/// - 2.0 Flash-Lite         →  $0.075 / $0.0075 / $0.30
/// - 1.5 Pro                →  $1.25  / $0.125  / $5.00
/// - 1.5 Flash              →  $0.075 / $0.0075 / $0.30
///
/// NOTE: the $0.75 input rate on 3.6/3.7/3.8 Flash is promotional and doubles to
/// $1.50 on 2027-01-01. Revisit this table before then.
///
/// Unknown models fall back to the current Flash tier so we don't silently
/// undercount usage for newly-released models.
pub fn antigravity_rates(model: &str) -> (f64, f64, f64) {
    let m = model.to_ascii_lowercase().replace('.', "-");

    const FLASH_CURRENT: (f64, f64, f64) = (0.75, 0.075, 3.75);
    const PRO_CURRENT: (f64, f64, f64) = (2.00, 0.20, 12.00);

    // 3.x family (Antigravity era). Versioned rows first, then the generic
    // Pro/Flash catch-alls so an unreleased 3.9 still lands on the right tier.
    if m.contains("3-8") && m.contains("flash") {
        return FLASH_CURRENT;
    }
    if m.contains("3-7") && m.contains("flash") {
        return FLASH_CURRENT;
    }
    if m.contains("3-6") && m.contains("flash") {
        return FLASH_CURRENT;
    }
    // "flash-lite" must precede "flash" — it contains it.
    if m.contains("3-5") && m.contains("flash-lite") {
        return (0.30, 0.03, 2.50);
    }
    if m.contains("3-5") && m.contains("flash") {
        return (1.50, 0.15, 9.00);
    }
    if m.contains("3-1") && m.contains("flash-lite") {
        return (0.25, 0.025, 1.50);
    }
    if m.contains("3-1") && m.contains("pro") {
        return PRO_CURRENT;
    }
    if m.contains("pro") && m.contains("-3") {
        return PRO_CURRENT;
    }
    if m.contains("flash") && m.contains("-3") {
        return FLASH_CURRENT;
    }

    // 2.5 family.
    if m.contains("2-5") && m.contains("pro") {
        return (1.25, 0.125, 10.00);
    }
    if m.contains("2-5") && m.contains("flash-lite") {
        return (0.10, 0.01, 0.40);
    }
    if m.contains("2-5") && m.contains("flash") {
        return (0.30, 0.03, 2.50);
    }

    // 2.0 family.
    if m.contains("2-0") && m.contains("flash-lite") {
        return (0.075, 0.0075, 0.30);
    }
    if m.contains("2-0") && m.contains("flash") {
        return (0.10, 0.01, 0.40);
    }

    // 1.5 family.
    if m.contains("1-5") && m.contains("pro") {
        return (1.25, 0.125, 5.00);
    }
    if m.contains("1-5") && m.contains("flash") {
        return (0.075, 0.0075, 0.30);
    }

    // Unknown / unspecified — use the current Flash tier rather than $0 so cost
    // estimates remain non-zero for novel model strings.
    FLASH_CURRENT
}

/// Compute Antigravity cost in USD, billing the cached portion at the model's
/// published cached-input rate.
pub fn antigravity_cost_usd(model: &str, input: u64, cached: u64, output: u64) -> f64 {
    let (in_per_m, cached_per_m, out_per_m) = antigravity_rates(model);
    let non_cached = input.saturating_sub(cached);
    per_million(non_cached, in_per_m)
        + per_million(cached, cached_per_m)
        + per_million(output, out_per_m)
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

    const M: u64 = 1_000_000;

    #[test]
    fn claude_current_opus_uses_reduced_rates() {
        // Opus 4.5+ dropped to $5/$25 — the old $15/$75 table overcharged 3×.
        let cost = claude_cost_usd("claude-opus-4-7", M, 0, 0, M);
        assert!((cost - 30.0).abs() < 0.0001);
    }

    #[test]
    fn claude_legacy_opus_keeps_premium_rates() {
        // Opus 4.0 / 4.1 predate the price cut.
        let cost = claude_cost_usd("claude-opus-4-1", M, 0, 0, M);
        assert!((cost - 90.0).abs() < 0.0001);
        let bare_four = claude_cost_usd("claude-opus-4", M, 0, 0, M);
        assert!((bare_four - 90.0).abs() < 0.0001);
    }

    #[test]
    fn claude_sonnet_tiers_split_at_five() {
        // Sonnet 5 is $2/$10; Sonnet 4.6 and earlier stayed at $3/$15.
        let five = claude_cost_usd("claude-sonnet-5", M, 0, 0, M);
        assert!((five - 12.0).abs() < 0.0001);
        let four_six = claude_cost_usd("claude-sonnet-4-6", M, 0, 0, M);
        assert!((four_six - 18.0).abs() < 0.0001);
    }

    #[test]
    fn claude_fable_cache_read_is_discounted() {
        // Fable 5.1 reads cache at $0.25/MTok, a quarter of Fable 5's $1.00.
        let (_, read_5_1, _, _) = claude_rates("claude-fable-5-1").unwrap();
        let (_, read_5, _, _) = claude_rates("claude-fable-5").unwrap();
        assert_eq!(read_5_1, 0.25);
        assert_eq!(read_5, 1.00);
    }

    #[test]
    fn claude_three_era_ids_price_correctly() {
        // The `claude-3-5-sonnet` id shape must not fall through the 4.x branches.
        assert_eq!(
            claude_rates("claude-3-5-sonnet"),
            Some((3.00, 0.30, 3.75, 15.00))
        );
        assert_eq!(
            claude_rates("claude-3-haiku"),
            Some((0.25, 0.03, 0.30, 1.25))
        );
        assert_eq!(
            claude_rates("claude-3-5-haiku"),
            Some((0.80, 0.08, 1.00, 4.00))
        );
    }

    #[test]
    fn claude_dotted_and_dashed_versions_agree() {
        assert_eq!(
            claude_rates("claude-opus-4.5"),
            claude_rates("claude-opus-4-5")
        );
    }

    #[test]
    fn claude_unknown_model_costs_nothing() {
        assert!(claude_rates("unknown-model").is_none());
        assert_eq!(claude_cost_usd("unknown-model", 1000, 0, 0, 500), 0.0);
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
    fn codex_gpt52_and_codex_variant_share_a_tier() {
        assert_eq!(codex_rates("gpt-5.2"), Some((1.75, 0.175, 14.00)));
        assert_eq!(codex_rates("gpt-5.3-codex"), Some((1.75, 0.175, 14.00)));
    }

    #[test]
    fn codex_pro_variants_beat_their_base_tier() {
        assert_eq!(codex_rates("gpt-5.5-pro"), Some((30.00, 3.00, 180.00)));
        assert_eq!(codex_rates("gpt-5.5"), Some((5.00, 0.50, 30.00)));
    }

    #[test]
    fn codex_caching_discount_applied() {
        // 1M input, half cached, no output. Cost should be:
        // 500k @ $1.25/M + 500k @ $0.125/M = $0.625 + $0.0625 = $0.6875
        let cost = codex_cost_usd("gpt-5", M, 500_000, 0);
        assert!((cost - 0.6875).abs() < 0.0001);
    }

    #[test]
    fn antigravity_unknown_falls_back_to_current_flash() {
        assert_eq!(
            antigravity_rates("antigravity-9000-mystery"),
            (0.75, 0.075, 3.75)
        );
    }

    #[test]
    fn antigravity_flash_lite_beats_generic_flash_match() {
        // "flash-lite" must take precedence over the generic "flash" branch.
        assert_eq!(
            antigravity_rates("antigravity-2.5-flash-lite"),
            (0.10, 0.01, 0.40)
        );
        assert_eq!(
            antigravity_rates("antigravity-3.5-flash-lite"),
            (0.30, 0.03, 2.50)
        );
    }

    #[test]
    fn antigravity_cost_uses_table_cached_rate() {
        // 1M fully-cached input on 3.5 Flash bills at $0.15/M, not $0.15 of the
        // input rate derived some other way.
        let cost = antigravity_cost_usd("antigravity-3.5-flash", M, M, 0);
        assert!((cost - 0.15).abs() < 0.0001);
    }
}
