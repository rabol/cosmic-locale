// SPDX-License-Identifier: GPL-3.0-only

//! Locale model — types, parsing, and async readers/writers for the
//! system locale configuration.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

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
