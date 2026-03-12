# Forge Swarm Guide

Date: 2026-03-12

This folder contains a Codex-first swarm package for implementing provider usage tracking improvements in OpenTokenMonitor with Forge.

## Why Forge

Forge is a universal CLI for coding agents built on the Agent Client Protocol (ACP). The current official site positions it as one interface for Claude, Codex, Gemini, and other agents, and highlights shared session history plus ACP support for agent plans and session modes.

Official references:

- Forge: https://forgeagents.dev/
- ACP Agent Plan: https://agentclientprotocol.com/protocol/agent-plan
- ACP Session Usage RFD: https://agentclientprotocol.com/rfds/session-usage

## Recommended session layout

Use 11 Codex sessions:

1. `00-orchestrator`
2. `01-claude-surface-research`
3. `02-codex-surface-research`
4. `03-gemini-surface-research`
5. `04-schema-capability-design`
6. `05-rust-provider-windows`
7. `06-frontend-window-rendering`
8. `07-model-breakdown-storage`
9. `08-acp-forge-export`
10. `09-tests-and-fixtures`
11. `10-docs-release-readiness`

The session list is captured in [swarm-manifest.json](./swarm-manifest.json).

## How to run

1. Install Forge if needed with the official command from the Forge homepage: `npm i -g @forge-agents/forge`.
2. Open one Forge Codex session per prompt file in `docs/forge/prompts/`.
3. Start with the orchestrator prompt and have it track the worker sessions using ACP-style plan states: `pending`, `in_progress`, and `completed`.
4. Feed every worker the research memo in `docs/research/provider-usage-research-2026-03-12.md` plus the specific prompt file for that worker.
5. Merge outputs in this order:
   - Research sessions
   - Schema/design
   - Backend and frontend implementation
   - Storage/export
   - Tests
   - Docs and release notes

## Repo-specific objectives

The swarm is designed to deliver these outcomes:

- Provider windows with explicit semantics: exact, approximate, or percent-only.
- Separate plan-window usage from per-model token accounting.
- Real provider labels for windows instead of generic session/weekly placeholders.
- A model-breakdown roadmap for Claude, Codex, and Gemini.
- ACP- and Forge-friendly usage export shapes for future multi-agent telemetry.

## Guardrails for every worker

- Do not invent official limits when the provider only exposes utilization percent.
- Do not label Gemini as having an official weekly limit unless a source published after 2026-03-12 says otherwise.
- Keep ChatGPT Codex plan windows separate from OpenAI API model quotas.
- Prefer local logs for exact token accounting and live provider surfaces for official quota windows.
- Always record provenance: `official`, `internal`, or `derived_local`.

## Acceptance criteria

- `cargo test` passes in `src-tauri`.
- `npm.cmd run build` passes at repo root.
- All window UIs show correct labels and semantics.
- Research citations are preserved in docs.
- No feature implies unsupported provider precision.
