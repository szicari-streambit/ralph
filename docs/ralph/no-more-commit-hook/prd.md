# no more commit hook

Canonical machine PRD: ralph/tasks/no-more-commit-hook/prd.json

<!-- RALPH:BEGIN PLANNING_LOG -->

## Planning Session 2026-01-23

### Discovery Q&A (Round 1)

**Q1: Problem statement?**
A: Pre-commit hook causes friction - requires REQ-xx in every commit, even non-Ralph changes. AI not always committing between iterations (possibly due to REQ-xx requirement or just forgetting).

**Q2: Scope?**
A: "Probably don't need the hook at all" - suggests removing both pre-commit and commit-msg hooks.

**Q3-6: Migration, backward compatibility?**
A: No known backward compatibility issues.

**Q7-10: Validation?**
A: User asks "what validation should remain?" - needs recommendation.

### Codebase Analysis

**Hook Infrastructure Found:**
- `.githooks/pre-commit` - Runs: cargo fmt --check, clippy, cargo check, cargo test
- `.githooks/commit-msg` - Calls `ralph hook commit-msg "$1"`
- `crates/ralph-cli/src/commands/hook.rs` - Validates commit msg contains REQ-xx pattern
- `crates/ralph-cli/src/main.rs` - Exposes `ralph hook commit-msg <file>` subcommand

**Documentation Found:**
- `docs/ralph/pre-commit/prd.md` - PRD for pre-commit hook feature
- `SPEC.md` - Mentions validation gates (fmt, lint, typecheck)

**Code to potentially remove:**
- `.githooks/pre-commit`
- `.githooks/commit-msg`
- `crates/ralph-cli/src/commands/hook.rs`
- Hook subcommand from `main.rs`

### Discovery Q&A (Round 2)

**Q11: Validation recommendation?**
A: Agreed - keep validation in implementation loop only.

**Q12: Code removal scope?**
A: Keep the `ralph hook` subcommand and `commands/hook.rs` for now.

**Q13: Hook files cleanup?**
A: Remove `pre-commit` and `commit-msg` scripts entirely.

**Q14: Agent behavior / REQ-xx?**
A: REQ-xx in commits is helpful but not required. Acceptable to go without if it causes friction.

**Q15: Test cleanup?**
A: Keep tests as-is.

### Discovery Q&A (Round 3)

**Q16: Pre-commit configuration?**
A: Yes, use the pre-commit framework with `.pre-commit-config.yaml` for opt-in hooks.

**Q17: Pre-commit hooks to include?**
A: Replicate all current checks: fmt, clippy, check, test.

### Decisions Summary

| Decision | Choice |
|----------|--------|
| Remove hook scripts | Yes - delete `.githooks/pre-commit` and `.githooks/commit-msg` |
| Keep hook code | Yes - `commands/hook.rs` and CLI subcommand remain |
| Validation approach | Implementation loop only (per SPEC.md) |
| REQ-xx requirement | Optional/encouraged, not enforced |
| Pre-commit framework | Yes - create `.pre-commit-config.yaml` for opt-in validation |

---

## PRD Draft v1

### Problem Statement
Git commit hooks (pre-commit and commit-msg) cause developer friction by:
1. Requiring REQ-xx references in every commit message, even for non-Ralph changes
2. Running full validation suite on every commit (slow)
3. Potentially blocking AI agent from committing during implementation iterations

### Solution
1. Remove the mandatory hook scripts
2. Provide opt-in validation via pre-commit framework (`.pre-commit-config.yaml`)
3. Developers can choose to install hooks with `pre-commit install`

### Scope
- **In scope**: 
  - Delete `.githooks/pre-commit` and `.githooks/commit-msg`
  - Create `.pre-commit-config.yaml` with cargo fmt, clippy, check, test
- **Out of scope**: Removing hook.rs code, modifying CLI, changing validation profiles

### Requirements

**REQ-01: Remove pre-commit hook script**
- Delete `.githooks/pre-commit`
- Acceptance criteria:
  - File `.githooks/pre-commit` no longer exists
  - `git commit` succeeds without running validation checks

**REQ-02: Remove commit-msg hook script**  
- Delete `.githooks/commit-msg`
- Acceptance criteria:
  - File `.githooks/commit-msg` no longer exists
  - Commits succeed without REQ-xx reference requirement

**REQ-03: Create pre-commit framework configuration**
- Create `.pre-commit-config.yaml` at repository root
- Acceptance criteria:
  - File exists with valid pre-commit configuration
  - Includes hook for `cargo fmt --check`
  - Includes hook for `cargo clippy --all-targets --all-features -- -D warnings`
  - Includes hook for `cargo check --all-targets --all-features`
  - Includes hook for `cargo test --all-features`
  - Running `pre-commit run --all-files` executes all checks

**REQ-04: Preserve .githooks directory structure**
- Keep `.githooks/` directory with `.gitkeep` for potential future use
- Acceptance criteria:
  - Directory `.githooks/` exists
  - Contains `.gitkeep` file

### Discovery Q&A (Round 4)

**Q18: Documentation update?**
A: Yes, add requirement for documentation updates.

**Q19: Anything else?**
A: Nothing else to add.

<!-- RALPH:END PLANNING_LOG -->
