# Ralph CLI — Implementation Specification (v2) — Complete

Normative specification. Implementations MUST follow MUST/SHOULD/MAY requirements.

## Goals
- Turn feature PRD into automated implementation loop using GitHub Copilot CLI
- Keep work auditable (append-only ledger) and reviewable (branch-per-PRD)
- Make planning re-entrant
- Support Rust ecosystem (cargo, clippy, AVRO/JSON Schema)
- Support Nix flake-based reproducible builds

## Copilot CLI contracts

Planning (interactive):
copilot --agent=ralph-planner --model claude-opus-4.5

Run loop (unattended):
copilot -p "<prompt>" --agent=ralph-implementer --model gpt-5-mini --allow-all-tools --allow-all-paths

## Validation gates per iteration
1. fmt
2. lint
3. typecheck
(Short-circuit on first failure; full test sweep every 5th iteration)

## Rust + Nix
See docs/RUST_REQUIREMENTS.md and docs/NIX_REQUIREMENTS.md
