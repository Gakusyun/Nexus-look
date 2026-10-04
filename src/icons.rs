//! The names of the icons the library ships.
//!
//! Every widget that draws a glyph draws one of these, and call sites that need an icon of their
//! own pass a path of their own. There is deliberately no `icon("close")` — stringly-typed paths
//! fail at paint time, silently, because a missing path just draws nothing.

pub const ALERT: &str = "nexus-look/alert.svg";
pub const CHECK: &str = "nexus-look/check.svg";
pub const CHEVRON_DOWN: &str = "nexus-look/chevron-down.svg";
pub const CHEVRON_RIGHT: &str = "nexus-look/chevron-right.svg";
/// The window-control close glyph, and the "no" of any inline dismissal.
pub const CLOSE: &str = "nexus-look/close.svg";
pub const FOLDER: &str = "nexus-look/folder.svg";
pub const GEAR: &str = "nexus-look/gear.svg";
pub const LINK: &str = "nexus-look/link.svg";
pub const MAXIMIZE: &str = "nexus-look/max.svg";
pub const MINIMIZE: &str = "nexus-look/min.svg";
pub const MINUS: &str = "nexus-look/minus.svg";
pub const PLUS: &str = "nexus-look/plus.svg";
pub const RESTORE: &str = "nexus-look/restore.svg";
pub const SEARCH: &str = "nexus-look/search.svg";
pub const TRASH: &str = "nexus-look/trash.svg";
