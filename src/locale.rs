// SPDX-License-Identifier: GPL-3.0-only

//! Locale model — types, parsing, and async readers/writers for the
//! system locale configuration.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;
use zbus::Connection;
use zbus::proxy;

/// Validated locale identifier (e.g. `"en_US.UTF-8"`, `"C"`, `"POSIX"`).
///
/// Construction goes through [`LocaleCode::new`], which performs minimal
/// shape validation (non-empty, no whitespace, no control characters).
/// Stricter validation against the system's installed locales happens at
/// the point where the value is actually applied, not here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocaleCode(String);

impl LocaleCode {
    /// Construct a `LocaleCode` from a string, returning `None` if the value
    /// is empty after trimming or contains whitespace / control characters.
    #[must_use]
    pub fn new(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.is_empty() {
            return None;
        }
        if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return None;
        }
        Some(Self(value.to_string()))
    }

    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LocaleCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Parsed system-wide locale configuration.
///
/// Mirrors the shape of `/etc/default/locale` or `/etc/locale.conf`: an
/// optional primary `LANG`, an optional `LANGUAGE` fallback list (a
/// colon-separated list, not itself a single locale), and any number of
/// per-category `LC_*` overrides.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocaleSettings {
    pub lang: Option<LocaleCode>,
    pub language: Option<String>,
    pub lc_overrides: BTreeMap<String, LocaleCode>,
}

/// Where the loaded settings came from.
///
/// Knowing this lets the UI display a meaningful description and refuse
/// to "reset" anything when the values came from the process
/// environment (which we can't authoritatively change).
#[derive(Debug, Clone)]
pub enum LocaleSource {
    File(PathBuf),
    Environment,
}

/// A successful read of the system locale plus the source it came from.
#[derive(Debug, Clone)]
pub struct LoadedLocale {
    pub settings: LocaleSettings,
    pub source: LocaleSource,
}

/// Where a category's effective value comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategorySource {
    /// Set explicitly via an `LC_*=` line in the config or the
    /// process environment.
    Override,
    /// No explicit setting; takes its value from `LANG`.
    Inherited,
    /// Neither `LANG` nor an explicit override is set; falls back to
    /// the POSIX `C` locale.
    Default,
}

/// Resolved view of a single `LC_*` category.
#[derive(Debug, Clone)]
pub struct CategoryView {
    pub name: &'static str,
    pub value: String,
    pub source: CategorySource,
}

/// The full set of POSIX `LC_*` categories, in the order glibc and
/// `localectl status` print them. `LC_ALL` is intentionally omitted —
/// it's a runtime override that isn't stored in the locale config.
pub const LC_CATEGORIES: &[&str] = &[
    "LC_CTYPE",
    "LC_NUMERIC",
    "LC_TIME",
    "LC_COLLATE",
    "LC_MONETARY",
    "LC_MESSAGES",
    "LC_PAPER",
    "LC_NAME",
    "LC_ADDRESS",
    "LC_TELEPHONE",
    "LC_MEASUREMENT",
    "LC_IDENTIFICATION",
];

/// Resolve every `LC_*` category against the parsed settings.
///
/// For each category, the effective value is the explicit override if
/// present, otherwise `LANG`'s value, otherwise the POSIX `C` default.
#[must_use]
pub fn effective_categories(settings: &LocaleSettings) -> Vec<CategoryView> {
    LC_CATEGORIES
        .iter()
        .map(|&name| {
            if let Some(code) = settings.lc_overrides.get(name) {
                CategoryView {
                    name,
                    value: code.as_str().to_string(),
                    source: CategorySource::Override,
                }
            } else if let Some(lang) = &settings.lang {
                CategoryView {
                    name,
                    value: lang.as_str().to_string(),
                    source: CategorySource::Inherited,
                }
            } else {
                CategoryView {
                    name,
                    value: "C".to_string(),
                    source: CategorySource::Default,
                }
            }
        })
        .collect()
}

/// Errors that can occur while loading or applying the system locale.
#[derive(Debug, Clone, thiserror::Error)]
pub enum LocaleError {
    /// The configured locale config file existed but could not be read.
    #[error("failed to read {path}: {source}")]
    ReadConfig {
        path: PathBuf,
        #[source]
        source: Arc<std::io::Error>,
    },

    /// The polkit prompt was dismissed or authorisation was denied.
    #[error("authorisation was cancelled or denied")]
    Cancelled,

    /// systemd-localed (or the system bus) returned an error we don't
    /// recognise as a polkit cancellation.
    #[error("locale daemon error: {0}")]
    Daemon(String),

    /// A non-privileged subprocess (e.g. `locale -a`) failed to launch.
    #[error("failed to launch {command}: {source}")]
    CommandSpawnFailed {
        command: String,
        #[source]
        source: Arc<std::io::Error>,
    },

    /// A non-privileged subprocess exited non-zero.
    #[error("{command} exited with status {status}: {stderr}")]
    CommandFailed {
        command: String,
        status: i32,
        stderr: String,
    },
}

