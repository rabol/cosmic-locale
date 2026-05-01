// SPDX-License-Identifier: GPL-3.0-only

//! Locale categories page — resolved per-`LC_*` view, including
//! inheritance from `LANG`. Each row carries an action button that
//! opens a picker in the context drawer.

use crate::app::{AppModel, Message};
use crate::fl;
use crate::locale::{
    self, CategorySource, CategoryView, LoadedLocale, LocaleCode, LocaleError, LocalePreview,
};
use cosmic::app::context_drawer;
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget::{self, icon, settings};

pub fn view(model: &AppModel) -> Element<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    let body: Element<'_, Message> = match &model.current_locale {
        None => widget::text::body(fl!("current-locale-loading")).into(),
        Some(Ok(loaded)) => render_loaded(loaded),
        Some(Err(err)) => render_error(err),
    };

    widget::column::with_capacity(2)
        .push(widget::text::title1(fl!("page-locale-categories")))
        .push(body)
        .spacing(space_s)
        .height(Length::Fill)
        .into()
}

fn render_loaded(loaded: &LoadedLocale) -> Element<'_, Message> {
    let mut section = settings::section().title(fl!("locale-categories-section"));

    for view in locale::effective_categories(&loaded.settings) {
        section = section.add(category_row(&view));
    }

    section.into()
}

fn category_row(view: &CategoryView) -> Element<'static, Message> {
    let space_xxs = cosmic::theme::spacing().space_xxs;

    let source_label = match view.source {
        CategorySource::Override => fl!("locale-categories-source-override"),
        CategorySource::Inherited => fl!("locale-categories-source-inherited"),
        CategorySource::Default => fl!("locale-categories-source-default"),
    };

    let value_with_source = format!("{}  ·  {}", view.value, source_label);
    let category_name = view.name.to_string();
    let current_value = view.value.clone();

    let edit_button = widget::button::icon(icon::from_name("view-more-symbolic"))
        .on_press(Message::OpenCategoryPicker {
            category: category_name.clone(),
            current: current_value,
        })
        .tooltip(fl!("locale-categories-edit"));

    let control = widget::row::with_capacity(2)
        .push(widget::text::body(value_with_source))
        .push(edit_button)
        .spacing(space_xxs)
        .align_y(cosmic::iced::Alignment::Center);

    settings::item::builder(category_name)
        .control(control)
        .into()
}

fn render_error(err: &LocaleError) -> Element<'_, Message> {
    widget::text::body(fl!("current-locale-error", reason = err.to_string())).into()
}

/// Build the locale picker view shown in the context drawer when a
/// category's three-dot button is clicked.
pub fn picker_drawer(model: &AppModel) -> context_drawer::ContextDrawer<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    let body: Element<'_, Message> = match (&model.picker, &model.available_locales) {
        (Some(picker), Some(Ok(available))) => render_picker_body(picker, available),
        (_, Some(Err(err))) => {
            widget::text::body(fl!("picker-load-failed", reason = err.to_string())).into()
        }
        _ => widget::text::body(fl!("picker-loading")).into(),
    };

    let title = model.picker.as_ref().map_or_else(
        || fl!("page-locale-categories"),
        |p| fl!("picker-title", category = p.category.clone()),
    );

    let footer = picker_footer(model);

    let content: Element<'_, Message> = widget::column::with_capacity(2)
        .push(body)
        .push(footer)
        .spacing(space_s)
        .into();

    context_drawer::context_drawer(content, Message::CategoryPickerCancel).title(title)
}

fn render_picker_body<'a>(
    picker: &'a crate::app::PickerState,
    available: &'a [LocaleCode],
) -> Element<'a, Message> {
    let space_s = cosmic::theme::spacing().space_s;
    let space_xxs = cosmic::theme::spacing().space_xxs;

    let search = widget::text_input(fl!("picker-search-placeholder"), picker.search.as_str())
        .on_input(Message::CategoryPickerSearch);

    let needle = picker.search.to_lowercase();
    let mut list = widget::column::with_capacity(available.len()).spacing(space_xxs);
    let mut shown = 0usize;
    for code in available {
        let s = code.as_str();
        if !needle.is_empty() && !s.to_lowercase().contains(&needle) {
            continue;
        }
        shown += 1;
        let is_selected = picker.selected == s;
        let label = if is_selected {
            format!("● {s}")
        } else {
            format!("  {s}")
        };
        list = list.push(
            widget::button::text(label)
                .on_press(Message::CategoryPickerSelect(s.to_string()))
                .width(Length::Fill),
        );
    }

    let list_element: Element<'a, Message> = if shown == 0 {
        widget::text::body(fl!("picker-empty")).into()
    } else {
        widget::scrollable(list).height(Length::Fill).into()
    };

    let mut col = widget::column::with_capacity(4)
        .push(search)
        .push(list_element)
        .push(preview_panel(picker))
        .spacing(space_s);

    if let Some(err) = &picker.last_error {
        col = col.push(widget::text::body(fl!(
            "picker-error",
            reason = err.to_string()
        )));
    }

    col.into()
}

fn preview_panel(picker: &crate::app::PickerState) -> Element<'_, Message> {
    let mut section = settings::section().title(fl!("picker-preview-title"));

    if let Some(preview) = preview_for(picker) {
        section = section
            .add(
                settings::item::builder(fl!("picker-preview-date"))
                    .control(widget::text::body(preview.date.clone())),
            )
            .add(
                settings::item::builder(fl!("picker-preview-time"))
                    .control(widget::text::body(preview.time.clone())),
            )
            .add(
                settings::item::builder(fl!("picker-preview-datetime"))
                    .control(widget::text::body(preview.datetime.clone())),
            )
            .add(
                settings::item::builder(fl!("picker-preview-number"))
                    .control(widget::text::body(preview.number.clone())),
            );
    } else {
        section = section.add(
            settings::item::builder(fl!("picker-preview-loading")).control(widget::text::body("")),
        );
    }

    section.into()
}

fn preview_for(picker: &crate::app::PickerState) -> Option<&LocalePreview> {
    let preview = picker.preview.as_ref()?;
    if preview.locale == picker.selected {
        Some(preview)
    } else {
        None
    }
}

fn picker_footer(model: &AppModel) -> Element<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    let cancel =
        widget::button::standard(fl!("picker-cancel")).on_press(Message::CategoryPickerCancel);

    let (apply_label, apply_enabled) = match &model.picker {
        Some(p) if p.in_flight => (fl!("picker-applying"), false),
        Some(p) => (fl!("picker-apply"), p.selected != p.initial_value),
        None => (fl!("picker-apply"), false),
    };

    let mut apply = widget::button::suggested(apply_label);
    if apply_enabled {
        apply = apply.on_press(Message::CategoryPickerApply);
    }

    widget::row::with_capacity(2)
        .push(cancel)
        .push(apply)
        .spacing(space_s)
        .into()
}
