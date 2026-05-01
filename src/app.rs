// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::fl;
use crate::locale::{
    self, LoadedLocale, LocaleCode, LocaleError, LocaleGen, LocalePreview, LocaleSource,
};
use crate::pages;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Length, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{self, about::About, icon, menu, nav_bar};
use std::collections::HashMap;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// The about page for this app.
    about: About,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Configuration data that persists between application runs.
    config: Config,
    /// Result of the most recent system locale read.
    ///
    /// `None` while the initial load is in flight.
    pub(crate) current_locale: Option<Result<LoadedLocale, LocaleError>>,
    /// `true` while a `localectl` reset is awaiting completion.
    pub(crate) reset_in_flight: bool,
    /// Error from the most recent reset attempt, if any. Cleared on the
    /// next attempt or on a successful reset.
    pub(crate) last_reset_error: Option<LocaleError>,
    /// Installed locales from `locale -a`. `None` while loading.
    pub(crate) available_locales: Option<Result<Vec<LocaleCode>, LocaleError>>,
    /// State of the per-category picker, if a category is being edited.
    pub(crate) picker: Option<PickerState>,
    /// Parsed `/etc/locale.gen` plus the user's pending edits.
    /// `None` while the initial load is in flight.
    pub(crate) locale_gen: Option<Result<LocaleGenState, LocaleError>>,
    /// Whether the privileged helper that the Locale management page
    /// invokes via `pkexec` is installed on disk. Captured once at
    /// startup; `just install` puts it in place.
    pub(crate) helper_installed: bool,
}

/// State for the locale-management page: the freshly-loaded baseline,
/// the user's currently-edited copy, search text, in-flight flag, and
/// the most recent apply error.
#[derive(Debug, Clone)]
pub struct LocaleGenState {
    pub initial: LocaleGen,
    pub current: LocaleGen,
    pub search: String,
    pub in_flight: bool,
    pub last_error: Option<LocaleError>,
}

impl LocaleGenState {
    /// Whether the user has changed anything from the loaded baseline.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.initial != self.current
    }
}

/// What the locale picker is going to write when the user clicks Apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerTarget {
    /// Set a single `LC_*` category.
    Category(String),
    /// Set the system language: writes `LANG` plus every `LC_*` to
    /// the chosen value, mirroring cosmic-settings' region picker.
    SystemLanguage,
}

