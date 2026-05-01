app-title = Cosmic Locale
about = About
repository = Repository
view = View

# Navigation
page-welcome = Welcome
page-current-locale = Current locale
page-locale-categories = Locale categories
page-locale-management = Locale management

# Welcome page
welcome-title = Welcome to Cosmic Locale
welcome-body = Manage your system language and locale settings.

welcome-pages-title = What you can do here
welcome-page-current-locale = View the active LANG, LANGUAGE, and any LC_* overrides; reset overrides back to LANG.
welcome-page-locale-categories = Drill into the twelve POSIX categories with a per-category picker and a live preview.
welcome-page-locale-management = Toggle which locales are installed by editing /etc/locale.gen and running locale-gen.

welcome-summary-title = System summary
welcome-summary-loading = Loading…
welcome-summary-error = Failed to read system locale: { $reason }
welcome-summary-lang-set = Language: { $lang }
welcome-summary-lang-unset = Language: not set
welcome-summary-overrides-none = No category overrides.
welcome-summary-overrides-some = Category overrides: { $count }.
welcome-summary-source-file = Loaded from { $path }.
welcome-summary-source-env = Loaded from the process environment.

welcome-actions-title = Quick actions
welcome-action-current = View current locale
welcome-action-management = Manage installed locales

# Placeholder shown on pages whose content has not yet been implemented.
coming-soon = Coming soon.

# Current locale page
current-locale-section = System locale
current-locale-lang = Language (LANG)
current-locale-language = Fallback list (LANGUAGE)
current-locale-overrides = Category overrides
current-locale-source = Source
current-locale-not-set = Not set
current-locale-loading = Loading…
current-locale-error = Failed to read system locale: { $reason }
# Locale categories page
locale-categories-section = Effective categories
locale-categories-source-override = Override
locale-categories-source-inherited = Inherited from LANG
locale-categories-source-default = Default (C)
locale-categories-edit = Change locale…

# Category picker (context drawer)
picker-title = Change { $category }
picker-search-placeholder = Search locales
picker-empty = No matching locales.
picker-loading = Loading installed locales…
picker-load-failed = Could not list installed locales: { $reason }
picker-apply = Apply
picker-cancel = Cancel
picker-applying = Applying…
picker-error = { $reason }
picker-preview-title = Preview
picker-preview-date = Date
picker-preview-time = Time
picker-preview-datetime = Date and time
picker-preview-number = Number
picker-preview-loading = Computing preview…

# Locale management page
locale-management-section = Installed locales
locale-management-search-placeholder = Search locales
locale-management-empty = No locales match your search.
locale-management-loading = Loading /etc/locale.gen…
locale-management-load-failed = Could not read /etc/locale.gen: { $reason }
locale-management-apply = Apply
locale-management-applying = Applying…
locale-management-apply-failed = Could not apply: { $reason }
locale-management-apply-cancelled = Authentication cancelled or denied.
locale-management-helper-missing = The privileged helper is not installed at { $path }. Run `sudo just install` from the project root so the Apply button can rewrite /etc/locale.gen and run locale-gen.

current-locale-reset = Reset overrides to language
current-locale-reset-pending = Resetting…
current-locale-reset-cancelled = Authentication cancelled or denied.
current-locale-reset-failed = Could not reset overrides: { $reason }
