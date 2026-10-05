//! A switch: the shape a boolean takes.
//!
//! `Segmented` picks a *value* out of a set of labels; a switch says one setting is on or off.
//! Two labels in a two-cell control read as two more values to weigh, while a knob sitting at one
//! end reads as a state — which is exactly why a boolean does not get a `Segmented` with "开/关"
//! written in it. See `STYLE.md` §10.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    App, ClickEvent, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px,
};

use super::lift;
use crate::theme::Theme;
use crate::tokens::{CONTROL, RADIUS, SWITCH, SWITCH_KNOB, SWITCH_PAD};

/// What a switch reports when clicked: the value *after* the toggle, so a handler never has to
/// work out — and possibly get wrong — which state it is moving to.
pub type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

/// On or off.
///
/// `CONTROL` tall like every other control in a row, and the knob's travel is a deliberate
/// 16 px: enough to read as a move, small enough that the whole thing stays a rectangle you can
/// hit rather than a track you have to aim at.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    on: bool,
    handler: Option<ChangeHandler>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            on: false,
            handler: None,
        }
    }

    /// The state it is showing. Off is the default, so `.new(..)` alone is a working control.
    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.handler = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let Self { id, on, handler } = self;
        let dark = theme.is_dark;
        // Two colours, each with its own step: the track's base one notch darker on hover and two
        // on press — the same ramp `Button` uses, and the same rule (§6.1: a step, not a tween).
        let (track, knob) = if on {
            (theme.accent, theme.on_accent)
        } else {
            (theme.border, theme.surface)
        };

        div()
            .id(id)
            .relative()
            .w(px(SWITCH))
            .h(px(CONTROL))
            .rounded(px(RADIUS))
            .bg(track)
            .cursor_pointer()
            .hover(move |style| style.bg(lift(track, dark, 0.08)))
            .active(move |style| style.bg(lift(track, dark, 0.16)))
            .when_some(handler, |element, handler| {
                element.on_click(move |event, window, cx| handler(!on, event, window, cx))
            })
            .child(
                // Positioned rather than laid out: the knob must never be able to change the
                // track's size, and it jumps — motion belongs to things appearing and
                // disappearing, not to a control being held down.
                div()
                    .absolute()
                    .left(px(if on {
                        SWITCH - SWITCH_KNOB - SWITCH_PAD
                    } else {
                        SWITCH_PAD
                    }))
                    .top(px(SWITCH_PAD))
                    .size(px(SWITCH_KNOB))
                    .rounded(px(RADIUS))
                    .bg(knob),
            )
    }
}
