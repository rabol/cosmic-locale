// SPDX-License-Identifier: GPL-3.0-only

//! Welcome page — landing screen explaining what the app does, with
//! a live snapshot of the system's current locale and quick-action
//! buttons that deep-link to the other nav pages.

use crate::app::{AppModel, Message, Page, PickerTarget};
use crate::fl;
use crate::locale::{LoadedLocale, LocaleError, LocaleSource};
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget::{self, settings};

pub fn view(model: &AppModel) -> Element<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    widget::column::with_capacity(5)
        .push(widget::text::title1(fl!("welcome-title")))
        .push(widget::text::body(fl!("welcome-body")))
        .push(pages_section())
        .push(summary_section(model))
        .push(actions_section(model))
        .spacing(space_s)
        .height(Length::Fill)
        .into()
}

fn pages_section() -> Element<'static, Message> {
    let space_xxs = cosmic::theme::spacing().space_xxs;

    let entry = |title: String, desc: String| {
        widget::column::with_capacity(2)
            .push(widget::text::heading(title))
            .push(widget::text::body(desc))
            .spacing(space_xxs)
    };

    settings::section()
        .title(fl!("welcome-pages-title"))
        .add(entry(
            fl!("page-current-locale"),
            fl!("welcome-page-current-locale"),
        ))
        .add(entry(
            fl!("page-locale-categories"),
            fl!("welcome-page-locale-categories"),
        ))
        .add(entry(
            fl!("page-locale-management"),
            fl!("welcome-page-locale-management"),
        ))
        .into()
}

fn summary_section(model: &AppModel) -> Element<'_, Message> {
    let mut section = settings::section().title(fl!("welcome-summary-title"));

    match &model.current_locale {
        None => {
            section = section.add(widget::text::body(fl!("welcome-summary-loading")));
        }
        Some(Err(err)) => {
            section = section.add(widget::text::body(summary_error_text(err)));
        }
        Some(Ok(loaded)) => {
            for line in summary_lines(loaded) {
                section = section.add(widget::text::body(line));
            }
        }
    }

    section.into()
}

fn summary_lines(loaded: &LoadedLocale) -> Vec<String> {
    let mut out = Vec::with_capacity(3);

    let lang = match &loaded.settings.lang {
        Some(code) => fl!("welcome-summary-lang-set", lang = code.as_str().to_string()),
        None => fl!("welcome-summary-lang-unset"),
    };
    out.push(lang);

    // Count *real* overrides only — entries whose value differs from
    // LANG don't count, matching the Current locale page's display.
    let real_overrides = loaded
        .settings
        .lc_overrides
        .iter()
        .filter(|(_, value)| {
            loaded
                .settings
                .lang
                .as_ref()
                .is_none_or(|lang| value.as_str() != lang.as_str())
        })
        .count();
    out.push(if real_overrides == 0 {
        fl!("welcome-summary-overrides-none")
    } else {
        fl!("welcome-summary-overrides-some", count = real_overrides)
    });

    let source = match &loaded.source {
        LocaleSource::File(path) => {
            fl!(
                "welcome-summary-source-file",
                path = path.display().to_string()
            )
        }
        LocaleSource::Environment => fl!("welcome-summary-source-env"),
    };
    out.push(source);

    out
}

fn summary_error_text(err: &LocaleError) -> String {
    fl!("welcome-summary-error", reason = err.to_string())
}

fn actions_section(model: &AppModel) -> Element<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    let to_current = widget::button::standard(fl!("welcome-action-current"))
        .on_press(Message::SelectPage(Page::CurrentLocale));
    let to_management = widget::button::standard(fl!("welcome-action-management"))
        .on_press(Message::SelectPage(Page::LocaleManagement));

    let lang_value = model
        .current_locale
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(|loaded| loaded.settings.lang.as_ref())
        .map(|c| c.as_str().to_string());

    let mut to_language = widget::button::standard(fl!("welcome-action-language"));
    if let Some(current) = lang_value {
        to_language = to_language.on_press(Message::OpenLocalePicker {
            target: PickerTarget::SystemLanguage,
            current,
        });
    }

    widget::column::with_capacity(2)
        .push(widget::text::heading(fl!("welcome-actions-title")))
        .push(
            widget::row::with_capacity(3)
                .push(to_language)
                .push(to_current)
                .push(to_management)
                .spacing(space_s),
        )
        .spacing(space_s)
        .into()
}
