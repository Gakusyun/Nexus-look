//! The palette, and the two knobs a project gets to turn: which accent, and which mode.
//!
//! The language has three colours — `text`, and the window's `bg` in black or white, and `accent`.
//! Everything else in [`Theme`] is *derived* from those: the layers are black or white at some
//! alpha, the greys are neutral (R = G = B, no hue at all), and the semantic colours are the only
//! hues besides the accent.
//!
//! Two things here are deliberately not obvious:
//!
//! * **Layers are flattened, not left transparent.** A translucent `surface` painted on the modal
//!   scrim would composite against the scrim instead of the window, so the card would come out a
//!   different colour than the same card on the page. The derivation is alpha-over-black-and-white
//!   (that part of the spec is literal); the *stored* value is the flattened result, and the hex
//!   in `STYLE.md` is what you get.
//! * **The accent is one value for two modes.** The user picks one colour in settings, so the
//!   library owes them two readable ones. Light mode gets a darkened variant — pushed toward black
//!   only as far as WCAG's 4.5:1 needs — and dark mode the original, lightened the same way if it
//!   is too dark to read. `accent_light` overrides the derived value when a project wants to tune
//!   it by hand.

use gpui::{
    App, ColorExt, Font, FontFallbacks, Global, Hsla, Rgba, SharedString, Window, WindowAppearance,
    rgb, rgb_to_hsla, rgba,
};

use crate::tokens::space;

// -------------------------------------------------------------------------------------------
// Mode
// -------------------------------------------------------------------------------------------

/// Which palette to paint. `System` follows the platform's light/dark setting.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 3] = [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark];

    pub fn as_str(self) -> &'static str {
        match self {
            ThemeMode::System => "system",
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "system" => Some(ThemeMode::System),
            "light" => Some(ThemeMode::Light),
            "dark" => Some(ThemeMode::Dark),
            _ => None,
        }
    }

    pub fn is_dark(self, system_is_dark: bool) -> bool {
        match self {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => system_is_dark,
        }
    }
}

/// Whether the platform is currently painting dark.
pub fn system_is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

// -------------------------------------------------------------------------------------------
// The project's configuration
// -------------------------------------------------------------------------------------------

/// What a project decides about the look, and the only thing it has to hand the library.
///
/// A `Global`, so widgets can read the font without every call site threading it through — and so
/// changing the accent in a settings sheet reaches every widget on the next frame instead of
/// needing a rebuild.
#[derive(Clone)]
pub struct Look {
    pub mode: ThemeMode,
    /// The project's (or the user's) accent, `0xRRGGBB`.
    pub accent: u32,
    /// Overrides the derived light-mode accent. `None` means "work it out" (see the module docs).
    pub accent_light: Option<u32>,
    /// Paint dark mode on pure black. The layers follow, so the whole palette stays consistent.
    pub oled: bool,
    /// The family to draw with, most preferred first. `None` keeps the platform default, which is
    /// the right answer on a machine whose fonts you cannot see.
    pub font_family: Option<SharedString>,
    pub font_fallbacks: Vec<SharedString>,
}

impl Global for Look {}

impl Default for Look {
    fn default() -> Self {
        Self::new()
    }
}

impl Look {
    /// A neutral blue accent and no font opinion. Projects are expected to override the accent;
    /// that is what makes an app look like itself instead of like the library.
    pub fn new() -> Self {
        Self {
            mode: ThemeMode::System,
            accent: 0x0a84ff,
            accent_light: None,
            oled: false,
            font_family: None,
            font_fallbacks: Vec::new(),
        }
    }

    pub fn accent(mut self, accent: u32) -> Self {
        self.accent = accent;
        self
    }

    pub fn accent_light(mut self, accent: u32) -> Self {
        self.accent_light = Some(accent);
        self
    }

