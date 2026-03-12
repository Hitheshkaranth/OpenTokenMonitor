# OpenTokenMonitor Provider Usage Research

Date: 2026-03-12

Scope: consumer and developer-facing usage tracking for Claude, Codex, and Gemini as it applies to this repository's local-first desktop monitor. This is not a generic SaaS billing dashboard review.

## Executive summary

OpenTokenMonitor can build a working tracker for all three providers, but the data quality is not uniform:

- Claude consumer plans expose real five-hour and seven-day rolling limits, but Anthropic's public plan docs do not publish exact remaining token counts for those windows. A working monitor should combine local log token counts with percent-only rolling-window data when available.
- Codex on ChatGPT plans exposes task-dependent usage limits by plan, but OpenAI does not publish a public local-environment usage API. A working monitor should treat live Codex plan windows as percent-only when sourced from the internal ChatGPT usage endpoint, and keep model token accounting separate.
- Gemini publishes model-specific RPM, TPM, and RPD quotas per project and documents token counting. A working monitor can reliably track per-model rate-limit ceilings and per-request token usage, but there is no official weekly Gemini limit to display.

## What changed in the repo

This pass updates the repo to match those findings:

- `src-tauri/src/usage/models.rs` now distinguishes exact, approximate, and percent-only usage windows.
- `src-tauri/src/providers/claude/mod.rs` and `src-tauri/src/providers/codex/mod.rs` stop pretending percent-based subscription windows are absolute token counts.
- `src/components/meters/WindowMeter.tsx` and `src/components/providers/ProviderOverview.tsx` now render real window labels instead of forcing every provider into "Session" and "Weekly".

## Capability matrix

| Provider | Official limit surface | Official remaining surface | Model-specific surface | Weekly limit available? | Best working strategy |
| --- | --- | --- | --- | --- | --- |
| Claude | Consumer help-center plan windows, Claude Code docs, API rate-limit docs | Percent-like usage guidance for consumer plans, no public exact token remainder | Per-model differences exist; Max also separates a Sonnet weekly limit | Yes for Pro/Max consumer plans | Local token logs + OAuth utilization percent + explicit "approximate" or "percent-only" UI states |
| Codex | ChatGPT help-center plan guidance plus API model docs for Codex models | No public local-environment usage API; Compliance API excludes local usage | Yes for API models (`gpt-5.1-codex`, `gpt-5.2-codex`, `gpt-5.3-codex`, etc.) | Plan-dependent, but public docs do not publish exact local weekly counters | Local Codex logs for per-model tokens/cost, internal usage endpoint only for percent windows, never fake exact token remainder |
| Gemini | AI Studio / Gemini API docs publish per-model RPM, TPM, RPD | Exact rate-limit ceilings are public; remaining budget is per-project and not fully standardized in one public endpoint | Yes, quotas vary by model and tier; token counting is documented | No official weekly limit found | Track per-model quotas and per-request tokens; optionally synthesize a rolling seven-day chart, but label it as derived, not official |

## Provider findings

### Claude

What is official and current:

- The Pro plan resets session-based usage every five hours and also has a seven-day weekly usage limit across models. Source: Anthropic Help Center, "What is the Pro plan?"  
  https://support.claude.com/en/articles/8325606-what-is-claude-pro
- The Max plan increases per-session capacity and has two weekly usage limits: one across all models and another for Sonnet models only. Source: Anthropic Help Center, "What is the Max plan?"  
  https://support.claude.com/en/articles/11049741-what-is-the-max-plan
- Claude Code exposes session cost and usage details through `/cost`, but Anthropic explicitly says `/cost` is not intended for Claude Max and Pro subscribers. Source: Claude Docs, "Manage costs effectively"  
  https://docs.claude.com/en/docs/claude-code/costs
- Claude Code status lines can display context-window usage, cost, duration, and model information inline. Source: Claude Docs, "Status line setup"  
  https://docs.claude.com/en/docs/claude-code/statusline
- Anthropic's public API docs say rate limits are applied separately per model and refer users to the Console for current limits and behavior. Source: Anthropic Docs, "Rate limits"  
  https://docs.claude.com/en/api/rate-limits

Useful but not a hard SLA:

- Anthropic has a localized help-center article for Claude Code with Pro/Max that gives directional examples for older-model usage, including roughly 10-40 Claude Code prompts per five hours and about 40-80 hours of Sonnet 4 within weekly limits for typical Pro users. Treat this as planning guidance, not a guaranteed quota. Source: Anthropic Help Center, French localized article  
  https://support.claude.com/fr/articles/11145838-utiliser-claude-code-avec-votre-forfait-pro-ou-max

