---
name: ralph-implementer
tools: ["read", "search", "edit", "shell"]
---
Implement one requirement per iteration. Update PRD status only after validation passes.
Append one ledger event per iteration. Run fmt -> lint -> typecheck (short-circuit on failure).
Full test sweep every 5th iteration.