/// State for the locale picker shown in the context drawer. The same
/// drawer/UI is used for changing one `LC_*` category and for the
/// "Set system language" action; only [`PickerState::target`]
/// differs.
#[derive(Debug, Clone)]
pub struct PickerState {
    pub target: PickerTarget,
    pub initial_value: String,
    pub selected: String,
    pub search: String,
    pub in_flight: bool,
    pub last_error: Option<LocaleError>,
    /// Cached preview for `selected`. Stale results (where the locale
    /// no longer matches `selected`) are dropped on arrival.
    pub preview: Option<LocalePreview>,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    AvailableLocalesLoaded(Result<Vec<LocaleCode>, LocaleError>),
    CategoryPickerApply,
    CategoryPickerCancel,
    CategoryPickerSearch(String),
    CategoryPickerSelect(String),
    CategoryUpdated(Result<(), LocaleError>),
    CurrentLocaleLoaded(Result<LoadedLocale, LocaleError>),
    LaunchUrl(String),
    LcOverridesReset(Result<(), LocaleError>),
    LocaleGenApplied(Result<(), LocaleError>),
    LocaleGenApply,
    LocaleGenLoaded(Result<LocaleGen, LocaleError>),
    LocaleGenSearch(String),
    LocaleGenToggle(usize),
    OpenLocalePicker {
        target: PickerTarget,
        current: String,
    },
    PreviewLoaded(LocalePreview),
    ResetLcOverrides,
    SelectPage(Page),
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "dev.rabol.cosmic-locale";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text(fl!("page-welcome"))
            .data::<Page>(Page::Welcome)
            .icon(icon::from_name("emblem-default-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("page-current-locale"))
            .data::<Page>(Page::CurrentLocale)
            .icon(icon::from_name("preferences-desktop-locale-symbolic"));

        nav.insert()
            .text(fl!("page-locale-categories"))
            .data::<Page>(Page::LocaleCategories)
            .icon(icon::from_name("view-list-symbolic"));

        nav.insert()
            .text(fl!("page-locale-management"))
            .data::<Page>(Page::LocaleManagement)
            .icon(icon::from_name("system-software-install-symbolic"));

        // Create the about widget
        let about = About::default()
            .name(fl!("app-title"))
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([(fl!("repository"), REPOSITORY)])
            .license(env!("CARGO_PKG_LICENSE"));

        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            about,
            nav,
            key_binds: HashMap::new(),
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((errors, config)) => {
                        for why in errors {
                            tracing::error!(%why, "error loading app config");
                        }

                        config
                    }
                })
                .unwrap_or_default(),
            current_locale: None,
            reset_in_flight: false,
            last_reset_error: None,
            available_locales: None,
            picker: None,
            locale_gen: None,
            helper_installed: locale::helper_installed(),
        };

        if !app.helper_installed {
            tracing::warn!(
                path = locale::APPLY_LOCALE_GEN_HELPER,
                "privileged helper not installed; Apply on the locale management page will be disabled until `sudo just install` has been run"
            );
        }

        let title_command = app.update_title();
        let load_locale = cosmic::task::future(async {
            Message::CurrentLocaleLoaded(locale::read_default_locale().await)
        })
        .map(cosmic::Action::App);
        let load_available = cosmic::task::future(async {
            Message::AvailableLocalesLoaded(locale::list_installed_locales().await)
        })
        .map(cosmic::Action::App);
        let load_locale_gen = cosmic::task::future(async {
            Message::LocaleGenLoaded(locale::read_locale_gen().await)
        })
        .map(cosmic::Action::App);

        (
            app,
            Task::batch([title_command, load_locale, load_available, load_locale_gen]),
        )
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            ),
            ContextPage::LocalePicker => pages::locale_categories::picker_drawer(self),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        let page = self
            .nav
            .active_data::<Page>()
            .copied()
            .unwrap_or(Page::Welcome);

        let content: Element<_> = match page {
            Page::Welcome => pages::welcome::view(self),
            Page::CurrentLocale => pages::current_locale::view(self),
            Page::LocaleCategories => pages::locale_categories::view(self),
            Page::LocaleManagement => pages::locale_management::view(self),
        };

        widget::container(content)
            .width(600)
            .height(Length::Fill)
            .apply(widget::container)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They can be dynamically
    /// stopped and started conditionally based on application state, or persist
    /// indefinitely.
    fn subscription(&self) -> Subscription<Self::Message> {
        // Watch for application configuration changes.
        self.core()
            .watch_config::<Config>(Self::APP_ID)
            .map(|update| {
                for why in update.errors {
                    tracing::error!(?why, "app config error");
                }

                Message::UpdateConfig(update.config)
            })
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    #[allow(clippy::too_many_lines)]
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::CurrentLocaleLoaded(result) => {
                if let Err(why) = &result {
                    tracing::error!(%why, "failed to read system locale");
                }
                self.current_locale = Some(result);
            }

            Message::ResetLcOverrides => {
                let Some(Ok(loaded)) = &self.current_locale else {
                    return Task::none();
                };
                if !matches!(loaded.source, LocaleSource::File(_))
                    || loaded.settings.lc_overrides.is_empty()
                    || self.reset_in_flight
                {
                    return Task::none();
                }

                self.reset_in_flight = true;
                self.last_reset_error = None;

                return cosmic::task::future(async {
                    Message::LcOverridesReset(locale::reset_lc_overrides().await)
                })
                .map(cosmic::Action::App);
            }

            Message::LcOverridesReset(result) => {
                self.reset_in_flight = false;
                match result {
                    Ok(()) => {
                        return cosmic::task::future(async {
                            Message::CurrentLocaleLoaded(locale::read_default_locale().await)
                        })
                        .map(cosmic::Action::App);
                    }
                    Err(err) => {
                        tracing::error!(%err, "failed to reset LC_* overrides");
                        self.last_reset_error = Some(err);
                    }
                }
            }

            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }

            Message::UpdateConfig(config) => {
                self.config = config;
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    tracing::error!(%err, %url, "failed to open url");
                }
            },

            Message::AvailableLocalesLoaded(result) => {
                if let Err(why) = &result {
                    tracing::error!(%why, "failed to list installed locales");
                }
                self.available_locales = Some(result);
            }

            Message::OpenLocalePicker { target, current } => {
                self.picker = Some(PickerState {
                    target,
                    initial_value: current.clone(),
                    selected: current.clone(),
                    search: String::new(),
                    in_flight: false,
                    last_error: None,
                    preview: None,
                });
                self.context_page = ContextPage::LocalePicker;
                self.core.window.show_context = true;

                return cosmic::task::future(async move {
                    Message::PreviewLoaded(locale::preview_locale(current).await)
                })
                .map(cosmic::Action::App);
            }

            Message::CategoryPickerSearch(search) => {
                if let Some(picker) = self.picker.as_mut() {
                    picker.search = search;
                }
            }

            Message::CategoryPickerSelect(value) => {
                let Some(picker) = self.picker.as_mut() else {
                    return Task::none();
                };
                if picker.selected == value {
                    return Task::none();
                }
                picker.selected.clone_from(&value);
                picker.last_error = None;
                // Drop the stale preview so the panel doesn't show
                // values for the previously-selected locale while the
                // new one is being computed.
                picker.preview = None;

                return cosmic::task::future(async move {
                    Message::PreviewLoaded(locale::preview_locale(value).await)
                })
                .map(cosmic::Action::App);
            }

            Message::PreviewLoaded(preview) => {
                if let Some(picker) = self.picker.as_mut()
                    && picker.selected == preview.locale
                {
                    picker.preview = Some(preview);
                }
            }

            Message::CategoryPickerCancel => {
                self.picker = None;
                self.core.window.show_context = false;
            }

            Message::CategoryPickerApply => {
                let Some(picker) = self.picker.as_mut() else {
                    return Task::none();
                };
                if picker.in_flight || picker.selected == picker.initial_value {
                    return Task::none();
                }

                picker.in_flight = true;
                picker.last_error = None;

                let target = picker.target.clone();
                let value = picker.selected.clone();

                return cosmic::task::future(async move {
                    let result = match target {
                        PickerTarget::Category(name) => locale::set_category(&name, &value).await,
                        PickerTarget::SystemLanguage => locale::set_system_language(&value).await,
                    };
                    Message::CategoryUpdated(result)
                })
                .map(cosmic::Action::App);
            }

            Message::CategoryUpdated(result) => {
                match result {
                    Ok(()) => {
                        // Close the picker and refresh the page.
                        self.picker = None;
                        self.core.window.show_context = false;
                        return cosmic::task::future(async {
                            Message::CurrentLocaleLoaded(locale::read_default_locale().await)
                        })
                        .map(cosmic::Action::App);
                    }
                    Err(err) => {
                        tracing::error!(%err, "failed to update LC_* category");
                        if let Some(picker) = self.picker.as_mut() {
                            picker.in_flight = false;
                            picker.last_error = Some(err);
                        }
                    }
                }
            }

            Message::LocaleGenLoaded(result) => {
                if let Err(why) = &result {
                    tracing::error!(%why, "failed to read /etc/locale.gen");
                }
                self.locale_gen = Some(result.map(|loaded| LocaleGenState {
                    initial: loaded.clone(),
                    current: loaded,
                    search: String::new(),
                    in_flight: false,
                    last_error: None,
                }));
            }

            Message::LocaleGenSearch(search) => {
                if let Some(Ok(state)) = self.locale_gen.as_mut() {
                    state.search = search;
                }
            }

            Message::LocaleGenToggle(line_index) => {
                if let Some(Ok(state)) = self.locale_gen.as_mut() {
                    state.current.toggle(line_index);
                    state.last_error = None;
                }
            }

            Message::LocaleGenApply => {
                let Some(Ok(state)) = self.locale_gen.as_mut() else {
                    return Task::none();
                };
                if state.in_flight || !state.is_dirty() {
                    return Task::none();
                }
                state.in_flight = true;
                state.last_error = None;
                let to_apply = state.current.clone();
                return cosmic::task::future(async move {
                    Message::LocaleGenApplied(locale::apply_locale_gen(&to_apply).await)
                })
                .map(cosmic::Action::App);
            }

            Message::SelectPage(target_page) => {
                let entity = self
                    .nav
                    .iter()
                    .find(|&entity| self.nav.data::<Page>(entity).copied() == Some(target_page));
                if let Some(entity) = entity {
                    self.nav.activate(entity);
                    return self.update_title();
                }
            }

            Message::LocaleGenApplied(result) => {
                if let Some(Ok(state)) = self.locale_gen.as_mut() {
                    state.in_flight = false;
                    match result {
                        Ok(()) => {
                            state.initial = state.current.clone();
                        }
                        Err(err) => {
                            tracing::error!(%err, "failed to apply /etc/locale.gen");
                            state.last_error = Some(err);
                            return Task::none();
                        }
                    }
                }
                // After a successful apply, refresh the installed list so
                // the categories picker reflects the newly-generated locales.
                return cosmic::task::future(async {
                    Message::AvailableLocalesLoaded(locale::list_installed_locales().await)
                })
                .map(cosmic::Action::App);
            }
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        // Activate the page in the model.
        self.nav.activate(id);

        self.update_title()
    }
}

impl AppModel {
    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
}

/// Identifies a top-level page in the application.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Page {
    Welcome,
    CurrentLocale,
    LocaleCategories,
    LocaleManagement,
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
    /// Locale picker — handles both per-category edits and the
    /// "Set system language" action; the actual mode is carried
    /// by [`PickerState::target`].
    LocalePicker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}
