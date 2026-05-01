# Cosmic Locale

A native [libcosmic](https://github.com/pop-os/libcosmic) application
for managing system language and locale settings on Linux. Inspired
by [MX-Linux/mx-locale](https://github.com/MX-Linux/mx-locale), it
exposes the things `localectl` and `locale-gen` do — but with a GUI
that fits into a COSMIC desktop session and authenticates the same
way other COSMIC apps do.

## What it does

Four pages, one per nav entry:

- **Welcome** — landing screen with a short description of what
  the app manages.
- **Current locale** — shows `LANG`, `LANGUAGE`, the file the
  values were read from (`/etc/locale.conf` on systemd systems or
  `/etc/default/locale` on Debian/Ubuntu/Pop!_OS), and any `LC_*`
  overrides. A "Reset overrides to language" button clears every
  `LC_*` override by setting them all to the current `LANG` value.
- **Locale categories** — resolves all twelve POSIX `LC_*`
  categories and tags each one as **Override**, **Inherited from
  LANG**, or **Default (C)**. Every row carries a three-dot button
  that opens a context-drawer picker: search through the system's
  installed UTF-8 locales (from `locale -a`) and pick a new value
  for that single category. The picker also shows a live preview
  panel with today's date, the current time, a long datetime, and
  a sample formatted number, all rendered using the selected
  locale.
- **Locale management** — multi-select editor for
  `/etc/locale.gen`. Tick a row to enable that locale, untick to
  disable, click **Apply** to rewrite the file and run
  `locale-gen` in one privileged step. After a successful apply,
  the picker on the previous page automatically refreshes to
  include any newly-generated locales.

All privileged actions go through polkit. With the rules file
installed (see below) the active wheel/sudo user is auto-granted
both `org.freedesktop.locale1.set-locale` (used by the per-category
picker and the reset button) and our own
`dev.rabol.cosmic-locale.apply-locale-gen` (used by the Locale
management page). No prompts in the common case.

## Supported platforms

- **OS:** any modern Linux running systemd. Tested on Arch and
  Pop!_OS.
- **Locale generation** (the management page) requires
  `locale-gen` to be installed on `PATH`. That covers Arch,
  Debian, Ubuntu, and Pop!_OS out of the box; not all distros use
  this tool — Fedora, for instance, ships pre-generated locales
  through `glibc-langpack-*` packages and has no `locale-gen`.
- **Polkit auth** uses the same flow as cosmic-settings, so a
  working polkit agent for your session is required (cosmic-osd on
  COSMIC, polkit-gnome on GNOME, polkit-kde on KDE,
  lxqt-policykit anywhere).

## Getting started

```sh
just                # build release binary
sudo just install   # required for full functionality (see below)
just run            # build and run from target/release
just check          # clippy
```

`sudo just install` is **required** before the Locale management
page's **Apply** button will work — the privileged helper at
`/usr/libexec/cosmic-locale/apply-locale-gen` is invoked via
`pkexec`, so it has to actually exist on disk. The other three
pages work without installing because they go through D-Bus to
systemd-localed.

The rest of the `just` recipes:

- `just install` — install binary, desktop entry, appstream
  metadata, icon, polkit policy + rules, and the privileged helper.
- `just uninstall` — remove everything `just install` placed.
- `just vendor` — create a vendored dependency tarball.
- `just build-vendored` — build with the vendored tarball.
- `just check-json` — clippy with JSON output, useful for IDEs.

### Polkit and the privileged helper

Applying changes from the **Locale management** page (rewriting
`/etc/locale.gen` and running `locale-gen`) requires root and runs
through `pkexec`. `just install` lays down three files alongside
the binary:

- `/usr/share/polkit-1/actions/dev.rabol.cosmic-locale.policy`
  declares the polkit action
  `dev.rabol.cosmic-locale.apply-locale-gen` and points it at the
  helper. Without our `.rules` file the action falls back to the
  policy default (`auth_admin_keep`), which prompts but caches the
  auth — better than the old generic "run /bin/sh as root" prompt.
- `/usr/share/polkit-1/rules.d/dev.rabol.cosmic-locale.rules`
  grants that action — and `org.freedesktop.locale1.set-locale`
  used by the per-category picker — to local active members of
  `wheel` or `sudo` without prompting. This mirrors the rule the
  cosmic-settings package ships, so cosmic-locale behaves the same
  way on systems where cosmic-settings isn't installed.
- `/usr/libexec/cosmic-locale/apply-locale-gen` is a small POSIX
  shell helper. It reads the new `/etc/locale.gen` from stdin,
  writes it atomically via `mktemp` + `mv`, then execs
  `locale-gen`.

Without these files the app still works, but every privileged
action will surface a polkit prompt — or, for `locale-gen`,
outright fail if the helper isn't installed at all. For
development, run `sudo just install` once so the **Apply** button
on the Locale management page can find the helper.

## Packaging

If packaging for a Linux distribution, vendor dependencies locally
with the `vendor` rule, and build with the vendored sources using
the `build-vendored` rule. When installing files, use the
`rootdir` and `prefix` variables to change installation paths.

```sh
just vendor
just build-vendored
just rootdir=debian/cosmic-locale prefix=/usr install
```

It is recommended to build a source tarball with the vendored
dependencies, which can typically be done by running `just vendor`
on the host system before it enters the build environment.

## Translators

[Fluent][fluent] is used for localization. Fluent's translation
files live in the [i18n directory](./i18n). To add a new
translation, copy the [English (en) localization](./i18n/en) of
the project, rename `en` to the desired [ISO 639-1 language
code][iso-codes], and translate each [message
identifier][fluent-guide]. If a string doesn't need translating in
your language, omit it — the loader falls back to English.

## Developers

Developers should install [rustup][rustup] and configure their
editor to use [rust-analyzer][rust-analyzer]. To improve
compilation times, disable LTO in the release profile, install the
[mold][mold] linker, and configure [sccache][sccache] for use with
Rust. The [mold][mold] linker will only improve link times if LTO
is disabled.

## Acknowledgements

- The behaviour cosmic-locale re-implements comes from
  [MX-Linux's mx-locale][mx-locale], a Qt/C++ tool that wraps
  `update-locale` and `dpkg-reconfigure locales`. This is not a
  port of that code — it's a libcosmic application that follows
  the same overall workflow.
- The project skeleton came from a [cosmic-utils][cosmic-utils]
  app template — the libcosmic nav bar, context drawer, settings
  widgets, and the just-based build flow originated there.

## AI assistance

Parts of cosmic-locale's source code were drafted with the help of
an AI coding assistant (Claude). The role distribution is worth
being explicit about:

- The **idea**, scope, milestone plan, and product direction came
  from the human author.
- Every feature, UI change, polkit decision, and refactor was
  **requested** by the human and **approved** before being
  committed.
- The human reviewed every diff before it landed on `main` and
  ran the app at every step to verify behaviour.

The AI was used as a tool for drafting code, surfacing libcosmic
API patterns, and generating test cases — not for deciding what
to build or whether a piece of work was correct. Bug reports and
design feedback belong with the maintainer, not the tool.

## Disclaimer

The author has made every effort to respect upstream licences and
to credit the projects this software builds on. If anything
appears incorrect, missing, or improperly attributed — in this
README, the source headers, the polkit policy, the helper script,
or anywhere else — please [open an issue][issues] or email the
maintainer at <steen@rabol.dev> so it can be corrected.

## License

cosmic-locale is licensed under the [GNU General Public License
v3.0 only](./LICENSE). The full licence text lives in
[LICENSE](./LICENSE); each source file carries an SPDX identifier
matching that licence.

[fluent]: https://projectfluent.org/
[fluent-guide]: https://projectfluent.org/fluent/guide/hello.html
[iso-codes]: https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes
[just]: https://github.com/casey/just
[rustup]: https://rustup.rs/
[rust-analyzer]: https://rust-analyzer.github.io/
[mold]: https://github.com/rui314/mold
[sccache]: https://github.com/mozilla/sccache
[mx-locale]: https://github.com/MX-Linux/mx-locale
[cosmic-utils]: https://github.com/cosmic-utils
[issues]: https://github.com/rabol/cosmic-locale/issues