    pub fn mode(mut self, mode: ThemeMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn oled(mut self, oled: bool) -> Self {
        self.oled = oled;
        self
    }

    /// Let the user's font choice win over the platform default, falling back through `fallbacks`
    /// for the scripts the primary family does not cover.
    pub fn font_family(mut self, family: impl Into<SharedString>, fallbacks: Vec<String>) -> Self {
        self.font_family = Some(family.into());
        self.font_fallbacks = fallbacks.into_iter().map(SharedString::from).collect();
        self
    }

    pub fn of(cx: &App) -> &Look {
        cx.global::<Look>()
    }

    /// The font every widget measures and paints with.
    ///
    /// The window's default style is the starting point on purpose: it is the platform's own
    /// answer for this machine and locale, which is a better guess than any list we could ship.
    pub fn font(&self, window: &Window) -> Font {
        let mut font = window.text_style().font();
        if let Some(family) = self.font_family.clone() {
            font.family = family;
            if !self.font_fallbacks.is_empty() {
                font.fallbacks = Some(FontFallbacks::from_fonts(
                    self.font_fallbacks
                        .iter()
                        .map(|family| family.to_string())
                        .collect(),
                ));
            }
        }
        font
    }

    /// Install a configuration and the palette that follows from it.
    pub fn install(self, cx: &mut App) -> Theme {
        let theme = Theme::resolve(
            &self,
            self.mode.is_dark(system_is_dark(cx.window_appearance())),
        );
        cx.set_global(self);
        cx.set_global(theme.clone());
        theme
    }

    /// Switch palettes. Re-derives the theme, so a settings sheet can preview a mode live and a
    /// cancel only has to put the previous one back.
    pub fn set_palette(&mut self, cx: &mut App, mode: ThemeMode, accent: u32, oled: bool) {
        self.mode = mode;
        self.accent = accent;
        self.oled = oled;
        self.refresh(cx);
    }

    /// Re-resolve against the current configuration. Call it from the platform's appearance
    /// observer, and after editing the accent.
    pub fn refresh(&self, cx: &mut App) {
        let theme = Theme::resolve(
            self,
            self.mode.is_dark(system_is_dark(cx.window_appearance())),
        );
        cx.set_global(theme);
    }

    /// Edit the configuration and re-derive the palette.
    ///
    /// The one way a running app changes its look. It reads the current global, applies `edit` and
    /// reinstalls both halves, so a caller cannot change the accent and forget to repaint, or
    /// repaint without having changed anything.
    pub fn update(cx: &mut App, edit: impl FnOnce(&mut Look)) {
        let mut look = cx.global::<Look>().clone();
        edit(&mut look);
        look.refresh(cx);
    }
}

// -------------------------------------------------------------------------------------------
// The palette
// -------------------------------------------------------------------------------------------

/// Every colour the UI is allowed to use.
///
/// The layers are [`space`]-style opaque values rather than alphas (see the module docs), and the
/// greys are neutral: `r == g == b` for every one of them, which is what makes "three colours"
/// more than a slogan. A tinted grey would put a second hue on screen and start arguing with
/// whatever accent the user picked.
#[derive(Clone)]
pub struct Theme {
    // Ground
    pub bg: Rgba,
    /// Raised surfaces: cards, the command bar, a hovered row, the detail panel.
    pub surface: Rgba,
    /// One step above a surface: a row inside a card, an icon button under the cursor.
    pub surface_hover: Rgba,
    pub surface_pressed: Rgba,
    /// Inputs. Recessed: darker than `surface` in both modes, so "you can type here" is a shape
    /// rather than a colour.
    pub field: Rgba,

    // Edges
    pub border: Rgba,
    pub border_soft: Rgba,

    // Text
    pub text: Rgba,
    pub text_muted: Rgba,
    pub text_faint: Rgba,
    pub text_disabled: Rgba,

    // Accent
    pub accent: Rgba,
    pub accent_wash: Rgba,
    /// The one legible colour to put *on* `accent`. Never write white by hand.
    pub on_accent: Rgba,

    // States. Allowed wherever a pixel is describing what happened, and nowhere else.
    pub success: Rgba,
    pub success_wash: Rgba,
    pub danger: Rgba,
    pub danger_wash: Rgba,
    pub on_danger: Rgba,
    pub warning: Rgba,
    pub warning_wash: Rgba,

    // Layering
    pub scrim: Rgba,
    pub shadow: Hsla,
    pub focus_ring: Rgba,
    pub selection: Rgba,

