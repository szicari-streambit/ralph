# no rust

Canonical machine PRD: ralph/tasks/no-rust/prd.json

<!-- RALPH:BEGIN PLANNING_LOG -->

## Planning Log

### 2026-01-23 20:59 - Issue Identification

**User Concern:** The PRD(s) may mention rust, but I don't want that because rust is only for THIS repo. Where might rust sneak into future prd planning?

**Root Cause Analysis:**

1. **File:** `crates/ralph-cli/src/commands/plan.rs`, Line 88
   - `create_initial_prd()` hardcodes `validation_profiles: vec!["rust-cargo".to_string()]`
   - This means EVERY new PRD created by `ralph plan <feature>` gets Rust validation profiles by default
   - This is incorrect because Ralph should work with ANY language/project

2. **Current State:**
   - All existing PRDs have `"rust-cargo"` validation profile:
     - `pre-commit/prd.json` - line 7 ✅ (correct - this IS about Rust pre-commit hooks)
     - `stateless-execution/prd.json` - line 7 ❌ (should be `"nix-flake"` only, and it is!)
     - `no-more-commit-hook/prd.json` - line 7 ✅ (correct - modifying Rust hooks)
     - `no-rust/prd.json` - line 7 ❌ (ironic - "no rust" has rust-cargo profile!)

3. **Impact:**
   - When users run `ralph plan <feature>` in a Python/Node/Go/etc project, they get Rust validation
   - This will cause validation to fail or run incorrect checks
   - Violates the language-agnostic design goal from REQ-07 of stateless-execution

**Recommendation:**

Change the default `validation_profiles` in `create_initial_prd()` to an empty array `[]` or make it configurable.

### Questions for User (Round 1)

Before drafting requirements, I need clarification:

1. **Default behavior:** Should `ralph plan` create PRDs with:
   - Empty validation profiles `[]` (user must specify)?
   - Auto-detect based on files in target repo (e.g., check for Cargo.toml, package.json, requirements.txt)?
   - Accept a CLI flag like `ralph plan --profile rust-cargo my-feature`?

2. **Existing PRD migration:** Should we update the `no-rust/prd.json` to remove `"rust-cargo"` from validation profiles?

3. **Scope of changes:**
   - Just fix the hardcoded default in `plan.rs`?
   - Add auto-detection logic?
   - Add validation to warn if profiles don't match detected project type?

4. **Documentation:** Should we document validation profile selection in the README or user guide?

5. **Backward compatibility:** Do we need to support existing PRDs that might rely on the rust-cargo default?

### User Answers (2026-01-23 21:21)

1. **Auto-detect with fallback to empty** - Need to discuss implementation details
2. **No fix needed** - This repo (ralph-spec-v2-complete) requires Rust validation for code changes
3. **Scope:** Implement auto-detection logic, validation warnings anticipated
4. **Documentation:** Want to see preview of profile selection behavior documentation
5. **Backward compatibility:** No concerns

### Anticipated Validation Warnings

Based on auto-detection approach, I anticipate these warning scenarios:

1. **Profile Mismatch Warning:**
   - PRD specifies `rust-cargo` but `Cargo.toml` not found
   - PRD specifies `nix-flake` but `flake.nix` not found
   - Warning: "Validation profile 'X' specified but detection files not found"

2. **Multiple Profiles Detected:**
   - Both `Cargo.toml` and `package.json` exist (polyglot repo)
   - Warning: "Multiple project types detected, using profiles: [rust-cargo, node-npm]"

3. **No Profiles Detected:**
   - No recognized project files found
   - Info: "No validation profiles auto-detected, PRD created with empty profiles []"

4. **Profile Not Defined:**
   - Auto-detect finds `package.json` but `node-npm` profile doesn't exist in `validation.json`
   - Warning: "Detected project type 'node-npm' but profile not defined in validation.json"

### Proposed Documentation Preview

```markdown
## Validation Profiles

Ralph uses validation profiles to run language/framework-specific checks during implementation.

### Auto-Detection

When you run `ralph plan <feature>`, Ralph automatically detects validation profiles based on files in your repository:

| File Detected | Profile Applied |
|---------------|-----------------|
| `Cargo.toml` | `rust-cargo` |
| `flake.nix` | `nix-flake` |
| `package.json` | `node-npm` *(future)* |
| `requirements.txt` | `python-pip` *(future)* |

If no project files are detected, the PRD is created with empty validation profiles `[]`.

### Manual Override

You can explicitly specify profiles using the `--profile` flag:

```bash
ralph plan --profile rust-cargo my-feature
ralph plan --profile nix-flake,rust-cargo polyglot-feature
```

### Defining Custom Profiles

Profiles are defined in `ralph/validation.json`. Each profile specifies:
- **detect**: File patterns to auto-detect the profile
- **commands**: Commands to run for fmt, lint, typecheck, test
```

