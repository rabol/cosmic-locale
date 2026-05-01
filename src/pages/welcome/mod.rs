// SPDX-License-Identifier: GPL-3.0-only

//! Welcome page — landing screen explaining what the app does.

use crate::app::{AppModel, Message};
use crate::fl;
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::widget;

pub fn view(_model: &AppModel) -> Element<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    widget::column::with_capacity(2)
        .push(widget::text::title1(fl!("welcome-title")))
        .push(widget::text::body(fl!("welcome-body")))
        .spacing(space_s)
        .height(Length::Fill)
        .into()
}
