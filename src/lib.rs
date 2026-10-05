//! Nexus-look — one visual language for GPUI-CE desktop apps.
//!
//! The rules live in `STYLE.md` at the repository root; **this crate is their only
//! implementation**. Nothing here is decoration for a single app: every widget exists because a
//! second, hand-rolled copy of it in some project would drift from the first.
//!
//! Three ideas carry the whole look, and every widget obeys them:
//!
//! - **Three colours** — black, white, and the project's accent. Everything else is black or
//!   white at some alpha, except a small group of semantic colours that may only be used where
//!   they *describe a state*.
//! - **One radius** — every rectangle is [`RADIUS`]; pills are a different *shape*, not a
//!   different radius.
//! - **Two heights** — [`CONTROL`] and [`CONTROL_LG`]. A row with two controls in it has exactly
//!   two usable heights, and both of them line up.
//!
//! ```ignore
//! let look = Look::new().accent(0x7c5cff).oled(false);
//! gpui_platform::application()
//!     .with_assets(nexus_look::Assets.chain(my_icons))
//!     .run(move |cx| {
//!         nexus_look::init(cx, look);
//!         // …open a window; `Theme::of(cx)` answers with the palette from now on.
//!     });
//! ```
//!
//! Call sites supply data and behaviour; colour, size, spacing and timing come from here.

pub mod assets;
pub mod icons;
pub mod theme;
pub mod tokens;
pub mod widgets;

use gpui::App;

pub use assets::Assets;
pub use theme::{Look, PRESET_ACCENTS, Theme, ThemeMode, Tone, contrast, readable, readable_on};
pub use tokens::*;
pub use widgets::{
    Button, Choice, IconButton, Modal, Row, Segmented, SettingGroup, Sizing, SwatchGrid, Switch,
    TextEdit, TextInput, TitleBar, Toast, Variant, card, divider, heading, hint, icon,
    modal_body_max, modal_width, scroll_fade, subheading,
};

/// Install the look: the configuration and the palette that follows from it.
///
/// Both go in as globals, which is what lets a widget read the font and the colours without every
/// call site threading them through — and what makes changing the accent in a settings sheet reach
/// every widget on the next frame.
pub fn init(cx: &mut App, look: Look) -> Theme {
    look.install(cx)
}