### Questions for User (Round 2) - Auto-Detection Design

6. **Detection scope:** Should auto-detect scan:
   - Only the current working directory?
   - Walk up to find git root, then scan from there?
   - Scan recursively through subdirectories?

7. **Multiple profiles:** If both `Cargo.toml` and `flake.nix` exist (this repo!), should:
   - Include both profiles `["rust-cargo", "nix-flake"]`?
   - Prioritize one over the other?
   - Let user choose via interactive prompt?

8. **Profile definition source:** Should auto-detect:
   - Only detect profiles already defined in `ralph/validation.json`?
   - Use hardcoded detection rules (e.g., Cargo.toml → rust-cargo)?
   - Use the `detect` field from `validation.json` profiles (like the existing `anyFilesExist`)?

9. **CLI flag behavior:** If `--profile` is specified:
   - Completely override auto-detection?
   - Append to auto-detected profiles?
   - Validate that specified profile exists in validation.json?

10. **Future extensibility:** Should we design for:
    - Plugin-based profile detection?
    - User-defined detection rules in config file?
    - Or keep it simple with hardcoded rules for now?

### User Answers (2026-01-23 21:29)

6. **Git root detection scope** - Scan from git root (monorepo concerns deferred to later)
7. **Include all necessary profiles** - If changes cross languages, include all relevant profiles for validation
8. **Profile detection source** - Need clarification (explanation requested below)
9. **CLI flag behavior** - Not sure yet
10. **Future extensibility** - Keep it simple, no plugins/user config for now

### Profile Detection Source - Detailed Explanation

There are three approaches for how auto-detection determines which profiles to use:

#### Option A: Hardcoded Detection Rules in Code
```rust
// In plan.rs
fn detect_profiles(repo_root: &Path) -> Vec<String> {
    let mut profiles = Vec::new();
    
    if repo_root.join("Cargo.toml").exists() {
        profiles.push("rust-cargo".to_string());
    }
    if repo_root.join("flake.nix").exists() {
        profiles.push("nix-flake".to_string());
    }
    if repo_root.join("package.json").exists() {
        profiles.push("node-npm".to_string());
    }
    
    profiles
}
```

**Pros:** Simple, fast, no file parsing
**Cons:** Adding new profiles requires code changes + recompilation

#### Option B: Use validation.json detect rules
```rust
// Read ralph/validation.json and use the "detect" field
// Current validation.json has:
{
  "profiles": {
    "rust-cargo": {
      "detect": { "anyFilesExist": ["Cargo.toml"] },
      "commands": { ... }
    }
  }
}

// Code would:
// 1. Load validation.json
// 2. For each profile, check if detect conditions match
// 3. Add matching profiles
```

**Pros:** User can add new profiles without recompiling Ralph
**Cons:** Requires validation.json to exist, more complex detection logic

#### Option C: Hybrid Approach
```rust
// Try to load validation.json first
// If it exists, use detect rules from there
// Otherwise, fall back to hardcoded detection for common types
```

**Pros:** Works in repos without validation.json, extensible when it exists
**Cons:** Most complex implementation

**Current State:** The existing `ralph/validation.json` already has a `detect` field for `rust-cargo`!

### Questions for User (Round 3) - Implementation Details

11. **Profile detection source decision:** Based on the explanation above:
    - Option A (hardcoded rules)?
    - Option B (use validation.json detect rules)?
    - Option C (hybrid with fallback)?

12. **Behavior when validation.json missing:** If a user runs `ralph plan` in a fresh repo without `ralph/validation.json`:
    - Auto-detect using hardcoded rules?
    - Fail with error "Run 'ralph init' first"?
    - Create empty profiles and warn user?

13. **CLI flag validation:** If user specifies `ralph plan --profile nonexistent-profile`:
    - Fail with error?
    - Warn but continue?
    - Silently accept and let validation fail later?

14. **Profile name standardization:** Should we enforce naming conventions?
    - Lowercase with hyphens (rust-cargo, node-npm)?
    - Any string allowed?
    - Reserve certain prefixes (e.g., "custom-*")?

15. **Verbose output:** Should `ralph plan` with verbose flag show:
    - Which files were checked during detection?
    - Why each profile was selected/rejected?
    - Final list of profiles being used?

<!-- RALPH:END PLANNING_LOG -->