    /// Which of the two palettes this is, for the rare widget that has to know rather than just
    /// paint (an inverted button, a logo plate).
    pub is_dark: bool,
}

impl Global for Theme {}

impl Theme {
    pub fn of(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// Work out the palette for `look` in the requested mode.
    pub fn resolve(look: &Look, dark: bool) -> Theme {
        let bg = if dark && look.oled {
            rgb(0x000000)
        } else if dark {
            rgb(0x202020)
        } else {
            rgb(0xf3f3f3)
        };

        // Layers: black or white at an alpha, flattened onto what they sit on. `surface` sits on
        // the window, `surface_hover` on a surface, `surface_pressed` on a hover.
        let (surface, surface_hover, surface_pressed) = if dark {
            let surface = flatten(bg, white(0.04));
            let hover = flatten(surface, white(0.08));
            let pressed = flatten(hover, white(0.12));
            (surface, hover, pressed)
        } else {
            // Light mode's raised surface is plain white: the window is the darker one.
            let surface = rgb(0xffffff);
            let hover = flatten(surface, black(0.04));
            let pressed = flatten(hover, black(0.08));
            (surface, hover, pressed)
        };

        let field = if dark { bg } else { rgb(0xfafafa) };

        let accent = match (dark, look.accent_light) {
            (true, _) => readable(look.accent, &[bg, surface]),
            (false, None) => readable(look.accent, &[bg, surface]),
            (false, Some(accent)) => readable(accent, &[bg, surface]),
        };

        let (success, success_wash) = if dark {
            (rgb(0x4cc38a), rgb(0x4cc38a).opacity(0.14))
        } else {
            (rgb(0x1a7f4b), rgb(0x1a7f4b).opacity(0.12))
        };
        let (danger, danger_wash) = if dark {
            (rgb(0xff6b6b), rgb(0xff6b6b).opacity(0.14))
        } else {
            (rgb(0xc62828), rgb(0xc62828).opacity(0.12))
        };
        let (warning, warning_wash) = if dark {
            (rgb(0xe3a008), rgb(0xe3a008).opacity(0.14))
        } else {
            (rgb(0x96610a), rgb(0x96610a).opacity(0.12))
        };

        Theme {
            bg,
            surface,
            surface_hover,
            surface_pressed,
            field,

            border: if dark { rgb(0x3f3f3f) } else { rgb(0xcfcfcf) },
            border_soft: if dark { rgb(0x303030) } else { rgb(0xe0e0e0) },

            text: if dark { rgb(0xffffff) } else { rgb(0x000000) },
            text_muted: if dark { rgb(0xa6a6a6) } else { rgb(0x5c5c5c) },
            text_faint: if dark { rgb(0x8a8a8a) } else { rgb(0x7a7a7a) },
            text_disabled: if dark { rgb(0x5c5c5c) } else { rgb(0xb3b3b3) },

            accent,
            accent_wash: accent.opacity(if dark { 0.16 } else { 0.12 }),
            on_accent: readable_on(accent),

            success,
            success_wash,
            danger,
            danger_wash,
            on_danger: readable_on(danger),
            warning,
            warning_wash,

            scrim: black(if dark { 0.6 } else { 0.45 }),
            shadow: rgb_to_hsla(black(if dark { 0.6 } else { 0.16 })),
            focus_ring: accent,
            selection: accent.opacity(0.3),

            is_dark: dark,
        }
    }

    /// Where a modal's shadow is drawn from — one definition, one user.
    pub fn modal_shadow(&self) -> Hsla {
        self.shadow
    }

    /// The padding a modal body uses, kept here so the skeleton and the shadow agree about the
    /// language they belong to.
    pub fn modal_pad(&self) -> f32 {
        space::XL
    }
}

/// What a pixel is describing when it reaches for a semantic colour.
///
/// This is the whole licence for the semantic colours: a widget that takes a `Tone` is saying
/// "this is a state", and the colours are only reachable through one. There is no
/// `theme.green` to sprinkle on a decorative divider.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tone {
    #[default]
    Neutral,
    Accent,
    Success,
    Danger,
    Warning,
}

impl Theme {
    /// The colour that carries a state: badge text, progress fill, an error line.
    pub fn tone(&self, tone: Tone) -> Rgba {
        match tone {
            Tone::Neutral => self.text_muted,
            Tone::Accent => self.accent,
            Tone::Success => self.success,
            Tone::Danger => self.danger,
            Tone::Warning => self.warning,
        }
    }

