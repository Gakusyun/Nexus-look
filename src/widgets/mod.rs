//! The widgets. This is the only implementation of the language — a call site supplies data and
//! behaviour, and everything visual comes from here.
//!
//! Three mechanical facts about GPUI-CE shape how these are written, and all three are why the
//! widgets exist rather than the call sites doing it themselves:
//!
//! * **An element's style is self-contained.** `compute_style_internal` starts from
//!   `Style::default()`, so nothing inherits: not `text_color`, not `font`, not hover state. A
//!   widget that forgets to set the font draws in the platform default while its neighbours draw
//!   in the user's chosen family.
//! * **A `Svg` with no colour of its own is skipped entirely** — not drawn in black, skipped. So
//!   every icon takes its tint as an argument (see [`icon`]).
//! * **Hover styles in this version are not transitioned.** `hover(..)` swaps a style refinement
//!   instantly. Animated hover therefore needs the hover *state* to be readable during render,
//!   which is what [`track_hover`] is for: it parks a tiny entity in the element state tree keyed
//!   by the widget's own id, so a stateless `RenderOnce` widget can still animate.

mod button;
mod icon_button;
pub mod text_edit;
mod text_input;

pub use button::{Button, Variant};
pub use icon_button::IconButton;
pub use text_edit::TextEdit;
pub use text_input::TextInput;

use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    Animation, App, Div, ElementId, Entity, FontWeight, Rgba, SharedString, Svg, Window, div,
    ease_in_out, px, rgb, svg,
};

use crate::theme::{Look, Theme, Tone};
use crate::tokens::{ICON_SM, RADIUS, space, text};

/// How big a control is. Two sizes, and the second one has to be earned (see `STYLE.md`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Sizing {
    /// `CONTROL`: every control in a row.
    #[default]
    Md,
    /// `CONTROL_LG`: the command bar's input and the page's primary call to action.
    Lg,
}

impl Sizing {
    pub fn height(self) -> f32 {
        match self {
            Sizing::Md => crate::tokens::CONTROL,
            Sizing::Lg => crate::tokens::CONTROL_LG,
        }
    }

    /// Horizontal padding. Wider text needs more room, not more height.
    pub fn pad(self) -> f32 {
        match self {
            Sizing::Md => space::MD,
            Sizing::Lg => space::LG,
        }
    }

    /// The icon size that reads as part of the label rather than as a second element.
    pub fn glyph(self) -> f32 {
        match self {
            Sizing::Md => ICON_SM,
            Sizing::Lg => crate::tokens::ICON_MD,
        }
    }
}

/// An icon, tinted explicitly.
///
/// The tint is a parameter rather than something the parent sets because a `Svg` **does not
/// inherit `text_color`**: GPUI builds the style it paints with from `Style::default()` plus that
/// element's own refinements, never from the surrounding `Div`. And `Svg::paint` only draws when
/// its own `style.text.color` is `Some`, so an icon whose parent carried the colour is silently
/// skipped — no warning, no error, just an invisible button.
pub fn icon(path: impl Into<SharedString>, size: f32, tint: Rgba) -> Svg {
    svg().path(path).size(px(size)).flex_none().text_color(tint)
}

/// True when the platform is painting dark. Widgets that need to *mix* a colour (hover on a filled
/// button goes toward white in the dark palette and toward black in the light one) ask here rather
/// than guessing from the accent.
pub(crate) fn lift(color: Rgba, dark: bool, amount: f32) -> Rgba {
    let toward = if dark { rgb(0xffffff) } else { rgb(0x000000) };
    blend(color, toward, amount)
}

/// Straight sRGB interpolation. Not colour-correct, and deliberately so: a 100ms hover fade has no
/// visible mid-tone, and a correct-space conversion per frame would be work for nobody.
pub(crate) fn blend(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let (fr, fg, fb) = crate::theme::channels(from);
    let (tr, tg, tb) = crate::theme::channels(to);
    crate::theme::srgb(
        fr + (tr - fr) * t,
        fg + (tg - fg) * t,
        fb + (tb - fb) * t,
        from.alpha + (to.alpha - from.alpha) * t,
    )
}

/// The easing for anything that toggles back and forth: hover, press, selection.
pub(crate) fn toggle(duration: Duration) -> Animation {
    Animation::new(duration).with_easing(ease_in_out)
}

/// Whether a control is under the cursor, kept for as long as the control keeps its id.
///
/// `ever` exists because a hover animation is a transition keyed on which state is current, so the
/// frame the animation starts at always draws the state it is coming *from*. On mount that would
/// mean every button flashing its hover colour for one frame. Until the pointer has arrived once
/// there is nothing to transition from, so the widget paints the resting state directly and never
/// starts an animation at all.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Hover {
    pub hovered: bool,
    pub ever: bool,
}

/// Park a hover flag for `id` and read its current value.
///
/// A widget cannot store hover in a field: `RenderOnce` values are rebuilt from scratch every
/// frame, so a flag set by `on_hover` would be gone before the next render could read it. GPUI's
/// element state tree is the only place that outlives a frame without the widget owning an entity,
/// and `use_keyed_state` even re-notifies the owning view when the flag changes — which is what
/// makes the following frame start the animation.
pub(crate) fn track_hover(
    id: &ElementId,
    window: &mut Window,
    cx: &mut App,
) -> (Entity<Hover>, Hover) {
    let state = window.use_keyed_state(id.clone(), cx, |_, _| Hover::default());
    let current = *state.read(cx);
    (state, current)
}

/// The listener half of [`track_hover`], ready to hand to `on_hover`.
pub(crate) fn watch_hover(state: Entity<Hover>) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
    move |hovered: &bool, _window, cx| {
        state.update(cx, |state, cx| {
            let next = Hover {
                hovered: *hovered,
                ever: state.ever || *hovered,
            };
            if *state != next {
                *state = next;
                cx.notify();
            }
        });
    }
}

/// A raised surface: cards, the command bar, the detail panel, a settings group.
///
/// A function rather than a builder because there is nothing to decide — the moment a caller needs
/// a different padding they are describing a different thing, and that thing should get a name.
pub fn card(theme: &Theme) -> Div {
    div()
        .rounded(px(RADIUS))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .p(px(space::LG))
}

/// A hairline between two groups of things.
pub fn divider(theme: &Theme) -> Div {
    div().h(px(1.0)).w_full().bg(theme.border_soft)
}

/// The one line of explanatory text under a control.
///
/// There is no second hint widget and no `error_note`: an error is a hint whose tone is
/// [`Tone::Danger`], which is exactly the rule for semantic colours — the pixel is describing a
/// state, so it is allowed to have a colour.
pub fn hint(
    text_str: impl Into<SharedString>,
    tone: Tone,
    theme: &Theme,
    window: &Window,
    cx: &App,
) -> Div {
    div()
        .text_size(px(text::CAPTION))
        .text_color(theme.tone(tone))
        .font(Look::of(cx).font(window))
        .child(text_str.into())
}

/// A section heading inside a card or a modal body.
pub fn heading(label: impl Into<SharedString>, theme: &Theme, window: &Window, cx: &App) -> Div {
    div()
        .text_size(px(text::HEADING))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.text)
        .font(Look::of(cx).font(window))
        .child(label.into())
}
