//! A message that floats above the page instead of pushing it around.
//!
//! A transient notice is not part of the page. Putting it in the column means it shoves the list
//! down for the six seconds it lives, and then yanks it back — the layout moves twice for something
//! that is, by definition, about to leave. So it floats: absolutely positioned, centred at the
//! bottom, with the shadow that marks a real floating layer.
//!
//! Two things it deliberately does *not* have:
//!
//! * **No hitbox.** Clicks land on whatever is under it. A message the user is not meant to answer
//!   has no business eating a click aimed at the row behind it.
//! * **No exit animation.** The card fades and rises in; when the message is gone it is gone. An
//!   exit would need the app to keep a dead message alive for another 160ms, and there is nothing
//!   here worth that state machine.

use gpui::{
    AnimationExt, App, BoxShadow, ColorExt, ElementId, IntoElement, ParentElement, Rgba,
    SharedString, Styled, Window, div, px,
};

use super::{icon, settle};
use crate::icons;
use crate::theme::{Look, Theme, Tone};
use crate::tokens::{ICON_MD, RADIUS, motion, space, text};

pub struct Toast {
    id: ElementId,
    message: SharedString,
    tone: Tone,
    glyph: SharedString,
}

impl Toast {
    /// `id` only names the entry animation: a new message gets a new one and fades in again.
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            message: message.into(),
            tone: Tone::Warning,
            glyph: SharedString::from(icons::ALERT),
        }
    }

    /// What the message *is*. State, as everywhere else — the colour follows from it.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn icon(mut self, glyph: impl Into<SharedString>) -> Self {
        self.glyph = glyph.into();
        self
    }

    /// Place the toast. The parent must be `relative()`; this takes no room in its layout.
    pub fn build(self, window: &Window, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::of(cx);
        let font = Look::of(cx).font(window);
        let Self {
            id,
            message,
            tone,
            glyph,
        } = self;

        let surface = theme.surface;
        let border = theme.border;
        let ink = theme.text;
        let mark = theme.tone(tone);
        let shadow = theme.modal_shadow();

        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(space::XL))
            .flex()
            .flex_row()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(space::MD))
                    // One line of text, so it wraps instead of turning into a column.
                    .max_w(px(560.0))
                    .px(px(space::LG))
                    .py(px(space::MD))
                    .rounded(px(RADIUS))
                    .border_1()
                    .bg(surface)
                    .border_color(border)
                    .font(font)
                    .text_size(px(text::BODY))
                    .text_color(ink)
                    .shadow(vec![
                        BoxShadow::new(px(0.0), px(8.0), shadow).blur_radius(px(32.0)),
                    ])
                    .with_animation(
                        (id, "enter"),
                        settle(motion::BASE),
                        move |element, delta| {
                            // Element opacity does not exist in this version (`Styled` has no
                            // `opacity()`), so every colour carries the fade itself.
                            let fade = move |color: Rgba| color.opacity(delta);
                            element
                                .mt(px(motion::SHIFT * (1.0 - delta)))
                                .bg(fade(surface))
                                .border_color(fade(border))
                                .text_color(fade(ink))
                                .child(icon(glyph.clone(), ICON_MD, fade(mark)))
                                .child(message.clone())
                        },
                    ),
            )
    }
}