/// System paths checked for locale configuration, in priority order.
///
/// `/etc/default/locale` is the Debian/Ubuntu/Pop!_OS convention;
/// `/etc/locale.conf` is the systemd standard used by Arch, Fedora, etc.
const LOCALE_CONFIG_PATHS: &[&str] = &["/etc/default/locale", "/etc/locale.conf"];

/// Read the system locale configuration.
///
/// Tries each known config file in order; if a file is missing the next
/// one is attempted. If none of the files exist, falls back to reading
/// the current process environment (`LANG`, `LANGUAGE`, `LC_*`) and
/// reports the source as [`LocaleSource::Environment`].
///
/// # Errors
///
/// Returns [`LocaleError::ReadConfig`] only if a file exists but cannot
/// be read (e.g. permission denied). Missing files are not errors.
pub async fn read_default_locale() -> Result<LoadedLocale, LocaleError> {
    for path in LOCALE_CONFIG_PATHS {
        match tokio::fs::read_to_string(path).await {
            Ok(contents) => {
                return Ok(LoadedLocale {
                    settings: parse_default_locale(&contents),
                    source: LocaleSource::File(PathBuf::from(path)),
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(LocaleError::ReadConfig {
                    path: PathBuf::from(path),
                    source: Arc::new(err),
                });
            }
        }
    }

    Ok(LoadedLocale {
        settings: read_locale_from_env(),
        source: LocaleSource::Environment,
    })
}

/// Parse the contents of `/etc/default/locale` or `/etc/locale.conf`.
///
/// The format is shell-style `KEY=value`, optionally prefixed with
/// `export`, optionally with surrounding single or double quotes around
/// the value. Comment lines (starting with `#`) and blank lines are
/// skipped. Unrecognised keys are ignored. Values that fail
/// [`LocaleCode`] validation are silently dropped — the parser is
/// deliberately lenient so one bad entry doesn't blank the whole view.
#[must_use]
pub fn parse_default_locale(input: &str) -> LocaleSettings {
    let mut settings = LocaleSettings::default();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line).trim();

        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = strip_quotes(raw_value.trim());

        match key {
            "LANG" => settings.lang = LocaleCode::new(value),
            "LANGUAGE" if !value.is_empty() => {
                settings.language = Some(value.to_string());
            }
            k if k.starts_with("LC_") => {
                if let Some(code) = LocaleCode::new(value) {
                    settings.lc_overrides.insert(k.to_string(), code);
                }
            }
            _ => {}
        }
    }

    settings
}

fn read_locale_from_env() -> LocaleSettings {
    let mut settings = LocaleSettings::default();

    if let Ok(value) = std::env::var("LANG")
        && let Some(code) = LocaleCode::new(&value)
    {
        settings.lang = Some(code);
    }

    if let Ok(value) = std::env::var("LANGUAGE")
        && !value.is_empty()
    {
        settings.language = Some(value);
    }

    for (key, value) in std::env::vars() {
        if !key.starts_with("LC_") {
            continue;
        }
        if let Some(code) = LocaleCode::new(&value) {
            settings.lc_overrides.insert(key, code);
        }
    }

    settings
}

/// Talk to systemd-localed over the system D-Bus.
///
/// `Locale` is a property exposed as an array of `KEY=value` strings.
/// `SetLocale` accepts the *complete* desired state — anything not in
/// the array is removed — so we clear `LC_*` entries by passing
/// everything *except* them. The second argument requests
/// interactive polkit authorisation (i.e. the user gets prompted).
#[proxy(
    interface = "org.freedesktop.locale1",
    default_service = "org.freedesktop.locale1",
    default_path = "/org/freedesktop/locale1"
)]
trait Locale1 {
    fn set_locale(&self, locale: &[&str], interactive: bool) -> zbus::Result<()>;

    #[zbus(property)]
    fn locale(&self) -> zbus::Result<Vec<String>>;
}

/// Clear all `LC_*` overrides from the system locale via the
/// `org.freedesktop.locale1` D-Bus interface.
///
/// Reads the current locale array from systemd-localed, drops every
/// `LC_*` entry, and calls `SetLocale` with the filtered array.
/// systemd-localed handles the polkit auth and the file write.
///
/// # Errors
///
/// - [`LocaleError::Cancelled`] if the user dismissed the polkit
///   prompt or authorisation was denied.
/// - [`LocaleError::Daemon`] if the system bus is unreachable or
///   systemd-localed returned any other error.
pub async fn reset_lc_overrides() -> Result<(), LocaleError> {
    let conn = Connection::system()
        .await
        .map_err(|e| zbus_to_locale_error(&e))?;

    let proxy = Locale1Proxy::new(&conn)
        .await
        .map_err(|e| zbus_to_locale_error(&e))?;

    let current = proxy.locale().await.map_err(|e| zbus_to_locale_error(&e))?;
    let new_locale = build_reset_locale(&current)?;
    let new_locale_strs: Vec<&str> = new_locale.iter().map(String::as_str).collect();

    proxy
        .set_locale(&new_locale_strs, true)
        .await
        .map_err(|e| zbus_to_locale_error(&e))?;

    Ok(())
}