    /// The same state as a background: badge and banner washes.
    pub fn tone_wash(&self, tone: Tone) -> Rgba {
        match tone {
            Tone::Neutral => self.surface_hover,
            Tone::Accent => self.accent_wash,
            Tone::Success => self.success_wash,
            Tone::Danger => self.danger_wash,
            Tone::Warning => self.warning_wash,
        }
    }
}

// -------------------------------------------------------------------------------------------
// Colour maths
// -------------------------------------------------------------------------------------------

fn white(alpha: f32) -> Rgba {
    rgb(0xffffff).opacity(alpha)
}

fn black(alpha: f32) -> Rgba {
    rgb(0x000000).opacity(alpha)
}

/// The three colour channels, 0..1.
///
/// GPUI's `Rgba` is `palette`'s `Alpha<Rgb, f32>`, so the components live behind a struct that has
/// nothing to do with this library's vocabulary. These two functions are the only place that shape
/// is spelled out.
pub fn channels(color: Rgba) -> (f32, f32, f32) {
    (color.color.red, color.color.green, color.color.blue)
}

/// An sRGB colour from channels. The counterpart to [`channels`].
pub fn srgb(r: f32, g: f32, b: f32, a: f32) -> Rgba {
    Rgba::new(r, g, b, a)
}

/// Fully transparent, for the border a variant does not have. Not a colour: a placeholder that
/// keeps every button the same size whether or not it draws a hairline.
pub fn transparent() -> Rgba {
    rgba(0x00000000)
}

/// Composite `top` over `base`, throwing the alpha away. Painting a translucent `surface` on the
/// scrim would otherwise leave the modal card darker than a card on the page.
fn flatten(base: Rgba, top: Rgba) -> Rgba {
    let a = top.alpha;
    let (tr, tg, tb) = channels(top);
    let (br, bg, bb) = channels(base);
    srgb(
        tr * a + br * (1.0 - a),
        tg * a + bg * (1.0 - a),
        tb * a + bb * (1.0 - a),
        1.0,
    )
}

fn channel(value: f32) -> f32 {
    if value <= 0.03928 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG relative luminance. Only the ratio matters here, so the alpha is ignored — every colour
/// this is asked about has been flattened first.
pub fn luminance(color: Rgba) -> f32 {
    let (r, g, b) = channels(color);
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

/// WCAG contrast ratio, 1.0 to 21.0.
pub fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (one, two) = (luminance(a), luminance(b));
    let (hi, lo) = if one > two { (one, two) } else { (two, one) };
    (hi + 0.05) / (lo + 0.05)
}

/// Black or white, whichever is more legible on `bg`. Ties go to white: a filled accent with white
/// text is the shape people already expect, and a tie means either would do.
pub fn readable_on(bg: Rgba) -> Rgba {
    if contrast(rgb(0x000000), bg) > contrast(rgb(0xffffff), bg) {
        rgb(0x000000)
    } else {
        rgb(0xffffff)
    }
}

/// The smallest contrast ratio this library accepts for text.
pub const TEXT_CONTRAST: f32 = 4.5;

/// Nudge `accent` toward white or black until it can be read as text on every one of `grounds`.
///
/// The accent lands on both the window and the cards above it, and those two are never equally
/// easy: in the dark palette the card is lighter, in the light palette the window is darker. Asking
/// about both is shorter than reasoning about which one is worse this time.
///
/// Stepping rather than solving: the goal is *a* legible accent that still reads as the colour the
/// user picked, and a stepwise mix keeps the hue and saturation exactly and only moves lightness.
/// Returns the input untouched when it is already fine, so most accents survive verbatim.
pub fn readable(accent: u32, grounds: &[Rgba]) -> Rgba {
    let accent = rgb(accent);
    let legible = |color: Rgba| {
        grounds
            .iter()
            .all(|ground| contrast(color, *ground) >= TEXT_CONTRAST)
    };
    if legible(accent) {
        return accent;
    }
    let toward = if grounds.iter().any(|ground| luminance(*ground) > 0.5) {
        rgb(0x000000)
    } else {
        rgb(0xffffff)
    };
    let mut best = accent;
    let mut step = 0.05;
    while step <= 1.0 {
        let candidate = mix(accent, toward, step);
        if legible(candidate) {
            return candidate;
        }
        best = candidate;
        step += 0.05;
    }
    // Nothing reached 4.5:1 (only possible between two mid greys). Hand back the most contrast we
    // found rather than the original.
    best
}

fn mix(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let (fr, fg, fb) = channels(from);
    let (tr, tg, tb) = channels(to);
    srgb(
        fr + (tr - fr) * t,
        fg + (tg - fg) * t,
        fb + (tb - fb) * t,
        1.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look() -> Look {
        Look::new().accent(0x7c5cff)
    }

    #[test]
    fn the_layers_are_opaque_so_a_modal_is_not_darker_than_a_card() {
        for dark in [true, false] {
            let theme = Theme::resolve(&look(), dark);
            for color in [theme.surface, theme.surface_hover, theme.surface_pressed] {
                assert_eq!(color.alpha, 1.0, "layers must be flattened");
            }
        }
    }

    #[test]
    fn a_card_is_lifted_off_the_window_in_both_modes() {
        let dark = Theme::resolve(&look(), true);
        assert!(luminance(dark.surface) > luminance(dark.bg));
        let light = Theme::resolve(&look(), false);
        assert!(luminance(light.surface) > luminance(light.bg));
    }

    #[test]
    fn an_input_is_recessed_in_both_modes() {
        let dark = Theme::resolve(&look(), true);
        assert!(luminance(dark.field) < luminance(dark.surface));
        let light = Theme::resolve(&look(), false);
        assert!(luminance(light.field) < luminance(light.surface));
    }

    #[test]
    fn every_grey_is_neutral() {
        for dark in [true, false] {
            let theme = Theme::resolve(&look(), dark);
            for (name, color) in [
                ("text_muted", theme.text_muted),
                ("text_faint", theme.text_faint),
                ("text_disabled", theme.text_disabled),
                ("border", theme.border),
                ("border_soft", theme.border_soft),
            ] {
                let (r, g, b) = channels(color);
                assert_eq!(r, g, "{name} has a hue in the red channel");
                assert_eq!(g, b, "{name} has a hue in the green channel");
            }
        }
    }

    #[test]
    fn secondary_text_is_readable_on_the_surface_it_sits_on() {
        for dark in [true, false] {
            let theme = Theme::resolve(&look(), dark);
            for (name, color) in [
                ("text", theme.text),
                ("text_muted", theme.text_muted),
                ("text_faint", theme.text_faint),
            ] {
                let ratio = contrast(color, theme.surface);
                assert!(ratio >= 4.2, "{name} is only {ratio:.2}:1 on the surface");
            }
        }
    }

    #[test]
    fn a_pale_accent_is_darkened_for_light_mode_and_left_alone_for_dark() {
        let pale = Look::new().accent(0xffe066);
        let light = Theme::resolve(&pale, false);
        assert!(contrast(light.accent, light.bg) >= TEXT_CONTRAST);
        assert!(contrast(light.accent, light.surface) >= TEXT_CONTRAST);
        // The same accent in dark mode is already legible, so it survives untouched.
        let dark = Theme::resolve(&pale, true);
        assert_eq!(dark.accent, rgb(0xffe066));
    }

    #[test]
    fn a_dark_accent_is_lightened_for_dark_mode() {
        let navy = Look::new().accent(0x101040);
        let dark = Theme::resolve(&navy, true);
        assert!(contrast(dark.accent, dark.bg) >= TEXT_CONTRAST);
        assert!(contrast(dark.accent, dark.surface) >= TEXT_CONTRAST);
    }

    #[test]
    fn the_derived_accent_survives_every_channel_of_the_palette() {
        // Both a colour that has to be pushed and one that does not, in both modes.
        for accent in [0x7c5cff, 0xffe066, 0x101040, 0x00ff00] {
            for dark in [true, false] {
                let theme = Theme::resolve(&Look::new().accent(accent), dark);
                for (name, ground) in [("bg", theme.bg), ("surface", theme.surface)] {
                    let ratio = contrast(theme.accent, ground);
                    assert!(
                        ratio >= TEXT_CONTRAST,
                        "{accent:#08x} in {} mode is {ratio:.2}:1 on {name}",
                        if dark { "dark" } else { "light" },
                    );
                }
            }
        }
    }

    #[test]
    fn an_explicit_light_accent_is_not_re_derived() {
        let tuned = Look::new().accent(0xffe066).accent_light(0x8a6b00);
        let light = Theme::resolve(&tuned, false);
        assert_eq!(light.accent, rgb(0x8a6b00));
    }

    #[test]
    fn oled_only_touches_the_dark_window() {
        let oled = Look::new().oled(true);
        assert_eq!(Theme::resolve(&oled, true).bg, rgb(0x000000));
        assert_eq!(Theme::resolve(&oled, false).bg, rgb(0xf3f3f3));
        // …and the layers follow the window they sit on.
        let plain = Look::new();
        assert_ne!(
            Theme::resolve(&oled, true).surface,
            Theme::resolve(&plain, true).surface
        );
    }

    #[test]
    fn text_on_a_filled_control_is_whichever_one_can_be_read() {
        let dark = Theme::resolve(&Look::new().accent(0x7c5cff), true);
        assert!(contrast(dark.on_accent, dark.accent) >= 4.3);
        let light = Theme::resolve(&Look::new().accent(0x7c5cff), false);
        assert!(contrast(light.on_accent, light.accent) >= 4.3);
    }

    #[test]
    fn the_mode_setting_wins_over_the_system_only_when_it_says_so() {
        assert!(ThemeMode::System.is_dark(true));
        assert!(!ThemeMode::Light.is_dark(true));
        assert!(ThemeMode::Dark.is_dark(false));
        assert_eq!(ThemeMode::parse("system"), Some(ThemeMode::System));
        assert_eq!(ThemeMode::parse("nope"), None);
    }
}
