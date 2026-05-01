app-title = Cosmic Locale
about = Om
repository = Kodelager
view = Vis

# Navigation
page-welcome = Velkommen
page-current-locale = Aktuel lokalitet
page-locale-categories = Lokalitetskategorier
page-locale-management = Lokalitetsstyring

# Welcome page
welcome-title = Velkommen til Cosmic Locale
welcome-body = Administrér dit systems sprog- og lokalitetsindstillinger.

welcome-pages-title = Hvad du kan gøre her
welcome-page-current-locale = Se den aktuelle LANG, LANGUAGE og eventuelle LC_*-tilsidesættelser; nulstil tilsidesættelser tilbage til LANG.
welcome-page-locale-categories = Gå i dybden med de tolv POSIX-kategorier ved hjælp af en kategorivælger og en levende forhåndsvisning.
welcome-page-locale-management = Vælg hvilke lokaliteter der er installeret ved at redigere /etc/locale.gen og køre locale-gen.

welcome-summary-title = Systemoversigt
welcome-summary-loading = Indlæser…
welcome-summary-error = Kunne ikke læse systemlokalitet: { $reason }
welcome-summary-lang-set = Sprog: { $lang }
welcome-summary-lang-unset = Sprog: ikke angivet
welcome-summary-overrides-none = Ingen kategorier tilsidesat.
welcome-summary-overrides-some = Tilsidesatte kategorier: { $count }.
welcome-summary-source-file = Indlæst fra { $path }.
welcome-summary-source-env = Indlæst fra procesmiljøet.

welcome-actions-title = Hurtige handlinger
welcome-action-current = Se aktuel lokalitet
welcome-action-management = Administrér installerede lokaliteter
welcome-action-language = Indstil systemsprog

# Placeholder shown on pages whose content has not yet been implemented.
coming-soon = Kommer snart.

# Current locale page
current-locale-section = Systemlokalitet
current-locale-lang = Sprog (LANG)
current-locale-language = Reserveliste (LANGUAGE)
current-locale-overrides = Kategori­tilsidesættelser
current-locale-source = Kilde
current-locale-not-set = Ikke angivet
current-locale-loading = Indlæser…
current-locale-error = Kunne ikke læse systemlokalitet: { $reason }
# Locale categories page
locale-categories-section = Effektive kategorier
locale-categories-source-override = Tilsidesat
locale-categories-source-inherited = Arvet fra LANG
locale-categories-source-default = Standard (C)
locale-categories-edit = Ændr lokalitet…

# Category picker (context drawer)
picker-title = Ændr { $category }
picker-system-language-title = Indstil systemsprog
picker-search-placeholder = Søg lokaliteter
picker-empty = Ingen matchende lokaliteter.
picker-loading = Indlæser installerede lokaliteter…
picker-load-failed = Kunne ikke vise installerede lokaliteter: { $reason }
picker-apply = Anvend
picker-cancel = Annullér
picker-applying = Anvender…
picker-error = { $reason }
picker-preview-title = Forhåndsvisning
picker-preview-date = Dato
picker-preview-time = Klokkeslæt
picker-preview-datetime = Dato og klokkeslæt
picker-preview-number = Tal
picker-preview-loading = Beregner forhåndsvisning…

# Locale management page
locale-management-section = Installerede lokaliteter
locale-management-search-placeholder = Søg lokaliteter
locale-management-empty = Ingen lokaliteter matcher din søgning.
locale-management-loading = Indlæser /etc/locale.gen…
locale-management-load-failed = Kunne ikke læse /etc/locale.gen: { $reason }
locale-management-apply = Anvend
locale-management-applying = Anvender…
locale-management-apply-failed = Kunne ikke anvende: { $reason }
locale-management-apply-cancelled = Godkendelse annulleret eller afvist.
locale-management-helper-missing = Den privilegerede hjælper er ikke installeret på { $path }. Kør `sudo just install` fra projektets rodmappe, så Anvend-knappen kan omskrive /etc/locale.gen og køre locale-gen.

current-locale-reset = Nulstil tilsidesættelser til sprog
current-locale-reset-pending = Nulstiller…
current-locale-reset-cancelled = Godkendelse annulleret eller afvist.
current-locale-reset-failed = Kunne ikke nulstille tilsidesættelser: { $reason }
