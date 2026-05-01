// SPDX-License-Identifier: GPL-3.0-only

//! Top-level page modules. Each subfolder owns one nav-bar page.
//!
//! Pages currently expose a single `view(&AppModel)` free function;
//! per-page state and messages can be split out later if any page
//! grows enough to justify it.

pub mod current_locale;
pub mod locale_categories;
pub mod locale_management;
pub mod welcome;