Implications for OpenTokenMonitor:

- Exact remaining tokens in the five-hour or seven-day subscriber windows are not publicly documented in the same way API token quotas are.
- The repository's local `.claude` project logs are a strong source for actual token consumption by model and session.
- Any OAuth-based Claude subscription window should be shown as utilization percent unless Anthropic documents an exact counter with stable semantics.
- Claude Max/Opus or Sonnet-specific weekly windows should be represented as separate windows with clear labels, not merged into a single generic weekly badge.

Recommended product behavior:

- Show a "5h Window" and "7d Window" even when the underlying data is only percent-based.
- Show a distinct weekly model window only when the provider surface actually separates it.
- Keep token totals and estimated costs from local logs in a different row from subscription-window utilization.

### Codex

What is official and current:

- Codex is included with ChatGPT plans, and usage depends on plan and task complexity rather than a single public token counter. Source: OpenAI Help Center, "Using Codex with your ChatGPT plan"  
  https://help.openai.com/en/articles/11369540-codex-in-chatgpt
- OpenAI explicitly says Codex usage is available in the Compliance API for web or delegated cloud usage, but usage in local environments is not available there. Same source as above.
- OpenAI's public model docs currently list Codex API models such as `gpt-5.1-codex`, `gpt-5.1-codex-mini`, `gpt-5.2-codex`, and `gpt-5.3-codex`, each with a 400,000-token context window and explicit API tier limits. Sources:  
  https://developers.openai.com/api/docs/models/gpt-5.1-codex  
  https://developers.openai.com/api/docs/models/gpt-5.1-codex-mini  
  https://developers.openai.com/api/docs/models/gpt-5.2-codex  
  https://developers.openai.com/api/docs/models/gpt-5.3-codex
- OpenAI's official cookbook says Codex prompting should stay minimal and emphasizes that Codex models are optimized for agentic coding with sparse prompting. Source: OpenAI Cookbook, "GPT-5-Codex Prompting Guide"  
  https://cookbook.openai.com/examples/gpt-5-codex_prompting_guide

Implications for OpenTokenMonitor:

- There are two separate product layers to track:
  1. ChatGPT subscription usage windows for Codex in local tools.
  2. API model token accounting and rate limits for Codex models.
- The current repository already parses local Codex logs well enough to estimate per-model input, cached input, output tokens, and cost.
- The repository's live Codex bearer/cookie integration uses an internal ChatGPT backend endpoint that currently yields window utilization percentages. That is useful, but it is not the same as exact remaining tokens.
- Because OpenAI states local-environment usage is not available in the Compliance API, there is no official documented endpoint in the cited sources for exact local Codex remaining usage.

Recommended product behavior:

- Treat Codex plan windows as percent-only unless the CLI or app returns true counters with stable semantics.
- Keep Codex API model data in a separate feature area or details drawer so users do not confuse ChatGPT-plan windows with API tier rate limits.
- Surface local per-model token accounting from logs as exact or near-exact session data, and label plan windows independently.

### Gemini

What is official and current:

- Gemini API limits are model-specific and measured primarily in RPM, TPM, and RPD. They apply per project, not per API key, and RPD resets at midnight Pacific time. Source: Google AI for Developers, "Rate limits"  
  https://ai.google.dev/gemini-api/docs/rate-limits
- Google's quota page publishes concrete free-tier tables for models such as Gemini 2.5 Pro, Gemini 2.5 Flash, Gemini 2.5 Flash-Lite, and others. Source: Google AI for Developers, "Rate limits"  
  https://ai.google.dev/gemini-api/docs/quota
- Google's token counting docs describe `usage_metadata` in responses and `countTokens` for preflight estimation. Source: Google AI for Developers, "Token counting"  
  https://ai.google.dev/gemini-api/docs/token-counting
- The official Gemini CLI repository says Google-account login is best for quick starts, while API keys are best for model control and paid-tier access. The same README also highlights free-tier request envelopes for Gemini CLI usage with Google login. Source: Google Gemini CLI README  
  https://github.com/google-gemini/gemini-cli

Implications for OpenTokenMonitor:

