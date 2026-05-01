// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::fl;
use crate::locale::{self, LoadedLocale, LocaleError, LocaleSource};
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
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    CurrentLocaleLoaded(Result<LoadedLocale, LocaleError>),
    LaunchUrl(String),
    LcOverridesReset(Result<(), LocaleError>),
    ResetLcOverrides,
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
        };

        let title_command = app.update_title();
        let load_locale = cosmic::task::future(async {
            Message::CurrentLocaleLoaded(locale::read_default_locale().await)
        })
        .map(cosmic::Action::App);

        (app, Task::batch([title_command, load_locale]))
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
