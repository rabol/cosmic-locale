app-title = Cosmic Locale
about = Info
repository = Quelltextarchiv
view = Ansicht

# Navigation
page-welcome = Willkommen
page-current-locale = Aktuelle Locale
page-locale-categories = Locale-Kategorien
page-locale-management = Locale-Verwaltung

# Welcome page
welcome-title = Willkommen bei Cosmic Locale
welcome-body = Verwalte die Sprach- und Locale-Einstellungen deines Systems.

welcome-pages-title = Was du hier tun kannst
welcome-page-current-locale = Zeige das aktive LANG, LANGUAGE und vorhandene LC_*-Überschreibungen an; setze Überschreibungen auf LANG zurück.
welcome-page-locale-categories = Sieh dir die zwölf POSIX-Kategorien einzeln an, mit Auswahl pro Kategorie und Live-Vorschau.
welcome-page-locale-management = Lege fest, welche Locales installiert sind, indem du /etc/locale.gen bearbeitest und locale-gen ausführst.

welcome-summary-title = Systemübersicht
welcome-summary-loading = Lädt…
welcome-summary-error = Systemlocale konnte nicht gelesen werden: { $reason }
welcome-summary-lang-set = Sprache: { $lang }
welcome-summary-lang-unset = Sprache: nicht festgelegt
welcome-summary-overrides-none = Keine Kategorieüberschreibungen.
welcome-summary-overrides-some = Kategorieüberschreibungen: { $count }.
welcome-summary-source-file = Geladen aus { $path }.
welcome-summary-source-env = Geladen aus der Prozessumgebung.

welcome-actions-title = Schnellaktionen
welcome-action-current = Aktuelle Locale anzeigen
welcome-action-management = Installierte Locales verwalten
welcome-action-language = Systemsprache festlegen

# Placeholder shown on pages whose content has not yet been implemented.
coming-soon = Bald verfügbar.

# Current locale page
current-locale-section = Systemlocale
current-locale-lang = Sprache (LANG)
current-locale-language = Ausweichliste (LANGUAGE)
current-locale-overrides = Kategorieüberschreibungen
current-locale-source = Quelle
current-locale-not-set = Nicht festgelegt
current-locale-loading = Lädt…
current-locale-error = Systemlocale konnte nicht gelesen werden: { $reason }
# Locale categories page
locale-categories-section = Effektive Kategorien
locale-categories-source-override = Überschrieben
locale-categories-source-inherited = Geerbt von LANG
locale-categories-source-default = Standard (C)
locale-categories-edit = Locale ändern…

# Category picker (context drawer)
picker-title = { $category } ändern
picker-system-language-title = Systemsprache festlegen
picker-search-placeholder = Locales suchen
picker-empty = Keine passenden Locales.
picker-loading = Installierte Locales werden geladen…
picker-load-failed = Installierte Locales konnten nicht aufgelistet werden: { $reason }
picker-apply = Anwenden
picker-cancel = Abbrechen
picker-applying = Wird angewendet…
picker-error = { $reason }
picker-preview-title = Vorschau
picker-preview-date = Datum
picker-preview-time = Uhrzeit
picker-preview-datetime = Datum und Uhrzeit
picker-preview-number = Zahl
picker-preview-loading = Vorschau wird berechnet…

# Locale management page
locale-management-section = Installierte Locales
locale-management-search-placeholder = Locales suchen
locale-management-empty = Keine Locale entspricht deiner Suche.
locale-management-loading = /etc/locale.gen wird geladen…
locale-management-load-failed = /etc/locale.gen konnte nicht gelesen werden: { $reason }
locale-management-apply = Anwenden
locale-management-applying = Wird angewendet…
locale-management-apply-failed = Anwenden fehlgeschlagen: { $reason }
locale-management-apply-cancelled = Authentifizierung abgebrochen oder abgelehnt.
locale-management-helper-missing = Der privilegierte Helfer ist nicht unter { $path } installiert. Führe `sudo just install` aus dem Projektverzeichnis aus, damit der Anwenden-Button /etc/locale.gen neu schreiben und locale-gen ausführen kann.

current-locale-reset = Überschreibungen auf Sprache zurücksetzen
current-locale-reset-pending = Wird zurückgesetzt…
current-locale-reset-cancelled = Authentifizierung abgebrochen oder abgelehnt.
current-locale-reset-failed = Überschreibungen konnten nicht zurückgesetzt werden: { $reason }
