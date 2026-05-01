// SPDX-License-Identifier: GPL-3.0-only

//! Current locale page — shows the active LANG, LANGUAGE, and any LC_*
//! overrides parsed from `/etc/default/locale` or `/etc/locale.conf`.

use crate::app::{AppModel, Message};
use crate::fl;
use crate::locale::{LoadedLocale, LocaleError, LocaleSource};
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget::{self, settings};

pub fn view(model: &AppModel) -> Element<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    let body: Element<'_, Message> = match &model.current_locale {
        None => widget::text::body(fl!("current-locale-loading")).into(),
        Some(Ok(loaded)) => render_loaded(model, loaded),
        Some(Err(err)) => render_error(err),
    };

    widget::column::with_capacity(2)
        .push(widget::text::title1(fl!("page-current-locale")))
        .push(body)
        .spacing(space_s)
        .height(Length::Fill)
        .into()
}

fn render_loaded<'a>(model: &'a AppModel, loaded: &'a LoadedLocale) -> Element<'a, Message> {
    let space_s = cosmic::theme::spacing().space_s;
    let settings_data = &loaded.settings;

    let lang_value = settings_data
        .lang
        .as_ref()
        .map_or_else(|| fl!("current-locale-not-set"), |c| c.as_str().to_string());

    let mut main_section = settings::section()
        .title(fl!("current-locale-section"))
        .add(
            settings::item::builder(fl!("current-locale-lang"))
                .control(widget::text::body(lang_value)),
        );

    if let Some(language) = settings_data.language.as_deref() {
        main_section = main_section.add(
            settings::item::builder(fl!("current-locale-language"))
                .control(widget::text::body(language.to_string())),
        );
    }

    if let LocaleSource::File(path) = &loaded.source {
        main_section = main_section.add(
            settings::item::builder(fl!("current-locale-source"))
                .control(widget::text::body(path.display().to_string())),
        );
    }

    let mut column = widget::column::with_capacity(4)
        .push(main_section)
        .spacing(space_s);

    // An LC_* entry is only a meaningful "override" if its value differs
    // from LANG. After a reset systemd-localed leaves the LC_* entries
    // in /etc/locale.conf with values matching LANG; we hide those so
    // the section and button disappear as the user expects.
    let real_overrides: Vec<(&String, &str)> = settings_data
        .lc_overrides
        .iter()
        .filter(|(_, value)| {
            settings_data
                .lang
                .as_ref()
                .is_none_or(|lang| value.as_str() != lang.as_str())
        })
        .map(|(k, v)| (k, v.as_str()))
        .collect();

    if !real_overrides.is_empty() {
        let mut overrides = settings::section().title(fl!("current-locale-overrides"));
        for (key, value) in &real_overrides {
            overrides = overrides.add(
                settings::item::builder((*key).clone())
                    .control(widget::text::body((*value).to_string())),
            );
        }
        column = column.push(overrides);

        if matches!(loaded.source, LocaleSource::File(_)) {
            let label = if model.reset_in_flight {
                fl!("current-locale-reset-pending")
            } else {
                fl!("current-locale-reset")
            };
            let mut reset_button = widget::button::standard(label);
            if !model.reset_in_flight {
                reset_button = reset_button.on_press(Message::ResetLcOverrides);
            }
            column = column.push(reset_button);

            if let Some(err) = &model.last_reset_error {
                column = column.push(widget::text::body(reset_error_message(err)));
            }
        }
    }

    column.into()
}

fn render_error(err: &LocaleError) -> Element<'_, Message> {
    widget::text::body(fl!("current-locale-error", reason = err.to_string())).into()
}

fn reset_error_message(err: &LocaleError) -> String {
    match err {
        LocaleError::Cancelled => fl!("current-locale-reset-cancelled"),
        other => fl!("current-locale-reset-failed", reason = other.to_string()),
    }
}
