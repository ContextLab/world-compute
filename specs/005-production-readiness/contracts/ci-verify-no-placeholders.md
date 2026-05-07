# Contract: `scripts/verify-no-placeholders.sh` + allowlist format

**Scope**: The hard-blocking CI check for placeholder elimination (FR-038, SC-006).

## Script interface

`bash scripts/verify-no-placeholders.sh [--list] [--check-empty]`

- No args: scan and exit 0/64 (fail if any match not in allowlist).
- `--list`: print every match with its allowlist-membership status, exit 0.
- `--check-empty`: additionally assert `.placeholder-allowlist` has zero non-comment lines. **This mode is the spec-005-completion gate.**

## Tokens searched

Regex (case-insensitive): `\b(placeholder|stub|TODO|todo!|unimplemented!)\b`

## Paths scanned

- `src/**/*.rs` — always
- `adapters/**/src/**/*.rs` — always
- `gui/src-tauri/src/**/*.rs` — always
- `proto/**/*.proto` — always

## Paths NOT scanned

- `tests/**` — tests may use `todo!()` or `#[ignore]` with documentation explaining why
- `docs/**` — docs may freely reference historic placeholders
- `specs/**` — this spec itself contains the word "placeholder" by design
- `scripts/**` — may mention placeholders in the script that finds them
- `evidence/**` — evidence artifacts may mention historic placeholders
- `.placeholder-allowlist` — the allowlist itself

## Allowlist file format

`.placeholder-allowlist` at repository root. One entry per non-empty non-comment line:

```
# Comments start with # and are ignored.
src/some_file.rs:42 — brief rationale for why this placeholder reference must remain
```

Fields separated by `:` for path/line and ` — ` (space + em-dash + space) for rationale.

## Exit codes

- `0` — zero matches outside allowlist; `--check-empty` also passes if active
- `64` — at least one match in scanned paths is not in the allowlist
- `65` — `--check-empty` requested and allowlist has ≥ 1 non-comment entry

## CI integration

`.github/workflows/verify-no-placeholders.yml`:

- On every PR and push: run without flags. Fail on non-zero.
- On `005-production-readiness` branch + on each merge to `main`: run with `--check-empty`. Fail on non-zero.
- Do NOT run with `--check-empty` on long-term `main` after spec 005 closes — an empty allowlist is the completion gate, not a permanent enforcement policy.

## Edge cases

- A placeholder token appearing inside a string literal (e.g., test fixture data) in `src/` IS flagged. The fix is to move the fixture to `tests/` or to use a different sentinel string like `"SENTINEL_VALUE_FOR_TEST"`.
- A placeholder token inside `#[cfg(test)]`-gated code in `src/` IS flagged. The fix is to move the test to `tests/`.

## Relationship to constitution

- Principle V (Direct Testing) is strengthened: every placeholder removed reveals either a real implementation (confirmed safe by tests) or a missing implementation that must be filled in.
- The allowlist mechanism itself is a constitution-compatible way to document legitimate historic-context references in doc-comments; it is NOT a loophole for unfinished work.
