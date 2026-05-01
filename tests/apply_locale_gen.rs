//! Integration tests for `resources/apply-locale-gen`.
//!
//! The helper is invoked directly with `LOCALE_GEN_PATH` pointing at
//! a tempfile and `PATH` shadowed by a tempdir containing a fake
//! `locale-gen` script that records its invocation. This exercises
//! the same code path the production helper takes — read stdin,
//! atomically write target, exec locale-gen — without ever touching
//! `/etc/locale.gen` or the real `locale-gen` on the host.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

const HELPER: &str = "resources/apply-locale-gen";

/// Drop a fake `locale-gen` shell script in `dir` that records its
/// arguments to `log`. The fake exits 0 on every invocation.
fn install_fake_locale_gen(dir: &Path, log: &Path) {
    let bin = dir.join("locale-gen");
    let script = format!("#!/bin/sh\necho ran $@ > {log}\n", log = log.display());
    std::fs::write(&bin, script).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Build a `PATH` value where `dir` is searched first, falling back
/// to whatever `PATH` the test process inherited.
fn shadowed_path(dir: &Path) -> String {
    let inherited = std::env::var("PATH").unwrap_or_default();
    format!("{}:{inherited}", dir.display())
}

fn run_helper(target: &Path, fake_dir: &Path, stdin_bytes: &[u8]) -> std::process::Output {
    let mut child = Command::new(HELPER)
        .env("LOCALE_GEN_PATH", target)
        .env("PATH", shadowed_path(fake_dir))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn helper");

    child
        .stdin
        .as_mut()
        .expect("piped stdin")
        .write_all(stdin_bytes)
        .expect("write stdin");

    child.wait_with_output().expect("wait_with_output")
}

#[test]
fn writes_stdin_to_target_and_runs_locale_gen() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let target = tmp.path().join("locale.gen");
    let fake_dir = tmp.path().join("bin");
    let log = tmp.path().join("locale-gen.log");
    std::fs::create_dir(&fake_dir).unwrap();
    install_fake_locale_gen(&fake_dir, &log);

    let payload = b"# header comment\nen_US.UTF-8 UTF-8\nde_DE.UTF-8 UTF-8\n";
    let output = run_helper(&target, &fake_dir, payload);
    assert!(
        output.status.success(),
        "helper exited non-zero: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let written = std::fs::read(&target).expect("target was written");
    assert_eq!(written, payload);

    assert!(log.exists(), "fake locale-gen was not invoked");
}

#[test]
fn overwrites_existing_target_atomically() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let target = tmp.path().join("locale.gen");
    let fake_dir = tmp.path().join("bin");
    let log = tmp.path().join("locale-gen.log");
    std::fs::create_dir(&fake_dir).unwrap();
    install_fake_locale_gen(&fake_dir, &log);

    std::fs::write(&target, b"ORIGINAL\n").unwrap();

    let output = run_helper(&target, &fake_dir, b"REPLACEMENT\n");
    assert!(output.status.success());

    let final_contents = std::fs::read(&target).unwrap();
    assert_eq!(final_contents, b"REPLACEMENT\n");
}

#[test]
fn fails_loudly_when_locale_gen_is_missing_from_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let target = tmp.path().join("locale.gen");
    // Empty fake dir on an empty PATH means `locale-gen` cannot be
    // resolved by exec — the helper should exit non-zero rather than
    // silently succeed.
    let fake_dir = tmp.path().join("bin");
    std::fs::create_dir(&fake_dir).unwrap();

    let mut child = Command::new(HELPER)
        .env("LOCALE_GEN_PATH", &target)
        .env("PATH", fake_dir.to_str().unwrap()) // no inherited PATH
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn helper");

    child.stdin.as_mut().unwrap().write_all(b"junk\n").unwrap();

    let status = child.wait().unwrap();
    assert!(!status.success());
}