/// Set a single `LC_*` category to the given locale value via systemd-localed.
///
/// Reads the current `Locale` array, replaces (or appends) the entry
/// matching `category`, and submits the result through `SetLocale`.
/// Other variables stay untouched.
///
/// # Errors
///
/// See [`reset_lc_overrides`] — the same set of conditions apply.
pub async fn set_category(category: &str, value: &str) -> Result<(), LocaleError> {
    let conn = Connection::system()
        .await
        .map_err(|e| zbus_to_locale_error(&e))?;
    let proxy = Locale1Proxy::new(&conn)
        .await
        .map_err(|e| zbus_to_locale_error(&e))?;

    let current = proxy.locale().await.map_err(|e| zbus_to_locale_error(&e))?;
    let new_locale = build_category_set(&current, category, value);
    let new_locale_strs: Vec<&str> = new_locale.iter().map(String::as_str).collect();

    proxy
        .set_locale(&new_locale_strs, true)
        .await
        .map_err(|e| zbus_to_locale_error(&e))?;

    Ok(())
}

/// Replace (or append) a single category in the locale array.
///
/// If `category=` already appears in `current`, its value is replaced;
/// otherwise the new entry is appended. All other entries pass through
/// untouched and in the same order.
fn build_category_set(current: &[String], category: &str, value: &str) -> Vec<String> {
    let prefix = format!("{category}=");
    let new_entry = format!("{category}={value}");

    let mut out = Vec::with_capacity(current.len().max(1));
    let mut replaced = false;
    for entry in current {
        if entry.starts_with(&prefix) {
            out.push(new_entry.clone());
            replaced = true;
        } else {
            out.push(entry.clone());
        }
    }
    if !replaced {
        out.push(new_entry);
    }
    out
}

/// A small set of human-readable samples for a single locale, used by
/// the picker's preview panel. Anything we can't compute is left blank.
#[derive(Debug, Clone)]
pub struct LocalePreview {
    pub locale: String,
    pub date: String,
    pub time: String,
    pub datetime: String,
    pub number: String,
}

/// Compute a [`LocalePreview`] by shelling out to `date` and `printf`
/// with `LC_ALL` set to the chosen locale code. Failed sub-calls
/// degrade gracefully to empty strings.
pub async fn preview_locale(code: String) -> LocalePreview {
    let date = run_with_locale(&code, "date", &["+%x"]).await;
    let time = run_with_locale(&code, "date", &["+%X"]).await;
    let datetime = run_with_locale(&code, "date", &["+%c"]).await;
    let number = run_with_locale(&code, "/usr/bin/printf", &["%'d", "1234567"]).await;
    LocalePreview {
        locale: code,
        date,
        time,
        datetime,
        number,
    }
}

async fn run_with_locale(locale: &str, program: &str, args: &[&str]) -> String {
    tokio::process::Command::new(program)
        .env("LC_ALL", locale)
        .args(args)
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Run `locale -a` and return the parsed list of installed locales.
///
/// Pseudo-locales (`C`, `POSIX`) are filtered out, as are non-UTF-8
/// entries — matching the convention cosmic-settings uses for its
/// region picker.
///
/// # Errors
///
/// Returns [`LocaleError::CommandSpawnFailed`] if `locale` cannot be
/// launched, or [`LocaleError::CommandFailed`] on a non-zero exit.
/// Path to the locale-gen configuration file edited by the locale
/// management page.
const LOCALE_GEN_PATH: &str = "/etc/locale.gen";

/// Privileged helper invoked via `pkexec` to write `/etc/locale.gen`
/// and run `locale-gen`. Installed by `just install` from
/// `resources/apply-locale-gen`. Pkexec resolves this path to our
/// named polkit action via the `org.freedesktop.policykit.exec.path`
/// annotation in the shipped `.policy` file.
pub const APPLY_LOCALE_GEN_HELPER: &str = "/usr/libexec/cosmic-locale/apply-locale-gen";

/// Whether the privileged helper exists on disk.
///
/// `just install` puts it at [`APPLY_LOCALE_GEN_HELPER`]; from a
/// fresh checkout running `just run` without first running
/// `sudo just install` it won't be there yet, and trying to apply
/// changes from the locale management page would fail with whatever
/// stderr `pkexec` produces. The page uses this check to disable
/// the Apply button and show a clearer message instead.
#[must_use]
pub fn helper_installed() -> bool {
    std::path::Path::new(APPLY_LOCALE_GEN_HELPER).exists()
}

/// One row in `/etc/locale.gen` that the user can toggle on or off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleGenEntry {
    pub code: String,
    pub charset: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Line {
    /// A toggleable locale row.
    Entry(LocaleGenEntry),
    /// Anything else — header comments, blank lines, unrecognised
    /// content. Round-tripped verbatim so we don't clobber the file's
    /// hand-written guidance.
    Verbatim(String),
}

/// In-memory representation of `/etc/locale.gen` that preserves
/// non-locale lines so the file can be rewritten without losing
/// header comments or formatting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocaleGen {
    lines: Vec<Line>,
}

