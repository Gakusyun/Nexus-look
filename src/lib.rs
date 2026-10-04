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
//! - **One radius** — every rectangle is [`RADIUS`] millimetres-of-pixels; pills are a different
//!   *shape*, not a different radius.
//! - **Two heights** — [`CONTROL`] and [`CONTROL_LG`]. A row with two controls in it has exactly
//!   two usable heights, and both of them line up.
//!
//! Call sites supply data and behaviour; colour, size, spacing and timing come from here.

pub mod tokens;

pub use tokens::*;
