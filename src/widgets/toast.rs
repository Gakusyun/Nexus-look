//! A message that floats above the page instead of pushing it around.
//!
//! A transient notice is not part of the page. Putting it in the column means it shoves the list
//! down for the six seconds it lives, and then yanks it back — the layout moves twice for something
//! that is, by definition, about to leave. So it floats: absolutely positioned, centred at the top,
//! with the shadow that marks a real floating layer.
//!
//! It anchors to the nearest `relative()` ancestor rather than to the window, so the host decides
//! *where* the message belongs by where it puts it in the tree — under a toolbar, over a list —
//! and the library only decides how far from that edge it sits.
//!
//! Two things it deliberately does *not* have:
//!
//! * **No hitbox.** Clicks land on whatever is under it. A message the user is not meant to answer
//!   has no business eating a click aimed at the row behind it.
//! * **No exit animation.** The card fades and drops in; when the message is gone it is gone. An
//!   exit would need the app to keep a dead message alive for another 160ms, and there is nothing
//!   here worth that state machine.

use gpui::{
    AnimationExt, App, BoxShadow, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, px,
};

use super::{icon, settle};
use crate::icons;
use crate::theme::{Look, Theme, Tone};
use crate::tokens::{ICON_MD, RADIUS, motion, space, text};

#[derive(IntoElement)]
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
}

impl RenderOnce for Toast {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
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
            .top(px(space::XL))
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
                    .child(icon(glyph, ICON_MD, mark))
                    .child(message)
                    .shadow(vec![
                        BoxShadow::new(px(0.0), px(8.0), shadow).blur_radius(px(32.0)),
                    ])
                    .with_animation(
                        (id, "enter"),
                        settle(motion::BASE),
                        move |element, delta| {
                            // The card starts a step *above* where it settles and drops into place.
                            // `Styled::opacity` is what carries the fade through the icon and the
                            // text in the same pass as the card they sit on.
                            element
                                .opacity(delta)
                                .mt(px(-motion::SHIFT * (1.0 - delta)))
                        },
                    ),
            )
    }
}