impl LocaleGen {
    /// Iterate all locale entries paired with their line index.
    pub fn entries(&self) -> impl Iterator<Item = (usize, &LocaleGenEntry)> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| match line {
                Line::Entry(e) => Some((i, e)),
                Line::Verbatim(_) => None,
            })
    }

    /// Toggle the entry at the given line index. No-op if the index
    /// is out of bounds or refers to a verbatim line.
    pub fn toggle(&mut self, line_index: usize) {
        if let Some(Line::Entry(entry)) = self.lines.get_mut(line_index) {
            entry.enabled = !entry.enabled;
        }
    }
}

/// Read and parse `/etc/locale.gen`.
///
/// # Errors
///
/// Returns [`LocaleError::ReadConfig`] if the file exists but cannot be read.
/// A missing file results in an empty [`LocaleGen`].
pub async fn read_locale_gen() -> Result<LocaleGen, LocaleError> {
    match tokio::fs::read_to_string(LOCALE_GEN_PATH).await {
        Ok(contents) => Ok(parse_locale_gen(&contents)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(LocaleGen::default()),
        Err(err) => Err(LocaleError::ReadConfig {
            path: PathBuf::from(LOCALE_GEN_PATH),
            source: Arc::new(err),
        }),
    }
}

/// Parse the contents of `/etc/locale.gen`.
///
/// Each non-empty, non-pure-comment line is examined as `<code> <charset>`
/// (with an optional leading `#` marking a disabled entry). Anything that
/// doesn't fit that shape is preserved verbatim — header comments, blank
/// lines, and oddly-formatted entries all survive a parse + render round
/// trip.
#[must_use]
pub fn parse_locale_gen(input: &str) -> LocaleGen {
    let mut lines = Vec::new();
    for raw in input.lines() {
        if let Some(entry) = try_parse_locale_gen_entry(raw) {
            lines.push(Line::Entry(entry));
        } else {
            lines.push(Line::Verbatim(raw.to_string()));
        }
    }
    LocaleGen { lines }
}

/// Render a [`LocaleGen`] back to the on-disk format.
///
/// Entry lines normalise to `code charset\n` (or `# code charset\n`
/// when disabled). Verbatim lines pass through unchanged.
#[must_use]
pub fn render_locale_gen(locale_gen: &LocaleGen) -> String {
    let mut out = String::new();
    for line in &locale_gen.lines {
        match line {
            Line::Entry(e) => {
                if !e.enabled {
                    out.push_str("# ");
                }
                out.push_str(&e.code);
                out.push(' ');
                out.push_str(&e.charset);
                out.push('\n');
            }
            Line::Verbatim(s) => {
                out.push_str(s);
                out.push('\n');
            }
        }
    }
    out
}

fn try_parse_locale_gen_entry(raw: &str) -> Option<LocaleGenEntry> {
    let trimmed = raw.trim_start();

    // Strip exactly one leading '#' (with optional whitespace) for
    // commented entries. Lines like `## hello` aren't entries — they
    // start with `#` but the body after the first `#` still starts
    // with `#` and won't parse as a code.
    let (enabled, body) = if let Some(rest) = trimmed.strip_prefix('#') {
        (false, rest.trim_start())
    } else {
        (true, trimmed)
    };

    let mut parts = body.split_whitespace();
    let code = parts.next()?;
    let charset = parts.next()?;
    if parts.next().is_some() {
        return None;
    }

    if !looks_like_locale_code(code) || !looks_like_charset(charset) {
        return None;
    }

    Some(LocaleGenEntry {
        code: code.to_string(),
        charset: charset.to_string(),
        enabled,
    })
}

fn looks_like_locale_code(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    !s.contains(char::is_whitespace)
}

fn looks_like_charset(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase() && !s.contains(char::is_whitespace)
}

/// Apply a [`LocaleGen`] by writing it to `/etc/locale.gen` and then
/// running `locale-gen`, both via a single privileged `pkexec`
/// invocation so the user gets exactly one polkit prompt.
///
/// # Errors
///
/// - [`LocaleError::CommandSpawnFailed`] if `pkexec` cannot be launched.
/// - [`LocaleError::Cancelled`] if the polkit prompt was dismissed or
///   authorisation was denied.
/// - [`LocaleError::CommandFailed`] for any other non-zero exit
///   (including a `locale-gen` failure).
pub async fn apply_locale_gen(locale_gen: &LocaleGen) -> Result<(), LocaleError> {
    use std::process::Stdio;

    let contents = render_locale_gen(locale_gen);

    let spawn_err = |err: std::io::Error| LocaleError::CommandSpawnFailed {
        command: "pkexec".to_string(),
        source: Arc::new(err),
    };

    // pkexec invokes our installed helper, which polkit resolves to
    // the `dev.rabol.cosmic-locale.apply-locale-gen` action via the
    // `org.freedesktop.policykit.exec.path` annotation in the shipped
    // `.policy` file. The helper reads the new file contents from
    // stdin, writes them atomically to /etc/locale.gen, and runs
    // `locale-gen` as root.
    let mut child = tokio::process::Command::new("pkexec")
        .arg(APPLY_LOCALE_GEN_HELPER)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn_err)?;

    {
        let mut stdin = child
            .stdin
            .take()
            .expect("pkexec child was spawned with piped stdin");
        stdin
            .write_all(contents.as_bytes())
            .await
            .map_err(spawn_err)?;
        stdin.shutdown().await.ok();
    }

    let output = child.wait_with_output().await.map_err(spawn_err)?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(classify_pkexec_error(
        &stderr,
        output.status.code().unwrap_or(-1),
    ))
}

