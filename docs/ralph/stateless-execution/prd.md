# Stateless Execution - Install Ralph Anywhere

Canonical machine PRD: ralph/tasks/stateless-execution/prd.json

<!-- RALPH:BEGIN PLANNING_LOG -->
## Planning Session 2026-01-20

### Problem Statement

The Ralph CLI currently only works within the ralph-spec repository. The goal is to make Ralph installable via Nix and executable from **any repository**, enabling users to install Ralph on any machine and use it to manage PRD-driven development in their own projects.

### Discovery Q&A

**Q1: What problem does "stateless execution" solve?**
A: The Nix package built from flake.nix needs to include agent templates and be executable from any repository. Currently templates/ are not included in the Nix flake.

**Q2: What does "stateless" mean for Ralph?**
A: Ralph should be self-contained—installable anywhere and able to initialize new projects with bundled templates. The ledger is unrelated to this feature.

**Q3: Template installation behavior?**
A: `ralph init` should copy templates from the installed package location to the target repo.

**Q4: Which templates should be bundled?**
A: Only `.github/agents/*.agent.md` (agent instructions). Not the Ralph-project-specific files (Cargo.toml, flake.nix, .envrc from templates/).

**Q5: Agent template discovery at runtime?**
A: The package should be self-referential for finding templates. Nix makes this easy via compile-time path injection.

**Q6: Template customization/overrides?**
A: Not needed. Users can modify repo-local templates after init if they want customization.

**Q7: Validation profiles strategy?**
A: Option A selected—bundle default profiles in package, allow local override. Target repos can override with their own `ralph/validation.json`. However, validation config changes are **out of scope** for this feature; we'll handle that later.

**Q8: `ralph init` scope?**
A: Copy agent templates only (`.github/agents/*.agent.md`). Validation config is deferred.

**Q9: Config override location?**
A: Keep current pathing (`ralph/validation.json`).

**Q10: Init idempotency and flags?**
A: No flags needed currently. Behavior TBD (fail/skip/overwrite on existing files).

### Decisions

| Decision | Choice |
|----------|--------|
| Bundle agent templates in Nix package | Yes |
| Template discovery | Self-referential (Nix compile-time path) |
| `ralph init` copies | `.github/agents/*.agent.md` only |
| Validation config changes | Deferred to future feature |
| Template overrides | Not supported (edit local copies) |
| Flake `templates.<name>` output | Not needed (CLI handles init) |

### Architecture Notes

```
┌─────────────────────────────────────────────────────────┐
│                   Nix Package                           │
├─────────────────────────────────────────────────────────┤
│  bin/ralph          (CLI binary)                        │
│  share/ralph/agents/                                    │
│    ├── ralph-implementer.agent.md                       │
│    └── ralph-planner.agent.md                           │
└─────────────────────────────────────────────────────────┘
                          │
                          │ ralph init
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   Target Repository                     │
├─────────────────────────────────────────────────────────┤
│  .github/agents/                                        │
│    ├── ralph-implementer.agent.md  (copied)             │
│    └── ralph-planner.agent.md      (copied)             │
└─────────────────────────────────────────────────────────┘
```

### Next Steps (when resuming)

1. Define concrete requirements with acceptance criteria
2. Update flake.nix to bundle agents in `share/ralph/agents/`
3. Implement `ralph init` command to copy bundled agents
4. Add compile-time path injection for template discovery
5. Test installation and init from outside the repo

---

## Planning Session 2026-01-21 (Continued)

### Additional Q&A

**Q11: Init destination path?**
A: Always copy to `.github/agents/` relative to git root (not CWD).

**Q12: Error handling for existing files?**
A: Skip existing files silently.

**Q13: Compile-time path injection approach?**
A: Use `makeWrapper` in flake.nix to set `RALPH_SHARE_DIR` environment variable pointing to the Nix store path.

**Q14: Dev mode fallback?**
A: When `RALPH_SHARE_DIR` is not set, fall back to embedded template strings (current behavior) for `cargo run` development.

**Q15: Agent template files to bundle?**
A: `templates/.github/agents/ralph-implementer.agent.md` and `templates/.github/agents/ralph-planner.agent.md`

**Q16: Success output?**
A: "Planner and Implementer agents installed"

### Updated Decisions

| Decision | Choice |
|----------|--------|
| Init destination | Git root (not CWD) |
| Existing file handling | Skip silently |
| Path injection method | `makeWrapper` with `RALPH_SHARE_DIR` env var |
| Dev mode | Fallback to embedded templates |
| Success message | "Planner and Implementer agents installed" |

### Refined Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Nix Package                           │
├─────────────────────────────────────────────────────────┤
│  bin/ralph (wrapped)                                    │
│    └── RALPH_SHARE_DIR=/nix/store/.../share/ralph      │
│  share/ralph/agents/                                    │
│    ├── ralph-implementer.agent.md                       │
│    └── ralph-planner.agent.md                           │
└─────────────────────────────────────────────────────────┘
                          │
                          │ ralph init (from any repo)
                          ▼
