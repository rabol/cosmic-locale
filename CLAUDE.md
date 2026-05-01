# CLAUDE.md — cosmic-locale

Project context and rules for AI coding agents working on this repo.
Read this fully before making changes.

## What we're building

A **COSMIC desktop application** for managing system language and locale
variables on Linux. The app replicates the *functionality* of
[MX-Linux/mx-locale](https://github.com/MX-Linux/mx-locale) (a Qt/C++ GUI
that wraps `update-locale` and `dpkg-reconfigure locales`), but as a
native libcosmic application written in Rust.

This is **not a port**. mx-locale's C++/Qt code is a *specification* of
what features the app needs — what the user can do, what commands run
under the hood. The implementation here is a fresh libcosmic app.

## Stack

- **Rust** (stable toolchain)
- **libcosmic** — UI framework, see <https://github.com/pop-os/libcosmic>
- **iced** runtime (libcosmic builds on iced)
- **fluent** for i18n via `i18n_embed` and `i18n_embed_fl` (configured by `i18n.toml`)
- **just** for build/run/check recipes (see `justfile`)
- Project generated from <https://github.com/pop-os/cosmic-app-template>

## Authoritative references — consult, don't guess

libcosmic is pre-1.0 and its API changes. Your training data may be
stale or wrong about exact signatures, trait bounds, and widget APIs.

When unsure about a libcosmic widget, message pattern, theme token, or
trait, **read the source** rather than guessing:

- API docs: <https://pop-os.github.io/libcosmic/cosmic/>
- Book (architectural patterns): <https://pop-os.github.io/libcosmic-book/>
- Source: <https://github.com/pop-os/libcosmic>
- Reference apps for idiomatic usage:
  - <https://github.com/pop-os/cosmic-files>
  - <https://github.com/pop-os/cosmic-edit>
  - <https://github.com/pop-os/cosmic-settings> (closest analogue — settings-style app)

If a `cargo check` error mentions a libcosmic type or trait you're
unsure about, look at how `cosmic-settings` uses it before iterating
on guesses.

## Build, run, check

The repo uses `just`. Always prefer `just` recipes over raw cargo:

- `just` — build release
- `just run` — build and run the app
- `just check` — clippy
- `just check-json` — for IDE/LSP
- `just install` — system install
- `just vendor` / `just build-vendored` — vendored deps

Only invoke `cargo` directly when no `just` recipe covers the need
(e.g. `cargo add <crate>`, `cargo update`).

## Privilege model

Changing system locale requires root. Two acceptable approaches:

1. **MVP**: invoke privileged commands via `pkexec` from the app.
2. **Proper**: ship a polkit policy file (`org.cosmic.locale.policy`)
   and a small helper binary that performs the actual `update-locale`
   / `dpkg-reconfigure locales` work.

Rules:

- **Never** run the GUI itself as root.
- **Never** embed or log passwords.
- Surface privilege failures (user cancels pkexec, polkit denial) to
  the UI clearly — never silently swallow them.
- Audit every shelled-out command before adding it; treat any string
  that gets concatenated into a shell command as a security review.

## Workflow rules for the agent

- **Don't port mx-locale's code.** Read its UI files
  (`mainwindow.ui`, `mainwindow.cpp`, `choosedialog.cpp`, `cmd.cpp`)
  to understand the *behavior*, then implement the COSMIC equivalent
  using idiomatic libcosmic patterns.
- **Stop at milestone boundaries.** When a planned milestone is
  complete, stop, summarize the changes, and wait for the user to run
  `just run` and confirm before continuing. Don't chain milestones.
- **Plan before coding for new features.** For non-trivial work,
  produce a short plan (Message variants, view structure, async tasks
  needed) and get sign-off before writing implementation code.
- **Keep diffs reviewable.** Prefer many small, focused commits over
  large sweeping changes.

## Code style and quality

### Type system and errors

- **Never** use `.unwrap()` in production code paths.
- Use `.expect("descriptive message")` only for true invariant
  violations that should crash if violated.
