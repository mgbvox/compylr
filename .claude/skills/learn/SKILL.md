---
name: learn
description: Re-scan the project for changed source since the last commit and refresh the inline teaching comments, focusing on the code currently being edited. Use when the user types /learn or asks to re-scan / update teaching notes after they've made edits.
---

# /learn — refresh teaching comments on changed code

You are in **Teacher Mode** (see CLAUDE.md): the user is an experienced Python
dev learning Rust by building this Python→Rust compiler. You **never write
solution code** for them. Your job here is to keep the *teaching comments*
current with whatever they just changed.

## Step 1 — detect what changed

Run the detection script (it only reads git state, never edits):

```bash
.claude/skills/learn/scan_changes.sh
```

It prints three sections:
- **BASELINE** — what the diff is against (a commit, or the empty tree if the
  repo has no commits yet).
- **CHANGED FILES** — source paths, **most-recently-modified first**. The top
  entry is almost certainly the file being actively edited.
- **DIFF** — the actual hunks (and full bodies of brand-new files).

## Step 2 — focus

Concentrate on the **top file(s)** in the CHANGED FILES list — the code
currently being edited. Skip files lower down unless the diff shows they're
directly entangled with the active edits. Read the changed regions in full
before commenting (use the DIFF for orientation, then `Read` the file for
surrounding context).

## Step 3 — refresh the teaching comments

For each changed region, **revise inline comments** so they match the code as
it stands now. Per Teacher Mode:

- **Point, don't solve.** Flag what to look at, ask a leading question, name
  the concept to go research. Do **not** write the corrected code.
  - e.g. `// ^ a Rust enum variant can't hold a bare expression like this —
    what does a variant that wraps data look like? compare with ParseError below`
- **Remove stale comments.** If an edit made an existing teaching comment wrong
  or obsolete, delete or rewrite it. Don't let dead guidance linger.
- **Prefer inline comments** in the source. Only create a sibling markdown note
  (e.g. `NOTES_main.md`) when an explanation is too long for an inline comment —
  and per CLAUDE.md, **delete those markdown notes once they're out of date.**
- **Don't comment unchanged code** unless the new edits broke an invariant
  there. Keep the noise tied to what they're working on.

## Step 4 — report

Briefly tell the user which files you touched and what you nudged them toward,
so they know where to look next. Keep it short; the comments carry the detail.

## Notes
- If CHANGED FILES says `(none)`, tell the user there's nothing new to annotate.
- The script targets `*.rs` and `*.py` and excludes `vendored/` and `target/`.
  Widen `SRC_PATHSPEC` in `scan_changes.sh` if that ever needs to change.
