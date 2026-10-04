//! The one card that floats: settings, new download, a confirmation.
//!
//! Every popup in the app is this skeleton — a scrim that swallows clicks without closing, a card
//! of the shared width, a title, a scrollable body, and a right-aligned action row above a
//! hairline. Views supply content and nothing else. That is what keeps three dialogs from drifting
//! into three widths, three paddings and three button rows, which is exactly what happened before
//! this existed.
//!
//! ```ignore
//! Modal::new("add-card", strings.add_title)
//!     .scrolling()
//!     .block(uri_field)
//!     .block(save_row)
//!     .action(cancel)
//!     .action(start)
//! ```
//!
//! The scrim has no listener on purpose: clicking beside a card only takes focus off a box, it does
//! not throw the card away. Closing is always an explicit action in the footer, and the focus
//! returns to whatever opened the card — see `STYLE.md` §10.

use gpui::prelude::*;
use gpui::{
    AnimationExt, AnyElement, App, BoxShadow, ElementId, FontWeight, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, px,
};

use super::{part_id, settle};
use crate::theme::{Look, Theme};
use crate::tokens::{FADE, RADIUS, layout, motion, space, text};

/// How wide a table-shaped card actually gets: fixed, but shrunk to fit a narrow window.
pub fn modal_width(window: &Window) -> f32 {
    (window.viewport_size().width.as_f32() - layout::MODAL_W_SLACK)
        .clamp(layout::MODAL_W_MIN, layout::MODAL_W)
}

/// How tall a card's scrolling body may get: the window minus the card's chrome (title, paddings,
/// action row) and a margin top and bottom, so a card always floats instead of filling the screen.
/// The floor keeps the body usable on a very short window.
pub fn modal_body_max(window: &Window) -> f32 {
    (window.viewport_size().height.as_f32() - layout::MODAL_MAX_H_SLACK)
        .clamp(layout::MODAL_MAX_H_MIN, layout::MODAL_MAX_H_MAX)
}

#[derive(IntoElement)]
pub struct Modal {
    id: ElementId,
    title: SharedString,
    /// `None` uses [`modal_width`]; `Some` overrides it — only the confirmation box, which asks one
    /// question and looks silly stretched to a table's width.
    width: Option<f32>,
    /// Whether the body scrolls inside [`modal_body_max`] and carries the fade that says so.
    scrolling: bool,
    blocks: Vec<AnyElement>,
    actions: Vec<AnyElement>,
}

impl Modal {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            width: None,
            scrolling: false,
            blocks: Vec::new(),
            actions: Vec::new(),
        }
    }

    /// A card narrower than the table width.
    pub fn narrow(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Let the body scroll. The fade comes with it because GPUI draws no scrollbar, so the fade is
    /// the only cue that there is more below.
    pub fn scrolling(mut self) -> Self {
        self.scrolling = true;
        self
    }

    /// One block of the body. The spacing between blocks is the skeleton's business, so a caller
    /// never sets a gap of its own.
    pub fn block(mut self, block: impl IntoElement) -> Self {
        self.blocks.push(block.into_any_element());
        self
    }

    /// One button. Secondary actions first, the main action last.
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }
}

impl RenderOnce for Modal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let font = Look::of(cx).font(window);
        let Self {
            id,
            title,
            width,
            scrolling,
            blocks,
            actions,
        } = self;

        let width = width.unwrap_or_else(|| modal_width(window));
        let scrim = theme.scrim;
        let surface = theme.surface;
        let shadow = theme.modal_shadow();

        let blocks = div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(layout::MODAL_GAP))
            .children(blocks);

        let body = if scrolling {
            div()
                .relative()
                .flex()
                .flex_col()
                .flex_none()
                .child(
                    div()
                        .id(part_id(&id, "body"))
                        .flex()
                        .flex_col()
                        .flex_none()
                        .max_h(px(modal_body_max(window)))
                        // The padding is what the fade covers when the body is already at the end,
                        // so real content is never the thing that gets faded.
                        .pb(px(FADE))
                        .overflow_y_scroll()
                        .child(blocks),
                )
                .child(scroll_fade(surface))
                .into_any_element()
        } else {
            blocks.into_any_element()
        };

        div()
            .occlude()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(scrim)
            .child(
                div()
                    .id(id.clone())
                    .flex()
                    .flex_col()
                    .flex_none()
                    .gap(px(layout::MODAL_GAP))
                    .w(px(width))
                    .px(px(layout::MODAL_PAD))
                    .py(px(layout::MODAL_PAD))
                    .rounded(px(RADIUS))
                    .bg(surface)
                    .border_1()
                    .border_color(theme.border)
                    .shadow(vec![
                        BoxShadow::new(px(0.0), px(8.0), shadow).blur_radius(px(32.0)),
                    ])
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .font(font)
                    .text_size(px(text::BODY))
                    .with_animation(
                        (id, "enter"),
                        settle(motion::SLOW),
                        move |element, delta| {
                            // `Styled::opacity` multiplies down the whole subtree, so the card
                            // fades its title, its rows and its buttons in one go — and it lifts
                            // the last few pixels into place while it does.
                            element.opacity(delta).mt(px(motion::SHIFT * (1.0 - delta)))
                        },
                    )
                    .child(
                        div()
                            .text_size(px(text::TITLE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child(title),
                    )
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .flex_none()
                            .items_center()
                            .justify_end()
                            .gap(px(space::SM))
                            .border_t_1()
                            .border_color(theme.border_soft)
                            .pt(px(space::MD))
                            .children(actions),
                    ),
            )
    }
}

/// The gradient that frosts the bottom edge of a scrolling body, so the next row fades out instead
/// of being sliced in half.
///
/// GPUI never draws a scrollbar, so this is the only cue that there is more to see. Pair it with a
/// `pb(px(FADE))` on the scroller — that padding is what the gradient covers once the body is at
/// the end, which is what keeps real content from being the thing that fades.
///
/// The overlay carries no listener, cursor or hover group, which is what keeps it hitbox-free: it
/// cannot swallow the wheel events that belong to the scroller underneath.
pub fn scroll_fade(bg: gpui::Rgba) -> impl IntoElement {
    use gpui::{ColorExt, linear_color_stop, linear_gradient};
    div()
        .absolute()
        .bottom_0()
        .left_0()
        .w_full()
        .h(px(FADE))
        .bg(linear_gradient(
            180.0,
            linear_color_stop(bg.opacity(0.0), 0.0),
            linear_color_stop(bg, 1.0),
        ))
}
