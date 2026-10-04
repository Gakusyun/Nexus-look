//! The one rect text button.
//!
//! Four variants, and each one means something specific rather than looking a particular way:
//! `Primary` is *the* confirm on a screen, `Secondary` is every other action, `Ghost` is an action
//! that should not compete for attention, and `Danger` is the only way to ask for something
//! irreversible. There is deliberately no fifth: a call site that wants "a red outlined secondary"
//! is asking for a control the language does not have, and the answer is to use `Danger` or to
//! rethink the screen.

use gpui::prelude::*;
use gpui::{
    AnimationExt, App, ClickEvent, ElementId, FontWeight, ParentElement, RenderOnce, Rgba,
    SharedString, Styled, Window, div, px,
};

use super::{Sizing, blend, icon, lift, toggle, track_hover, watch_hover};
use crate::theme::{Look, Theme};
use crate::tokens::{RADIUS, motion, space, text};

pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// What a button is *for*. See the module docs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    /// Filled with the accent. One per screen.
    Primary,
    /// A raised surface with a hairline. Everything else.
    Secondary,
    /// Bare text. Does not draw attention, and must not be the only way to do something.
    Ghost,
    /// Filled with `danger`. Only for actions that cannot be undone.
    Danger,
}

/// The colours a variant moves between. One row per variant, so "what does Primary look like
/// pressed" has exactly one answer.
struct Palette {
    rest: Rgba,
    hover: Rgba,
    pressed: Rgba,
    border: Rgba,
    fg: Rgba,
    fg_hover: Rgba,
}

impl Palette {
    fn of(variant: Variant, theme: &Theme) -> Palette {
        let dark = theme.is_dark;
        match variant {
            Variant::Primary => Palette {
                rest: theme.accent,
                hover: lift(theme.accent, dark, 0.08),
                pressed: lift(theme.accent, dark, 0.16),
                border: transparent(),
                fg: theme.on_accent,
                fg_hover: theme.on_accent,
            },
            Variant::Secondary => Palette {
                rest: theme.surface,
                hover: theme.surface_hover,
                pressed: theme.surface_pressed,
                border: theme.border,
                fg: theme.text,
                fg_hover: theme.text,
            },
            Variant::Ghost => Palette {
                rest: transparent(),
                hover: theme.surface_hover,
                pressed: theme.surface_pressed,
                border: transparent(),
                fg: theme.text_muted,
                fg_hover: theme.text,
            },
            Variant::Danger => Palette {
                rest: theme.danger,
                hover: lift(theme.danger, dark, 0.08),
                pressed: lift(theme.danger, dark, 0.16),
                border: transparent(),
                fg: theme.on_danger,
                fg_hover: theme.on_danger,
            },
        }
    }
}

fn transparent() -> Rgba {
    crate::theme::transparent()
}

/// A button with a label, and optionally a leading icon.
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: Variant,
    sizing: Sizing,
    glyph: Option<SharedString>,
    disabled: bool,
    handler: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, variant: Variant) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant,
            sizing: Sizing::Md,
            glyph: None,
            disabled: false,
            handler: None,
        }
    }

    pub fn primary(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::new(id, label, Variant::Primary)
    }

    pub fn secondary(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::new(id, label, Variant::Secondary)
    }

    pub fn ghost(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::new(id, label, Variant::Ghost)
    }

    /// The destructive one. `Danger` is the only variant whose name is a warning, and the only
    /// variant that may be used for something the user cannot take back.
    pub fn danger(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::new(id, label, Variant::Danger)
    }

    /// The large size. Two places earn it; see `STYLE.md`.
    pub fn large(mut self) -> Self {
        self.sizing = Sizing::Lg;
        self
    }

    pub fn icon(mut self, glyph: impl Into<SharedString>) -> Self {
        self.glyph = Some(glyph.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.handler = Some(Box::new(handler));
        self
    }
}

