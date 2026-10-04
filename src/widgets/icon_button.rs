//! The square icon button: window controls, row actions, the gear.
//!
//! The one rule that matters here is the hover tint. An `Svg` does not inherit `text_color` and is
//! skipped outright when it has none, so the tint has to reach the `Svg` itself — which means the
//! icon is rebuilt inside the hover animation rather than recoloured. That is also why this widget
//! cannot be "just a div with an icon in it": the copy that forgets is an invisible button.

use gpui::prelude::*;
use gpui::{
    AnimationExt, App, ClickEvent, ColorExt, ElementId, RenderOnce, Rgba, SharedString, Window,
    WindowControlArea, div, px,
};

use super::{Sizing, blend, icon, lift, toggle, track_hover, watch_hover};
use crate::theme::Theme;
use crate::tokens::{RADIUS, motion};

pub type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// What the button's icon is saying. Not a colour — a meaning, so that "the delete one" has one
/// definition and the danger colour has one caller.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tone {
    /// Ordinary: muted at rest, full contrast under the cursor.
    #[default]
    Neutral,
    /// The destructive one — a row's delete, and nothing else.
    Danger,
}

pub struct IconButton {
    id: ElementId,
    glyph: SharedString,
    sizing: Sizing,
    tone: Tone,
    /// Washed in the accent while its toggle is on: a gear that means "settings are open".
    active: bool,
    /// Draws the hairline that marks a control standing on its own rather than inside a row.
    outlined: bool,
    /// When set, the platform — not a click handler — owns the hitbox.
    area: Option<WindowControlArea>,
    handler: Option<ClickHandler>,
}

impl IconButton {
    /// `id` identifies the button for the element state tree and for hover. Two buttons may not
    /// share one, which is why row actions pass something with the row's `seq` in it.
    pub fn new(id: impl Into<ElementId>, glyph: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            glyph: glyph.into(),
            sizing: Sizing::Md,
            tone: Tone::Neutral,
            active: false,
            outlined: false,
            area: None,
            handler: None,
        }
    }

    pub fn large(mut self) -> Self {
        self.sizing = Sizing::Lg;
        self
    }

    /// This is a delete-ish button. There is no `tint(...)`: a caller choosing a colour is a caller
    /// inventing a meaning the language does not have.
    pub fn danger(mut self) -> Self {
        self.tone = Tone::Danger;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn outlined(mut self) -> Self {
        self.outlined = true;
        self
    }

    /// Hand the hitbox to the platform. Window controls take this path and must not also have a
    /// click handler: two owners of one click is one of them never firing.
    pub fn window(mut self, area: WindowControlArea) -> Self {
        self.area = Some(area);
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

/// The colours a tone moves between.
struct Palette {
    fg: Rgba,
    fg_hover: Rgba,
    bg: Rgba,
    bg_hover: Rgba,
    border: Rgba,
}

impl Palette {
    fn of(tone: Tone, active: bool, theme: &Theme) -> Palette {
        let dark = theme.is_dark;
        if active {
            return Palette {
                fg: theme.accent,
                fg_hover: lift(theme.accent, dark, 0.1),
                bg: theme.accent_wash,
                bg_hover: theme.surface_hover,
                border: theme.accent.opacity(0.4),
            };
        }
        match tone {
            Tone::Neutral => Palette {
                fg: theme.text_muted,
                fg_hover: theme.text,
                bg: transparent(),
                bg_hover: theme.surface_hover,
                border: theme.border,
            },
            Tone::Danger => Palette {
                fg: theme.danger,
                fg_hover: theme.danger,
                bg: transparent(),
                bg_hover: theme.danger_wash,
                border: theme.border,
            },
        }
    }
}

fn transparent() -> Rgba {
    crate::theme::transparent()
}

impl RenderOnce for IconButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let Self {
            id,
            glyph,
            sizing,
            tone,
            active,
            outlined,
            area,
            handler,
        } = self;

        let palette = Palette::of(tone, active, &theme);
        let (hover, state) = track_hover(&id, window, cx);
        let size = sizing.height();
        let glyph_size = sizing.glyph();

        let element = div()
            .id(id.clone())
            .flex()
            .items_center()
            .justify_center()
            .size(px(size))
            .flex_none()
            .rounded(px(RADIUS))
            .border_1()
            .border_color(if outlined {
                palette.border
            } else {
                transparent()
            })
            .bg(palette.bg)
            .cursor_pointer()
            .on_hover(watch_hover(hover))
            .when_some(area, |element, area| element.window_control_area(area))
            .when_some(handler, |element, handler| {
                element.on_click(move |event, window, cx| handler(event, window, cx))
            });

        if !state.ever {
            return element
                .child(icon(glyph, glyph_size, palette.fg))
                .into_any_element();
        }

        element
            .with_animation(
                (
                    id,
                    if state.hovered {
                        "hover-in"
                    } else {
                        "hover-out"
                    },
                ),
                toggle(motion::FAST),
                move |element, delta| {
                    let t = if state.hovered { delta } else { 1.0 - delta };
                    element
                        .bg(blend(palette.bg, palette.bg_hover, t))
                        .child(icon(
                            glyph.clone(),
                            glyph_size,
                            blend(palette.fg, palette.fg_hover, t),
                        ))
                },
            )
            .into_any_element()
    }
}