- Use `Result<T, E>` for fallible operations; propagate with `?`.
- Use `thiserror` for library/internal error types.
- Use `anyhow` for application-level error context (`.context("...")`).
- Prefer `Option<T>` over sentinel values.
- Use newtypes to distinguish semantically different values that
  share an underlying type (e.g. `LocaleCode(String)` vs raw `String`).

### Functions and types

- Single responsibility per function and per type.
- Prefer borrowing (`&T`, `&mut T`) over taking ownership when the
  caller doesn't need to give it up.
- Limit function parameters to ~5; use a config struct beyond that.
- Return early to reduce nesting.
- Derive `Debug`, `Clone`, `PartialEq` where appropriate.
- Use `#[derive(Default)]` when a sensible default exists.
- Use the builder pattern for complex construction.
- Make struct fields private by default; add accessors when needed.

### Memory

- Prefer `&str` over `String` when ownership isn't required.
- Use `Cow<'_, str>` when ownership is conditional.
- Use `Vec::with_capacity()` when the size is known.
- Use `Arc`/`Rc` deliberately, not reflexively.

### Concurrency

- Long-running work (running `update-locale`, parsing `locale -a`
  output) must not block the UI thread. Use the iced/libcosmic async
  task system — return a `Command` / `Task` from `update`, not a
  blocking call.
- Apply `Send`/`Sync` bounds where the runtime requires them.

### Style

- 4 spaces for indentation, never tabs.
- 100-char line limit (rustfmt default).
- `snake_case` for fns/vars/modules, `PascalCase` for types/traits,
  `SCREAMING_SNAKE_CASE` for constants.
- Avoid wildcard imports except for preludes and `use super::*;`
  inside `#[cfg(test)]` modules.
- Organize imports: std, external crates, local modules.

### Documentation

- Doc comments on all public items (functions, structs, enums, methods).
- Document parameters, return values, and errors.
- Include examples in doc comments for non-obvious functions.

Example:

```rust
/// Apply a new system locale by invoking `update-locale` via pkexec.
///
/// # Arguments
///
/// * `locale` - Validated locale code (e.g. "en_US.UTF-8")
///
/// # Errors
///
/// Returns `LocaleError::PrivilegeDenied` if the user cancels the
/// pkexec prompt, or `LocaleError::CommandFailed` if `update-locale`
/// exits non-zero.
pub async fn apply_locale(locale: &LocaleCode) -> Result<(), LocaleError> {
    // ...
}
```

### Testing

- Unit tests for parsing logic (e.g. parsing `locale -a` output,
  `/etc/default/locale`) and any pure logic.
- Use `#[cfg(test)]` modules.
- Mock or abstract shell-outs behind a trait so tests don't depend on
  the host's actual locale configuration.
- Don't commit commented-out tests.

### Logging

- Use `tracing` (or `log`) — never `println!` for diagnostics.
- Use `tracing::error!` for failure paths, `tracing::info!` for
  significant state changes.
- Never log secrets or full command lines that may contain user data.

## Dependencies

- Document version constraints in `Cargo.toml`.
- Don't add a dependency to solve something the standard library or
  libcosmic already covers cleanly.
- When adding a crate, briefly justify it in the commit message.

## Tools and gates

Before considering work "done":

- `just check` passes (clippy clean).
- `cargo fmt --check` passes.
- `just run` launches the app and the changed feature behaves as
  expected (manual smoke test — describe what was tested).
- No new compiler warnings.
- No commented-out code, stray `dbg!`, or debug `println!`.
- No secrets, paths from your machine, or PII in committed files.

## Internationalization

- All user-facing strings go through fluent (`i18n_embed_fl::fl!`).
- Add new keys to `i18n/en/<crate>.ftl`.
- Don't hardcode English strings in `view` code.

## Git hygiene

- Clear, descriptive commit messages (imperative mood: "Add locale
  picker", not "Added locale picker").
- One logical change per commit.
- Never commit credentials, `.env`, or local-only paths.

---

**Priorities, in order:** correctness, idiomatic libcosmic usage,
clarity, then performance. This app sets a locale on a button click —
optimize for getting the libcosmic API right and the privilege flow
secure, not for SIMD or parallelism.