/// The label and the icon, painted in `fg`.
///
/// Split out because it is drawn twice: once for the resting state (no animation has ever run) and
/// once per frame while the hover fades. Building it in one place is what keeps those two from
/// disagreeing about the gap or the icon size.
fn content<E>(
    element: E,
    glyph: Option<SharedString>,
    label: SharedString,
    glyph_size: f32,
    fg: Rgba,
) -> E
where
    E: Styled + ParentElement,
{
    let element = element.text_color(fg);
    match glyph {
        Some(path) => element.child(icon(path, glyph_size, fg)).child(label),
        None => element.child(label),
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let font = Look::of(cx).font(window);
        let Self {
            id,
            label,
            variant,
            sizing,
            glyph,
            disabled,
            handler,
        } = self;

        let palette = Palette::of(variant, &theme);
        let fg = if disabled {
            theme.text_disabled
        } else {
            palette.fg
        };
        let (hover, state) = track_hover(&id, window, cx);
        // A disabled control does not react to the cursor at all — not a dimmer hover, not a
        // different cursor, nothing. Anything else reads as "it noticed, it just will not".
        let interacting = !disabled;
        let hovered = interacting && state.hovered;

        let element = div()
            .id(id.clone())
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(space::SM))
            .h(px(sizing.height()))
            .px(px(sizing.pad()))
            .flex_none()
            .rounded(px(RADIUS))
            .border_1()
            .border_color(palette.border)
            .bg(palette.rest)
            .font(font)
            .text_size(px(text::BODY))
            .font_weight(FontWeight::MEDIUM)
            .when(disabled, |element| element.cursor_default())
            .when(interacting, |element| {
                element
                    .cursor_pointer()
                    .on_hover(watch_hover(hover))
                    // Press is instant on purpose: a press that fades in reads as lag, and the
                    // animation budget is better spent on the state the user is *arriving at*.
                    .active(move |style| style.bg(palette.pressed))
            })
            .when_some(handler, |element, handler| {
                element.on_click(move |event, window, cx| handler(event, window, cx))
            });

        let glyph_size = sizing.glyph();
        if !interacting || !state.ever {
            return content(element, glyph, label, glyph_size, fg).into_any_element();
        }

        element
            .with_animation(
                (id, if hovered { "hover-in" } else { "hover-out" }),
                toggle(motion::FAST),
                move |element, delta| {
                    let t = if hovered { delta } else { 1.0 - delta };
                    let fg = blend(palette.fg, palette.fg_hover, t);
                    content(
                        element.bg(blend(palette.rest, palette.hover, t)),
                        glyph.clone(),
                        label.clone(),
                        glyph_size,
                        fg,
                    )
                },
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Look;
    use crate::tokens::CONTROL;

    #[test]
    fn the_two_sizes_are_the_two_heights() {
        assert_eq!(Sizing::Md.height(), CONTROL);
        assert!(Sizing::Lg.height() > CONTROL);
    }

    #[test]
    fn a_filled_button_keeps_its_own_text_colour() {
        let theme = Theme::resolve(&Look::new(), true);
        let palette = Palette::of(Variant::Primary, &theme);
        assert_eq!(palette.fg, theme.on_accent);
        assert_eq!(palette.fg, palette.fg_hover);
    }

    #[test]
    fn a_ghost_button_gains_contrast_on_hover() {
        let theme = Theme::resolve(&Look::new(), true);
        let palette = Palette::of(Variant::Ghost, &theme);
        assert_eq!(palette.fg, theme.text_muted);
        assert_eq!(palette.fg_hover, theme.text);
    }

    #[test]
    fn only_danger_reaches_for_the_danger_colour() {
        let theme = Theme::resolve(&Look::new(), true);
        for variant in [Variant::Primary, Variant::Secondary, Variant::Ghost] {
            let palette = Palette::of(variant, &theme);
            assert_ne!(palette.rest, theme.danger);
            assert_ne!(palette.hover, theme.danger);
        }
        assert_eq!(Palette::of(Variant::Danger, &theme).rest, theme.danger);
    }
}
