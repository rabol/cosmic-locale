app-title = Cosmic Locale
about = Acerca de
repository = Repositorio
view = Ver

# Navigation
page-welcome = Bienvenida
page-current-locale = Configuración regional actual
page-locale-categories = Categorías de configuración regional
page-locale-management = Gestión de configuración regional

# Welcome page
welcome-title = Bienvenido a Cosmic Locale
welcome-body = Administra el idioma del sistema y la configuración regional.

welcome-pages-title = Qué puedes hacer aquí
welcome-page-current-locale = Ver el LANG, LANGUAGE y cualquier sobrescritura LC_* activa; restablecer las sobrescrituras a LANG.
welcome-page-locale-categories = Profundiza en las doce categorías POSIX con un selector por categoría y una vista previa en tiempo real.
welcome-page-locale-management = Elige qué configuraciones regionales se instalan editando /etc/locale.gen y ejecutando locale-gen.

welcome-summary-title = Resumen del sistema
welcome-summary-loading = Cargando…
welcome-summary-error = No se pudo leer la configuración regional del sistema: { $reason }
welcome-summary-lang-set = Idioma: { $lang }
welcome-summary-lang-unset = Idioma: no configurado
welcome-summary-overrides-none = Sin sobrescrituras de categoría.
welcome-summary-overrides-some = Sobrescrituras de categoría: { $count }.
welcome-summary-source-file = Cargado desde { $path }.
welcome-summary-source-env = Cargado desde el entorno del proceso.

welcome-actions-title = Acciones rápidas
welcome-action-current = Ver configuración regional actual
welcome-action-management = Administrar configuraciones regionales instaladas
welcome-action-language = Configurar idioma del sistema

# Placeholder shown on pages whose content has not yet been implemented.
coming-soon = Próximamente.

# Current locale page
current-locale-section = Configuración regional del sistema
current-locale-lang = Idioma (LANG)
current-locale-language = Lista de respaldo (LANGUAGE)
current-locale-overrides = Sobrescrituras de categoría
current-locale-source = Origen
current-locale-not-set = No configurado
current-locale-loading = Cargando…
current-locale-error = No se pudo leer la configuración regional del sistema: { $reason }
# Locale categories page
locale-categories-section = Categorías efectivas
locale-categories-source-override = Sobrescrita
locale-categories-source-inherited = Heredada de LANG
locale-categories-source-default = Predeterminada (C)
locale-categories-edit = Cambiar configuración regional…

# Category picker (context drawer)
picker-title = Cambiar { $category }
picker-system-language-title = Configurar idioma del sistema
picker-search-placeholder = Buscar configuraciones regionales
picker-empty = Sin coincidencias.
picker-loading = Cargando configuraciones regionales instaladas…
picker-load-failed = No se pudo listar las configuraciones regionales instaladas: { $reason }
picker-apply = Aplicar
picker-cancel = Cancelar
picker-applying = Aplicando…
picker-error = { $reason }
picker-preview-title = Vista previa
picker-preview-date = Fecha
picker-preview-time = Hora
picker-preview-datetime = Fecha y hora
picker-preview-number = Número
picker-preview-loading = Calculando vista previa…

# Locale management page
locale-management-section = Configuraciones regionales instaladas
locale-management-search-placeholder = Buscar configuraciones regionales
locale-management-empty = Ninguna configuración regional coincide con tu búsqueda.
locale-management-loading = Cargando /etc/locale.gen…
locale-management-load-failed = No se pudo leer /etc/locale.gen: { $reason }
locale-management-apply = Aplicar
locale-management-applying = Aplicando…
locale-management-apply-failed = No se pudo aplicar: { $reason }
locale-management-apply-cancelled = Autenticación cancelada o denegada.
locale-management-helper-missing = El asistente privilegiado no está instalado en { $path }. Ejecuta `sudo just install` desde la raíz del proyecto para que el botón Aplicar pueda reescribir /etc/locale.gen y ejecutar locale-gen.

current-locale-reset = Restablecer sobrescrituras al idioma
current-locale-reset-pending = Restableciendo…
current-locale-reset-cancelled = Autenticación cancelada o denegada.
current-locale-reset-failed = No se pudieron restablecer las sobrescrituras: { $reason }
