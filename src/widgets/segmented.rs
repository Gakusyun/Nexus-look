//! A row of choices: one option per value, the current one washed in accent.
//!
//! `Segmented` picks a **value** (a limit, a count, a language, a theme). `Tabs` says which page
//! you are on and `Chip` says what a list is filtered to; shape is meaning, so the three never
//! look alike. See `STYLE.md` §10.
//!
//! Options are **equal width** rather than sized to their labels. The control column is a fixed
//! 320px, and the widest picker in the app (five speed limits, one of them a word in Chinese) does
//! not fit that column at its natural width — it would wrap and turn one row into two. Equal
//! widths are the only arrangement that cannot do that, and they read as a ruler rather than as a
//! bag of buttons.

use gpui::prelude::*;
use gpui::{
    App, ClickEvent, ElementId, FontWeight, IntoElement, ParentElement, SharedString, Styled,
    Window, div, px,
};
use std::rc::Rc;

use super::part_id;
use crate::theme::Theme;
use crate::tokens::{CONTROL, RADIUS, SEGMENT, SEGMENT_PAD, space, text};

/// What a control does when it is clicked.
///
/// A boxed handler rather than a generic parameter, because options are built in a loop and a
/// `Vec` needs one type: with a generic the call site would have to name the closure's type, which
/// is exactly the kind of noise that pushes people back to hand-rolling the control.
pub type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// One option of a [`Segmented`]: what it says, whether it is the current value, and what picking
/// it does.
pub struct Choice {
    label: SharedString,
    selected: bool,
    on_click: Handler,
}

impl Choice {
    pub fn new(
        label: impl Into<SharedString>,
        selected: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            selected,
            on_click: Rc::new(on_click),
        }
    }
}

#[derive(IntoElement)]
pub struct Segmented {
    id: ElementId,
    choices: Vec<Choice>,
}

impl Segmented {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            choices: Vec::new(),
        }
    }

    pub fn choices(mut self, choices: impl IntoIterator<Item = Choice>) -> Self {
        self.choices.extend(choices);
        self
    }
}

impl RenderOnce for Segmented {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let Self { id, choices } = self;
        let selected_bg = theme.accent_wash;
        let hover_bg = theme.surface_hover;

        div()
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap(px(space::XS))
            .p(px(SEGMENT_PAD))
            .h(px(CONTROL))
            .rounded(px(RADIUS))
            .border_1()
            .border_color(theme.border)
            .children(choices.into_iter().enumerate().map(|(index, choice)| {
                let Choice {
                    label,
                    selected,
                    on_click,
                } = choice;
                let handler = on_click;
                let (accent, muted, text) = (theme.accent, theme.text_muted, theme.text);

                div()
                    .id(part_id(&id, index.to_string()))
                    .flex()
                    .flex_1()
                    .min_w(px(0.0))
                    .items_center()
                    .justify_center()
                    .h(px(SEGMENT))
                    .px(px(space::MD))
                    .rounded(px(RADIUS))
                    .truncate()
                    .cursor_pointer()
                    .text_size(px(text::CAPTION))
                    // Selected is a wash, not a stroke and not a shadow: the option is already a
                    // rectangle inside a rectangle, and a second outline would read as a focus ring.
                    .text_color(if selected { accent } else { muted })
                    .font_weight(FontWeight::MEDIUM)
                    .when(selected, |element| element.bg(selected_bg))
                    // Only the unselected options light up: hovering the current value must not
                    // wash its accent away, which is what "selected, then hovered" used to do.
                    .when(!selected, |element| {
                        element.hover(move |style| style.bg(hover_bg).text_color(text))
                    })
                    .on_click(move |event, window, cx| handler(event, window, cx))
                    .child(label)
            }))
    }
}
