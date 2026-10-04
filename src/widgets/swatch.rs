//! The accent picker: eight presets, a hex box for the ninth, and the way back to the default.
//!
//! The library owns this rather than each app because the presets are half the design language —
//! see [`PRESET_ACCENTS`] — and because "choose the app's colour" is a settings row that should
//! look and behave identically everywhere.
//!
//! Three decisions are baked in:
//!
//! * **The swatch is a fixed [`SWATCH`], not a size a caller can tune.** It is a target a finger
//!   aims at, and two projects with two sizes read as two different widgets.
//! * **The selected ring is `text`, never a semantic colour and never the accent.** The swatch
//!   already *is* the accent; ringing it in itself would say nothing, and red would say "wrong".
//! * **The dot marks the default, inside the swatch, in `readable_on` of that colour.** It is a
//!   fact about the swatch ("this is the one you get for free"), so it lives with the swatch
//!   instead of in a legend nobody reads.
//!
//! Labels, the hex box and the two buttons come from the caller: strings never live here.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px,
    rgb,
};

use super::{lift, part_id};
use crate::theme::{PRESET_ACCENTS, Theme, readable_on};
use crate::tokens::{SWATCH, SWATCH_DOT, space};

/// What picking a colour does.
///
/// Takes the value by reference because that is what `cx.listener` hands back — the same reason
/// `Segmented::Choice`'s handler does (see the note there).
pub type PickHandler = Rc<dyn Fn(&u32, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct SwatchGrid {
    id: ElementId,
    selected: u32,
    default: u32,
    /// The caller's hex box, present only while their custom row is open.
    hex: Option<AnyElement>,
    /// The caller's buttons — the custom toggle, the reset. Right-aligned, under everything else.
    actions: Vec<AnyElement>,
    on_pick: Option<PickHandler>,
}

impl SwatchGrid {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            selected: 0,
            default: 0,
            hex: None,
            actions: Vec::new(),
            on_pick: None,
        }
    }

    /// The accent in force. One swatch gets the ring; the preview beside the hex box shows it too,
    /// which is what makes typing a hex feel live rather than hopeful.
    pub fn selected(mut self, value: u32) -> Self {
        self.selected = value;
        self
    }

    /// The colour "reset" goes back to, marked with the dot. Pass the same value the app was
    /// installed with, so the dot and the button mean one colour.
    pub fn default(mut self, value: u32) -> Self {
        self.default = value;
        self
    }

    /// The hex box, while the caller has the custom row open. It takes what is left of the column
    /// and the preview sits beside it.
    pub fn hex(mut self, field: impl IntoElement) -> Self {
        self.hex = Some(field.into_any_element());
        self
    }

    /// A button that belongs to this control: the custom toggle, the reset.
    pub fn action(mut self, button: impl IntoElement) -> Self {
        self.actions.push(button.into_any_element());
        self
    }

    /// A preset was chosen (or the reset was pressed — the caller decides in one handler).
    pub fn on_pick(mut self, handler: impl Fn(&u32, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SwatchGrid {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let Self {
            id,
            selected,
            default,
            hex,
            actions,
            on_pick,
        } = self;

        let presets = div()
            .flex()
            .flex_row()
            .flex_none()
            .gap(px(space::SM))
            .children(PRESET_ACCENTS.iter().enumerate().map(|(index, &value)| {
                // Instant, one colour, no transition — see `STYLE.md` §6.1. The lift is the same
                // 8% the rest of the language hovers with, so the dot brightens rather than
                // acquiring an outline of its own.
                let hover = lift(rgb(value), theme.is_dark, 0.08);
                swatch(value, value == selected, value == default, &theme)
                    .id(part_id(&id, index.to_string()))
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .when_some(on_pick.clone(), |cell, handler| {
                        cell.on_click(move |_, window, cx| handler(&value, window, cx))
                    })
            }));

        let mut grid = div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(space::SM))
            .child(presets);

        if let Some(hex) = hex {
            grid = grid.child(
                div()
                    .flex()
                    .flex_row()
                    .flex_none()
                    .items_center()
                    .gap(px(space::SM))
                    .child(div().flex_1().min_w(px(0.0)).child(hex))
                    .child(swatch(selected, false, false, &theme)),
            );
        }

        if !actions.is_empty() {
            grid = grid.child(
                div()
                    .flex()
                    .flex_row()
                    .flex_none()
                    .items_center()
                    .justify_end()
                    .gap(px(space::SM))
                    .children(actions),
            );
        }

        grid
    }
}

/// One disc: the colour, the ring if it is the one in force, the dot if it is the default.
///
/// Both marks are **overlays** rather than borders on the cell itself, so picking a swatch never
/// changes its size or the row's rhythm — a selection that nudges its neighbours would make the
/// grid feel loose under the cursor.
fn swatch(value: u32, selected: bool, is_default: bool, theme: &Theme) -> gpui::Div {
    let colour = rgb(value);
    div()
        .relative()
        .flex_none()
        .size(px(SWATCH))
        .rounded_full()
        .bg(colour)
        .when(selected, |cell| {
            cell.child(
                div()
                    .absolute()
                    .size_full()
                    .rounded_full()
                    .border_2()
                    .border_color(theme.text),
            )
        })
        .when(is_default, |cell| {
            cell.child(
                div()
                    .absolute()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .size(px(SWATCH_DOT))
                            .rounded_full()
                            .bg(readable_on(colour)),
                    ),
            )
        })
}