┌─────────────────────────────────────────────────────────┐
│           Target Repository (at git root)               │
├─────────────────────────────────────────────────────────┤
│  .github/agents/                                        │
│    ├── ralph-implementer.agent.md  (copied or skipped)  │
│    └── ralph-planner.agent.md      (copied or skipped)  │
└─────────────────────────────────────────────────────────┘
```

### Requirements (Draft)

**REQ-01: Bundle agent templates in Nix package**
- Copy `templates/.github/agents/*.agent.md` to `$out/share/ralph/agents/` during Nix build
- Files must be readable from the Nix store

**REQ-02: Wrap binary with RALPH_SHARE_DIR**
- Use `makeWrapper` to set `RALPH_SHARE_DIR` pointing to `$out/share/ralph`
- Binary must have access to this env var at runtime

**REQ-03: Init reads from RALPH_SHARE_DIR when set**
- If `RALPH_SHARE_DIR` is set, read agent templates from `$RALPH_SHARE_DIR/agents/`
- If not set (dev mode), use embedded template strings as fallback

**REQ-04: Init copies to git root**
- Detect git repository root using `git rev-parse --show-toplevel` or walking up to find `.git`
- Copy agents to `<git-root>/.github/agents/`
- Error if not in a git repository

**REQ-05: Skip existing agent files**
- If `.github/agents/ralph-planner.agent.md` exists, skip it
- If `.github/agents/ralph-implementer.agent.md` exists, skip it
- No error or warning for skipped files

**REQ-06: Success output message**
- On success, print "Planner and Implementer agents installed"
- Verbose mode can show additional details

---

### Additional Q&A (Continued)

**Q17: Should ralph init still create other directories and boilerplate?**
A: Yes, ralph init should continue creating all standard boilerplate: directories (`ralph/tasks`, `docs/ralph`, `.githooks`), validation.json, and commit-msg hook.

**Q18: Which agent templates are authoritative?**
A: The templates in `templates/.github/agents/` are authoritative. The embedded strings in init.rs are outdated stubs.

**Q19: Template directory structure in Nix store?**
A: Preserve `templates/` folder structure: `share/ralph/templates/.github/agents/`. This mirrors the source tree layout.

**Q20: Preserve feedback loop content when replacing embedded templates?**
A: Yes. The authoritative `ralph-implementer.agent.md` (111 lines) contains comprehensive feedback loop guidance including:
- "⚠️ CRITICAL INSTRUCTIONS - READ FIRST" section
- "⚠️ PREVIOUS ITERATION FAILED VALIDATION" detection
- Validation pipeline explanation
- Common mistakes section

This is a superset of the current 13-line embedded stub. Replacement enhances, not removes, feedback loop content.

### Updated Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Nix Package                           │
├─────────────────────────────────────────────────────────┤
│  bin/ralph (wrapped)                                    │
│    └── RALPH_SHARE_DIR=/nix/store/.../share/ralph      │
│  share/ralph/templates/                                 │
│    └── .github/agents/                                  │
│          ├── ralph-implementer.agent.md                 │
│          └── ralph-planner.agent.md                     │
└─────────────────────────────────────────────────────────┘
                          │
                          │ ralph init (from any repo)
                          ▼
┌─────────────────────────────────────────────────────────┐
│           Target Repository (at git root)               │
├─────────────────────────────────────────────────────────┤
│  .github/agents/                                        │
│    ├── ralph-implementer.agent.md  (copied or skipped)  │
│    └── ralph-planner.agent.md      (copied or skipped)  │
│  .githooks/commit-msg                                   │
│  ralph/validation.json                                  │
│  ralph/tasks/                                           │
│  docs/ralph/                                            │
└─────────────────────────────────────────────────────────┘
```

### Implementation Constraints

⚠️ **CRITICAL**: When updating init.rs embedded template constants:
- The `ralph-implementer.agent.md` content MUST include the validation feedback loop guidance
- Look for: "⚠️ PREVIOUS ITERATION FAILED VALIDATION" detection instructions
- This is essential for the Ralph outer loop to communicate failures to the agent
- The authoritative template in `templates/.github/agents/ralph-implementer.agent.md` contains all required content

### Final Requirements Summary

| ID | Title | Status |
|----|-------|--------|
| REQ-01 | Bundle agent templates in Nix package | done |
| REQ-02 | Wrap binary with RALPH_SHARE_DIR | done |
| REQ-03 | Init reads from RALPH_SHARE_DIR with dev fallback | done |
| REQ-04 | Init copies to git root | done |
| REQ-05 | Skip existing files silently | done |
| REQ-06 | Success message | done |
| REQ-07 | End-to-end validation: build and init in fresh repo | done |

**REQ-07 Acceptance Criteria:**
- `nix build .#` completes successfully
- `ralph init` succeeds when run in a fresh git repository
- Initialized repo contains no hard-coded Rust-specific instructions (e.g., `cargo fmt`, `cargo clippy` in agent files)
- Agent templates in initialized repo are language-agnostic, referring to validation profiles rather than specific toolchains

Planning complete. Ready for implementation.
<!-- RALPH:END PLANNING_LOG -->