/// Map a `pkexec` exit code + stderr into a typed error.
///
/// Exit 126 means "user dismissed the auth dialog or wasn't
/// authorised." Some agents instead surface the cancellation via
/// stderr text such as "authentication failed" or "dismissed".
fn classify_pkexec_error(stderr: &str, status: i32) -> LocaleError {
    let trimmed = stderr.trim();

    if status == 126 {
        return LocaleError::Cancelled;
    }

    let lower = trimmed.to_lowercase();
    if lower.contains("authentication failed")
        || lower.contains("not authorized")
        || lower.contains("dismissed")
    {
        return LocaleError::Cancelled;
    }

    LocaleError::CommandFailed {
        command: "pkexec".to_string(),
        status,
        stderr: trimmed.to_string(),
    }
}

pub async fn list_installed_locales() -> Result<Vec<LocaleCode>, LocaleError> {
    let output = tokio::process::Command::new("locale")
        .arg("-a")
        .output()
        .await
        .map_err(|err| LocaleError::CommandSpawnFailed {
            command: "locale".to_string(),
            source: Arc::new(err),
        })?;

    if !output.status.success() {
        return Err(LocaleError::CommandFailed {
            command: "locale -a".to_string(),
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_locale_a(&stdout))
}

/// Parse the output of `locale -a` into a sorted, deduped list of
/// validated UTF-8 locale codes.
///
/// Drops blank lines, the pseudo-locales `C` and `POSIX` (with or
/// without a codeset suffix), and anything that doesn't carry a UTF-8
/// codeset.
#[must_use]
pub fn parse_locale_a(output: &str) -> Vec<LocaleCode> {
    let mut seen = std::collections::BTreeSet::new();
    for raw in output.lines() {
        let line = raw.trim();
        if line.is_empty() || is_pseudo_locale(line) || !is_utf8_locale(line) {
            continue;
        }
        if let Some(code) = LocaleCode::new(line) {
            seen.insert(code.as_str().to_string());
        }
    }
    seen.into_iter()
        .filter_map(|s| LocaleCode::new(&s))
        .collect()
}

fn is_pseudo_locale(line: &str) -> bool {
    let stem = line.split('.').next().unwrap_or(line);
    stem.eq_ignore_ascii_case("C") || stem.eq_ignore_ascii_case("POSIX")
}

fn is_utf8_locale(line: &str) -> bool {
    let codeset = match line.split_once('.') {
        Some((_, after)) => after.split('@').next().unwrap_or(after),
        None => return false,
    };
    let normalised = codeset.replace('-', "").to_ascii_lowercase();
    normalised == "utf8"
}

/// Compute the new locale array for "reset overrides to language": keep
/// `LANG` and re-set every existing `LC_*` to `LANG`'s value.
///
/// `SetLocale` only updates variables present in its input; absent
/// variables are left alone. To make `LC_*` overrides effectively
/// disappear we therefore have to assign each one explicitly to
/// `LANG`'s value rather than omit them.
fn build_reset_locale(current: &[String]) -> Result<Vec<String>, LocaleError> {
    let lang_value = current
        .iter()
        .find_map(|entry| entry.strip_prefix("LANG=").map(str::to_string))
        .ok_or_else(|| LocaleError::Daemon("system has no LANG configured".to_string()))?;

    let mut new_locale = Vec::with_capacity(current.len());
    new_locale.push(format!("LANG={lang_value}"));
    for entry in current {
        if let Some((key, _)) = entry.split_once('=')
            && key.starts_with("LC_")
        {
            new_locale.push(format!("{key}={lang_value}"));
        }
    }
    Ok(new_locale)
}

/// Convert a zbus error into our typed error. Polkit denial and
/// cancellation both surface as `org.freedesktop.PolicyKit1.Error.NotAuthorized`
/// (or `…AccessDenied` on some versions); everything else is just a
/// daemon error with the original message.
fn zbus_to_locale_error(err: &zbus::Error) -> LocaleError {
    if let zbus::Error::MethodError(name, _, _) = err {
        let n = name.as_str();
        if n.ends_with(".NotAuthorized") || n.ends_with(".AccessDenied") {
            return LocaleError::Cancelled;
        }
    }
    LocaleError::Daemon(err.to_string())
}

fn strip_quotes(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_code_rejects_empty_and_whitespace() {
        assert!(LocaleCode::new("").is_none());
        assert!(LocaleCode::new("   ").is_none());
        assert!(LocaleCode::new("en US").is_none());
        assert!(LocaleCode::new("en\tUS").is_none());
    }

    #[test]
    fn locale_code_accepts_typical_codes() {
        assert_eq!(
            LocaleCode::new("en_US.UTF-8").unwrap().as_str(),
            "en_US.UTF-8"
        );
        assert_eq!(LocaleCode::new("C").unwrap().as_str(), "C");
        assert_eq!(LocaleCode::new("POSIX").unwrap().as_str(), "POSIX");
        assert_eq!(
            LocaleCode::new("de_DE@euro").unwrap().as_str(),
            "de_DE@euro"
        );
    }

    #[test]
    fn locale_code_trims() {
        assert_eq!(
            LocaleCode::new("  en_US.UTF-8  ").unwrap().as_str(),
            "en_US.UTF-8"
        );
    }

    fn assert_empty(s: &LocaleSettings) {
        assert!(s.lang.is_none());
        assert!(s.language.is_none());
        assert!(s.lc_overrides.is_empty());
    }

    #[test]
    fn parse_handles_empty_input() {
        assert_empty(&parse_default_locale(""));
    }

    #[test]
    fn parse_handles_only_comments_and_blanks() {
        let input = "\
# comment
   # indented comment

\t
";
        assert_empty(&parse_default_locale(input));
    }

    #[test]
    fn parse_extracts_lang_and_lc_overrides() {
        let input = "\
LANG=en_US.UTF-8
LC_TIME=en_DK.UTF-8
LC_NUMERIC=de_DE.UTF-8
";
        let s = parse_default_locale(input);
        assert_eq!(s.lang.as_ref().unwrap().as_str(), "en_US.UTF-8");
        assert_eq!(
            s.lc_overrides.get("LC_TIME").unwrap().as_str(),
            "en_DK.UTF-8"
        );
        assert_eq!(
            s.lc_overrides.get("LC_NUMERIC").unwrap().as_str(),
            "de_DE.UTF-8"
        );
        assert!(s.language.is_none());
    }

    #[test]
    fn parse_strips_double_and_single_quotes() {
        let input = "\
LANG=\"en_US.UTF-8\"
LC_TIME='en_DK.UTF-8'
";
        let s = parse_default_locale(input);
        assert_eq!(s.lang.as_ref().unwrap().as_str(), "en_US.UTF-8");
        assert_eq!(
            s.lc_overrides.get("LC_TIME").unwrap().as_str(),
            "en_DK.UTF-8"
        );
    }

    #[test]
    fn parse_handles_export_prefix() {
        let input = "export LANG=en_US.UTF-8\n";
        let s = parse_default_locale(input);
        assert_eq!(s.lang.as_ref().unwrap().as_str(), "en_US.UTF-8");
    }

    #[test]
    fn parse_captures_language_as_raw_string() {
        let input = "LANGUAGE=en_US:en\n";
        let s = parse_default_locale(input);
        assert_eq!(s.language.as_deref(), Some("en_US:en"));
        assert!(s.lang.is_none());
    }

    #[test]
    fn parse_skips_invalid_locale_codes() {
        let input = "\
LANG=en_US.UTF-8
LC_TIME=
LC_NUMERIC=\"\"
LC_GARBAGE=has space
";
        let s = parse_default_locale(input);
        assert_eq!(s.lang.as_ref().unwrap().as_str(), "en_US.UTF-8");
        assert!(s.lc_overrides.is_empty());
    }

    #[test]
    fn parse_ignores_unknown_keys() {
        let input = "\
PATH=/usr/bin
HOME=/home/user
LANG=en_US.UTF-8
";
        let s = parse_default_locale(input);
        assert_eq!(s.lang.as_ref().unwrap().as_str(), "en_US.UTF-8");
        assert!(s.lc_overrides.is_empty());
    }

    #[test]
    fn parse_skips_lines_without_equals() {
        let input = "\
this is not a config line
LANG=en_US.UTF-8
";
        let s = parse_default_locale(input);
        assert_eq!(s.lang.as_ref().unwrap().as_str(), "en_US.UTF-8");
    }

    #[test]
    fn settings_default_is_empty() {
        assert_empty(&LocaleSettings::default());
    }

    #[test]
    fn build_reset_sets_each_lc_to_lang_value() {
        let current = vec![
            "LANG=en_US.UTF-8".to_string(),
            "LC_TIME=da_DK.utf8".to_string(),
            "LC_NUMERIC=de_DE.utf8".to_string(),
        ];

        let result = build_reset_locale(&current).unwrap();

        assert_eq!(
            result,
            vec![
                "LANG=en_US.UTF-8".to_string(),
                "LC_TIME=en_US.UTF-8".to_string(),
                "LC_NUMERIC=en_US.UTF-8".to_string(),
            ]
        );
    }

    #[test]
    fn build_reset_keeps_lang_when_no_overrides_present() {
        let current = vec!["LANG=en_US.UTF-8".to_string()];
        let result = build_reset_locale(&current).unwrap();
        assert_eq!(result, vec!["LANG=en_US.UTF-8".to_string()]);
    }

    #[test]
    fn build_reset_errors_when_lang_missing() {
        let current = vec!["LC_TIME=da_DK.utf8".to_string()];
        let result = build_reset_locale(&current);
        assert!(matches!(result, Err(LocaleError::Daemon(_))));
    }

    fn lang(value: &str) -> LocaleCode {
        LocaleCode::new(value).unwrap()
    }

    #[test]
    fn effective_returns_all_twelve_categories_in_order() {
        let settings = LocaleSettings::default();
        let result = effective_categories(&settings);
        assert_eq!(result.len(), LC_CATEGORIES.len());
        for (view, expected_name) in result.iter().zip(LC_CATEGORIES.iter()) {
            assert_eq!(view.name, *expected_name);
        }
    }

    #[test]
    fn effective_inherits_from_lang_when_no_overrides() {
        let mut settings = LocaleSettings::default();
        settings.lang = Some(lang("en_US.UTF-8"));

        for view in effective_categories(&settings) {
            assert_eq!(view.value, "en_US.UTF-8");
            assert_eq!(view.source, CategorySource::Inherited);
        }
    }

    #[test]
    fn effective_marks_explicit_overrides() {
        let mut settings = LocaleSettings::default();
        settings.lang = Some(lang("en_US.UTF-8"));
        settings
            .lc_overrides
            .insert("LC_TIME".to_string(), lang("en_DK.UTF-8"));
        settings
            .lc_overrides
            .insert("LC_NUMERIC".to_string(), lang("de_DE.UTF-8"));

        let by_name: std::collections::HashMap<_, _> = effective_categories(&settings)
            .into_iter()
            .map(|v| (v.name, v))
            .collect();

        let lc_time = &by_name["LC_TIME"];
        assert_eq!(lc_time.value, "en_DK.UTF-8");
        assert_eq!(lc_time.source, CategorySource::Override);

        let lc_numeric = &by_name["LC_NUMERIC"];
        assert_eq!(lc_numeric.value, "de_DE.UTF-8");
        assert_eq!(lc_numeric.source, CategorySource::Override);

        // Categories without an override should inherit LANG.
        let lc_ctype = &by_name["LC_CTYPE"];
        assert_eq!(lc_ctype.value, "en_US.UTF-8");
        assert_eq!(lc_ctype.source, CategorySource::Inherited);
    }

    #[test]
    fn effective_falls_back_to_c_when_nothing_set() {
        let settings = LocaleSettings::default();
        for view in effective_categories(&settings) {
            assert_eq!(view.value, "C");
            assert_eq!(view.source, CategorySource::Default);
        }
    }

    #[test]
    fn locale_gen_parses_enabled_and_disabled() {
        let input = "\
# This file lists locales that you wish to have built.
en_US.UTF-8 UTF-8
# en_GB.UTF-8 UTF-8
de_DE.UTF-8 UTF-8
";
        let parsed = parse_locale_gen(input);
        let entries: Vec<_> = parsed.entries().map(|(_, e)| e).collect();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].code, "en_US.UTF-8");
        assert!(entries[0].enabled);
        assert_eq!(entries[1].code, "en_GB.UTF-8");
        assert!(!entries[1].enabled);
        assert_eq!(entries[2].code, "de_DE.UTF-8");
        assert!(entries[2].enabled);
    }

    #[test]
    fn locale_gen_round_trips_unchanged_input() {
        let input = "\
# Header comment
en_US.UTF-8 UTF-8
# en_GB.UTF-8 UTF-8

# Trailing block
de_DE.UTF-8 UTF-8
";
        let parsed = parse_locale_gen(input);
        let rendered = render_locale_gen(&parsed);
        assert_eq!(rendered, input);
    }

    #[test]
    fn locale_gen_toggle_flips_only_target() {
        let input = "\
en_US.UTF-8 UTF-8
# en_GB.UTF-8 UTF-8
";
        let mut parsed = parse_locale_gen(input);
        let target_index = parsed
            .entries()
            .find(|(_, e)| e.code == "en_GB.UTF-8")
            .map(|(i, _)| i)
            .unwrap();
        parsed.toggle(target_index);

        let rendered = render_locale_gen(&parsed);
        assert_eq!(rendered, "en_US.UTF-8 UTF-8\nen_GB.UTF-8 UTF-8\n");
    }

    #[test]
    fn locale_gen_preserves_non_entry_lines_verbatim() {
        let input = "\
## A header with double-hash that isn't a locale entry
   # Indented comment with weirdness
not a locale line at all
en_US.UTF-8 UTF-8
";
        let parsed = parse_locale_gen(input);
        let rendered = render_locale_gen(&parsed);
        assert_eq!(rendered, input);
        // Only one toggleable entry survived parsing.
        assert_eq!(parsed.entries().count(), 1);
    }

    #[test]
    fn locale_gen_handles_multi_charset_for_same_base() {
        let input = "\
# en_US ISO-8859-1
en_US.UTF-8 UTF-8
";
        let parsed = parse_locale_gen(input);
        let entries: Vec<_> = parsed.entries().map(|(_, e)| e).collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "en_US");
        assert_eq!(entries[0].charset, "ISO-8859-1");
        assert!(!entries[0].enabled);
        assert_eq!(entries[1].code, "en_US.UTF-8");
        assert!(entries[1].enabled);
    }

    #[test]
    fn locale_gen_toggle_out_of_bounds_is_noop() {
        let mut parsed = parse_locale_gen("en_US.UTF-8 UTF-8\n");
        parsed.toggle(99);
        let rendered = render_locale_gen(&parsed);
        assert_eq!(rendered, "en_US.UTF-8 UTF-8\n");
    }

    #[test]
    fn build_category_set_replaces_existing() {
        let current = vec![
            "LANG=en_US.UTF-8".to_string(),
            "LC_TIME=da_DK.utf8".to_string(),
            "LC_NUMERIC=de_DE.utf8".to_string(),
        ];
        let result = build_category_set(&current, "LC_TIME", "fr_FR.UTF-8");
        assert_eq!(
            result,
            vec![
                "LANG=en_US.UTF-8".to_string(),
                "LC_TIME=fr_FR.UTF-8".to_string(),
                "LC_NUMERIC=de_DE.utf8".to_string(),
            ]
        );
    }

    #[test]
    fn build_category_set_appends_when_absent() {
        let current = vec!["LANG=en_US.UTF-8".to_string()];
        let result = build_category_set(&current, "LC_TIME", "en_DK.UTF-8");
        assert_eq!(
            result,
            vec![
                "LANG=en_US.UTF-8".to_string(),
                "LC_TIME=en_DK.UTF-8".to_string(),
            ]
        );
    }

    #[test]
    fn parse_locale_a_filters_pseudo_and_non_utf8() {
        let output = "
C
C.UTF-8
POSIX
en_US
en_US.UTF-8
de_DE.utf8
fr_FR.iso88591
de_DE.UTF-8@euro
";
        let result = parse_locale_a(output);
        let codes: Vec<&str> = result.iter().map(LocaleCode::as_str).collect();
        // C* and POSIX dropped; en_US has no codeset → dropped; iso88591 → dropped.
        // de_DE.utf8 and de_DE.UTF-8@euro both pass (utf8 normalisation).
        assert!(codes.contains(&"en_US.UTF-8"));
        assert!(codes.contains(&"de_DE.utf8"));
        assert!(codes.contains(&"de_DE.UTF-8@euro"));
        assert!(!codes.iter().any(|c| c.starts_with('C')));
        assert!(!codes.contains(&"POSIX"));
        assert!(!codes.contains(&"en_US"));
        assert!(!codes.contains(&"fr_FR.iso88591"));
    }

    #[test]
    fn parse_locale_a_dedupes_and_sorts() {
        let output = "fr_FR.UTF-8\nde_DE.UTF-8\nfr_FR.UTF-8\nen_US.UTF-8\n";
        let result = parse_locale_a(output);
        let codes: Vec<&str> = result.iter().map(LocaleCode::as_str).collect();
        assert_eq!(codes, vec!["de_DE.UTF-8", "en_US.UTF-8", "fr_FR.UTF-8"]);
    }

    #[test]
    fn parse_locale_a_handles_empty_output() {
        assert!(parse_locale_a("").is_empty());
        assert!(parse_locale_a("\n\n  \n").is_empty());
    }

    #[test]
    fn build_reset_ignores_non_lc_entries() {
        // LANGUAGE shouldn't be touched — only LANG + LC_* are emitted.
        let current = vec![
            "LANG=en_US.UTF-8".to_string(),
            "LANGUAGE=en_US:en".to_string(),
            "LC_TIME=da_DK.utf8".to_string(),
        ];
        let result = build_reset_locale(&current).unwrap();
        assert_eq!(
            result,
            vec![
                "LANG=en_US.UTF-8".to_string(),
                "LC_TIME=en_US.UTF-8".to_string(),
            ]
        );
    }
}
