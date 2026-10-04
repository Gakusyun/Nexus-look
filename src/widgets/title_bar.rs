//! The title bar, and the four window controls that live at its right end.
//!
//! Two platform details decide the shape of this widget, and both are the kind of thing every app
//! gets wrong once:
//!
//! * **The drag strip must be a *sibling* of the window buttons, never their ancestor.** The
//!   platform resolves a hit test by walking the window-control hitboxes in registration order and
//!   taking the first match, and paint registers a parent before its children. So a `Drag` area on
//!   an ancestor captures the whole strip: pressing Minimise returns `HTCAPTION` and the window
//!   just drags instead.
//! * **`WindowControlArea::Max` is already a toggle.** It maps to `HTMAXBUTTON`, which the OS
//!   switches between maximise and restore on its own. Only the glyph has to follow the state, and
//!   that is what `window.is_maximized()` is read for.
//!
//! The window buttons take no click handler: they hand the platform an area and it owns the click.

use gpui::prelude::*;
use gpui::{
    AnyElement, App, IntoElement, ParentElement, SharedString, Styled, Window, WindowControlArea,
    div, px,
};

use super::{IconButton, icon};
use crate::icons;
use crate::theme::{Look, Theme};
use crate::tokens::{ICON_MD, TITLEBAR, space, text};

/// What the app puts in its title bar beyond the four window controls.
pub struct TitleBar {
    title: SharedString,
    logo: Option<SharedString>,
    actions: Vec<AnyElement>,
}

impl TitleBar {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            logo: None,
            actions: Vec::new(),
        }
    }

    /// The product mark, drawn in the accent. A path in *your* asset source — the library has no
    /// opinion about what your app looks like.
    pub fn logo(mut self, path: impl Into<SharedString>) -> Self {
        self.logo = Some(path.into());
        self
    }

    /// A button that belongs next to the window controls — in practice, the gear.
    ///
    /// It is deliberately *not* a window control: it sits outside the drag strip, so it gets its own
    /// hitbox and an ordinary click handler.
    pub fn action(mut self, element: impl IntoElement) -> Self {
        self.actions.push(element.into_any_element());
        self
    }

    /// Returns a concrete element tree: nothing in it borrows the window or the theme, so callers
    /// can hand it straight to a parent without carrying either lifetime along.
    pub fn build(self, window: &Window, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::of(cx);
        let font = Look::of(cx).font(window);
        let Self {
            title,
            logo,
            actions,
        } = self;
        let maximized = window.is_maximized();

        div()
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .h(px(TITLEBAR))
            // The gutter matches the page's, so the logo lines up with the content below it.
            .pl(px(space::XL))
            .pr(px(space::XS))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .flex_1()
                    .h_full()
                    .gap(px(space::SM))
                    .window_control_area(WindowControlArea::Drag)
                    .children(logo.map(|path| icon(path, ICON_MD, theme.accent)))
                    .child(
                        div()
                            .text_size(px(text::BODY_STRONG))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .font(font)
                            .child(title),
                    ),
            )
            .children(actions)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .flex_none()
                    .child(
                        IconButton::new("window-min", icons::MINIMIZE)
                            .window(WindowControlArea::Min),
                    )
                    // `Max` either way: the OS toggles the window; only the glyph follows.
                    .child(
                        IconButton::new(
                            "window-max",
                            if maximized {
                                icons::RESTORE
                            } else {
                                icons::MAXIMIZE
                            },
                        )
                        .window(WindowControlArea::Max),
                    )
                    .child(
                        IconButton::new("window-close", icons::CLOSE)
                            .danger()
                            .window(WindowControlArea::Close),
                    ),
            )
    }
}
