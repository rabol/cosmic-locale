// SPDX-License-Identifier: GPL-3.0-only

//! Locale management page — multi-select view of `/etc/locale.gen`.
//! Toggle entries on or off and apply via `pkexec` to write the file
//! and run `locale-gen`.

use crate::app::{AppModel, LocaleGenState, Message};
use crate::fl;
use crate::locale::{self, LocaleError};
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget::{self, settings};

pub fn view(model: &AppModel) -> Element<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    let body: Element<'_, Message> = match &model.locale_gen {
        None => widget::text::body(fl!("locale-management-loading")).into(),
        Some(Err(err)) => render_load_error(err),
        Some(Ok(state)) => render_loaded(state, model.helper_installed),
    };

    widget::column::with_capacity(2)
        .push(widget::text::title1(fl!("page-locale-management")))
        .push(body)
        .spacing(space_s)
        .height(Length::Fill)
        .into()
}

fn render_loaded(state: &LocaleGenState, helper_installed: bool) -> Element<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    let search = widget::text_input(
        fl!("locale-management-search-placeholder"),
        state.search.as_str(),
    )
    .on_input(Message::LocaleGenSearch);

    let needle = state.search.to_lowercase();
    let mut list = widget::column::with_capacity(state.current.entries().count());
    let mut shown = 0usize;
    for (line_index, entry) in state.current.entries() {
        if !needle.is_empty()
            && !entry.code.to_lowercase().contains(&needle)
            && !entry.charset.to_lowercase().contains(&needle)
        {
            continue;
        }
        shown += 1;
        let label = format!("{} ({})", entry.code, entry.charset);
        list = list.push(
            widget::checkbox(entry.enabled)
                .label(label)
                .on_toggle(move |_| Message::LocaleGenToggle(line_index)),
        );
    }

    let list_element: Element<'_, Message> = if shown == 0 {
        widget::text::body(fl!("locale-management-empty")).into()
    } else {
        widget::scrollable(list).height(Length::Fill).into()
    };

    let footer = render_footer(state, helper_installed);

    let mut column = widget::column::with_capacity(5)
        .push(search)
        .push(list_element)
        .push(footer)
        .spacing(space_s);

    if !helper_installed {
        column = column.push(widget::text::body(fl!(
            "locale-management-helper-missing",
            path = locale::APPLY_LOCALE_GEN_HELPER
        )));
    }

    if let Some(err) = &state.last_error {
        column = column.push(widget::text::body(apply_error_message(err)));
    }

    settings::section()
        .title(fl!("locale-management-section"))
        .add(column)
        .into()
}

fn render_footer(state: &LocaleGenState, helper_installed: bool) -> Element<'_, Message> {
    let label = if state.in_flight {
        fl!("locale-management-applying")
    } else {
        fl!("locale-management-apply")
    };
    let mut button = widget::button::suggested(label);
    if !state.in_flight && state.is_dirty() && helper_installed {
        button = button.on_press(Message::LocaleGenApply);
    }
    button.into()
}

fn render_load_error(err: &LocaleError) -> Element<'_, Message> {
    widget::text::body(fl!(
        "locale-management-load-failed",
        reason = err.to_string()
    ))
    .into()
}

fn apply_error_message(err: &LocaleError) -> String {
    match err {
        LocaleError::Cancelled => fl!("locale-management-apply-cancelled"),
        other => fl!("locale-management-apply-failed", reason = other.to_string()),
    }
}
