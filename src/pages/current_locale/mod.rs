// SPDX-License-Identifier: GPL-3.0-only

//! Current locale page — shows the active LANG and LC_* values.
//!
//! Placeholder until the locale module lands.

use crate::app::{AppModel, Message};
use crate::fl;
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget;

pub fn view(_model: &AppModel) -> Element<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    widget::column::with_capacity(2)
        .push(widget::text::title1(fl!("page-current-locale")))
        .push(widget::text::body(fl!("coming-soon")))
        .spacing(space_s)
        .height(Length::Fill)
        .into()
}
