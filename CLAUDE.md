# bookcli — project conventions

A Rust CLI built as a **6-branch learning progression** (see `SPEC.md`). This is a
**hybrid-learning** project: I scaffold each branch, Bob writes the logic with Copilot.

These rules are scoped to this repo and **override the global `~/.claude/CLAUDE.md`**
where they conflict (notably the "no inline comments" / "self-documenting code" rules).

## Hybrid-learning workflow (per branch)

For each branch I produce **stubs + commented tests**, not finished code:

1. Write the branch's tests first, **fully implemented and failing** (from `SPEC.md`).
2. Add the types and function signatures the tests need.
3. Leave function bodies as `todo!("<hint>")` — Bob fills them in with Copilot.
4. Above each stub, write a `///` or `//` comment laying out the *approach*
   (the steps / the idiom to reach for), not the literal code.

Then Bob implements to green. I do **not** write the bodies unless asked.

## Comments — verbose on purpose

Override the global no-comments rule **for `src/` learning code**:

- Explain the *approach* and *why an idiom is used* (e.g. "`?` propagates the parse
  error up to `main`'s `anyhow` boundary"), so Bob can code fast with Copilot.
- Teaching comments are encouraged in `src/`. Keep **tests** clean (minimal comments).
- Still no sycophantic filler, no "this function does..." restatements — comments
  add information, they don't narrate.

## TDD-first

Tests come before implementation, always. Each branch's required tests are listed in
`SPEC.md`. Unit tests never touch the network or real disk:

- Storage tests use `InMemoryRepository` or a temp file via `BOOKCLI_STORE`.
- Search tests use `FakeSearch` with canned responses / JSON fixtures.

## One branch at a time

- Work the current branch only; don't implement future branches ahead.
- A branch ends **green** before merge — see the verification loop below.
- Prefer the *teachable* idiom over the clever one; Rust fluency is a goal here.

## Update the README each branch

When a branch adds a command, update `README.md`: new usage example + a line on what
it does. The README should always reflect what the CLI can do *today*.

## Verification loop (run before every commit/merge)

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

Cite what was run and the outcome.

## Naming

Crate `bookcli`, binary `book` (commands read `book search ...`).
Commit format: conventional commits (`feat:`, `fix:`, `docs:`, `test:`, ...).
