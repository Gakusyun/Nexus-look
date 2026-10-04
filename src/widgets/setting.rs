//! The settings table: a group of rows, each row a label on the left and one control on the right.
//!
//! The new-download dialog is the same table with different rows, which is why these two live in
//! the library rather than in the app: a dialog and a settings page that share a shape read as one
//! product, and the moment one of them hand-rolls a row the two drift apart.
//!
//! Three rules are baked in because breaking any of them is invisible until it is ugly:
//!
//! * **The control column is a fixed [`CONTROL_COL`] and every row's right edge is the same one.**
//!   A control that sized itself would line up with nothing, and a long label would squeeze it.
//! * **A row is at least [`CONTROL_LG`] tall**, so a column of rows keeps its rhythm whether or not
//!   a label happens to carry a note.
//! * **A group is a heading plus rows, never a card.** A sheet is one column down a page; cards
//!   inside a card would give the eye two rectangles to sort out for every group.

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Div, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, px,
};

use crate::theme::Theme;
use crate::tokens::{CONTROL_LG, layout, space, text};

/// One group of a settings table: a heading, an optional one-line summary, then rows.
///
/// An empty summary is dropped, so a group whose rows already say what it is needs no summary.
#[derive(IntoElement)]
pub struct SettingGroup {
    title: SharedString,
    summary: SharedString,
    children: Vec<AnyElement>,
}

impl SettingGroup {
    pub fn new(title: impl Into<SharedString>, summary: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            summary: summary.into(),
            children: Vec::new(),
        }
    }

    /// A row: label and note on the left, the control on the right.
    pub fn row(
        self,
        label: impl Into<SharedString>,
        note: impl Into<SharedString>,
        control: impl IntoElement,
    ) -> Self {
        self.child(Row::new(label, note).control(control))
    }

    /// Anything else the group holds: a subheading, a hint.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl RenderOnce for SettingGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let Self {
            title,
            summary,
            children,
        } = self;
        let has_summary = !summary.is_empty();

        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(space::MD))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .gap(px(space::XS))
                    .child(
                        div()
                            .text_size(px(text::HEADING))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child(title),
                    )
                    .when(has_summary, |element| {
                        element.child(
                            div()
                                .text_size(px(text::CAPTION))
                                .text_color(theme.text_faint)
                                .child(summary),
                        )
                    }),
            )
            .children(children)
    }
}

/// The second tier of a group that outgrew one heading, like the engine's concurrency settings.
///
/// A heading with no rule under it and no box around it: it is a break in the list, not a section.
pub fn subheading(label: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .pt(px(space::SM))
        .text_size(px(text::CAPTION))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(Theme::of(cx).text_muted)
        .child(label.into())
}

/// One row of a settings table. A builder rather than a function because a row has a second slot
/// that only some rows use: the button that acts on the control beside it.
#[derive(IntoElement)]
pub struct Row {
    label: SharedString,
    note: SharedString,
    control: Option<AnyElement>,
    action: Option<AnyElement>,
}

impl Row {
    pub fn new(label: impl Into<SharedString>, note: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            note: note.into(),
            control: None,
            action: None,
        }
    }

    /// The control in the right-hand column: a field, a picker, a read-only value.
    pub fn control(mut self, control: impl IntoElement) -> Self {
        self.control = Some(control.into_any_element());
        self
    }

    /// A button that acts on the control beside it — "change this folder", "open this file".
    ///
    /// It shares the control column instead of getting one of its own, so the row still has one
    /// thing in it and the column's right edge still lines up with every other row's.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.action = Some(action.into_any_element());
        self
    }
}

impl RenderOnce for Row {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let Self {
            label,
            note,
            control,
            action,
        } = self;
        let has_note = !note.is_empty();

        let mut cell = div()
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .w(px(layout::CONTROL_COL))
            .gap(px(space::SM));
        if let Some(control) = control {
            // The action, when there is one, is the only part with a size of its own; the control
            // takes what is left, which is what keeps a field from being the widest thing in a row
            // that also holds a button.
            cell = if action.is_some() {
                cell.child(div().flex_1().min_w(px(0.0)).child(control))
            } else {
                cell.child(div().flex_none().w_full().child(control))
            };
        }
        if let Some(action) = action {
            cell = cell.child(div().flex_none().child(action));
        }

        div()
            .flex()
            .flex_row()
            .flex_none()
            .items_center()
            .gap(px(space::XL))
            .min_h(px(CONTROL_LG))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w(px(0.0))
                    .gap(px(space::XS))
                    .child(
                        div()
                            .text_size(px(text::BODY))
                            .text_color(theme.text)
                            .child(label),
                    )
                    .when(has_note, |element| {
                        element.child(
                            div()
                                .text_size(px(text::CAPTION))
                                .text_color(theme.text_faint)
                                .child(note),
                        )
                    }),
            )
            .child(cell)
    }
}