- Gemini is the cleanest provider for exact published quota ceilings, but the published limit shape is not weekly.
- A correct Gemini monitor should prioritize daily and per-minute windows, and optionally show model-specific context and token counts.
- There is no official weekly Gemini limit in the cited current public docs, so a "weekly remaining" number would be a derived metric rather than an official provider budget.
- Local Gemini CLI data can still produce a rolling seven-day usage chart, but the UI should say "derived 7d usage" instead of implying an official seven-day hard limit.

Recommended product behavior:

- Show "Daily" and "RPM" or "Session" windows with request units when using official quota sources.
- Capture `usage_metadata` when present for exact per-request tokens.
- Add a provider note that Gemini weekly views are derived analytics unless Google publishes a first-class weekly budget.

## Recommended feature set

### P0

- Normalize usage windows into exact, approximate, and percent-only states.
- Preserve real window labels (`5h Window`, `7d Window`, `Daily`, `Weekly`) instead of collapsing them into a shared two-label UI.
- Separate plan-window usage from token-accounting usage.
- Add provider capability notes so the UI can explain when a number is official, estimated, or derived.

### P1

- Add a model breakdown panel per provider sourced from local logs or response metadata.
- Add a derived seven-day chart for Gemini with explicit labeling.
- Store a confidence field for every live counter source (`official`, `internal`, `derived_local`).

### P2

- Export normalized usage updates in an ACP-friendly payload shape so Forge or other ACP clients can consume the same telemetry.
- Add alert thresholds for 75%, 90%, and 95% usage, aligned with ACP's current session-usage discussion.

## Repository gap analysis

Before this pass, the repo had four misleading behaviors:

- `src-tauri/src/providers/claude/mod.rs` converted OAuth percentages into fake token limits.
- `src-tauri/src/providers/codex/bearer_fetcher.rs` exposed percentage windows but the UI rendered them like exact counts.
- `src-tauri/src/providers/gemini/mod.rs` had local fallback windows with invented ceilings and no provenance.
- `src/components/meters/WindowMeter.tsx` and `src/components/providers/ProviderOverview.tsx` forced provider windows into generic "Session" and "Weekly" labels.

This pass addresses the schema and UI labeling problem, but the remaining backlog still includes:

- Promoting Gemini's per-minute window to a clearer label than `Session`.
- Adding a dedicated per-model breakdown UI for Claude/Codex/Gemini.
- Emitting the normalized capability metadata over an ACP or Forge-friendly interface.

## Sources

- Anthropic Help Center, Pro plan: https://support.claude.com/en/articles/8325606-what-is-claude-pro
- Anthropic Help Center, Max plan: https://support.claude.com/en/articles/11049741-what-is-the-max-plan
- Anthropic Help Center, Claude Code localized usage guidance: https://support.claude.com/fr/articles/11145838-utiliser-claude-code-avec-votre-forfait-pro-ou-max
- Claude Docs, costs: https://docs.claude.com/en/docs/claude-code/costs
- Claude Docs, status line: https://docs.claude.com/en/docs/claude-code/statusline
- Anthropic Docs, rate limits: https://docs.claude.com/en/api/rate-limits
- OpenAI Help Center, Codex in ChatGPT: https://help.openai.com/en/articles/11369540-codex-in-chatgpt
- OpenAI model docs, `gpt-5.1-codex`: https://developers.openai.com/api/docs/models/gpt-5.1-codex
- OpenAI model docs, `gpt-5.1-codex-mini`: https://developers.openai.com/api/docs/models/gpt-5.1-codex-mini
- OpenAI model docs, `gpt-5.2-codex`: https://developers.openai.com/api/docs/models/gpt-5.2-codex
- OpenAI model docs, `gpt-5.3-codex`: https://developers.openai.com/api/docs/models/gpt-5.3-codex
- OpenAI Cookbook, Codex prompting guide: https://cookbook.openai.com/examples/gpt-5-codex_prompting_guide
- Google AI for Developers, Gemini rate limits: https://ai.google.dev/gemini-api/docs/rate-limits
- Google AI for Developers, Gemini quota tables: https://ai.google.dev/gemini-api/docs/quota
- Google AI for Developers, token counting: https://ai.google.dev/gemini-api/docs/token-counting
- Google Gemini CLI repository: https://github.com/google-gemini/gemini-cli
- Forge: https://forgeagents.dev/
- ACP agent plan: https://agentclientprotocol.com/protocol/agent-plan
- ACP session usage RFD: https://agentclientprotocol.com/rfds/session-usage
