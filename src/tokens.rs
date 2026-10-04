//! The numbers. Every one of them is referenced by name from the widgets, so a call site that
//! types a literal has visibly stepped outside the language.
//!
//! They are plain `f32` rather than `Pixels` on purpose: they are the *spec*, and the spec should
//! be readable without pulling GPUI's types into your head. Widgets wrap them in `px()`.

// ---------------------------------------------------------------------------------------------
// Shape
// ---------------------------------------------------------------------------------------------

/// The only corner radius in the language.
///
/// Concentric inners are not an exception: a segmented control's container and its selected
/// option are both `RADIUS`, and the container's 2px padding is what makes that look right.
/// Nested radii ("outer minus padding") would be a second value, and a second value is how
/// "slightly off" starts.
pub const RADIUS: f32 = 8.0;

/// Vertical thickness of a bar that is drawn, not laid out: dividers, input borders.
pub const STROKE: f32 = 1.0;

/// The focus ring and the tab indicator.
pub const THICK_STROKE: f32 = 2.0;

// ---------------------------------------------------------------------------------------------
// Size
// ---------------------------------------------------------------------------------------------

/// The one control height. Buttons, icon buttons, inputs, segmented controls and chips.
pub const CONTROL: f32 = 32.0;

/// The one large control height. Two places earn it: the command bar's main input and the page's
/// primary call to action. Anything else using it is a mistake, because a screen full of large
/// controls has no large controls.
pub const CONTROL_LG: f32 = 40.0;

/// Padding inside a segmented control, and therefore the height of one of its options:
/// `CONTROL - SEGMENT_PAD * 2 - STROKE * 2` adds up to exactly [`SEGMENT`], so a picker and the box
/// beside it in the same row share a top and a bottom edge.
pub const SEGMENT_PAD: f32 = 2.0;
pub const SEGMENT: f32 = CONTROL - SEGMENT_PAD * 2.0 - STROKE * 2.0;

/// A list row: two lines of text, a progress bar, and the row's actions.
pub const ROW: f32 = 56.0;

/// The title bar. Window buttons are [`CONTROL`] square, centred in it.
pub const TITLEBAR: f32 = 40.0;

/// The tab strip: text on a 2px indicator, no container.
pub const TABS: f32 = 40.0;

/// Width of the detail panel.
pub const PANEL: f32 = 360.0;

pub const PROGRESS: f32 = 4.0;
pub const BADGE: f32 = 20.0;

/// One colour swatch in the accent picker: round, 32 across. Fixed rather than tunable because
/// it is a target a finger aims at, and two projects' pickers with two sizes read as two
/// different widgets.
pub const SWATCH: f32 = 32.0;
/// The dot inside the swatch that marks the default accent.
pub const SWATCH_DOT: f32 = 8.0;

pub const ICON_SM: f32 = 16.0;
pub const ICON_MD: f32 = 20.0;
pub const ICON_LG: f32 = 32.0;

/// The text caret's width; its height is the font size times [`CARET_RATIO`].
pub const CARET: f32 = 1.5;
pub const CARET_RATIO: f32 = 1.2;

/// Height of the fade that stands in for the scrollbar GPUI-CE does not draw. Scrolling regions
/// pad their bottom by this much so real content is never dimmed.
pub const FADE: f32 = 16.0;

/// The smallest window worth painting: with the detail panel open the list keeps ~460px, which
/// is where a two-line row stops being readable. The window code enforces it.
pub const MIN_WINDOW: (f32, f32) = (880.0, 560.0);

// ---------------------------------------------------------------------------------------------
// Space
// ---------------------------------------------------------------------------------------------

/// Multiples of four, and nothing else.
pub mod space {
    /// Badge padding, container padding of a segmented control.
    pub const XS: f32 = 4.0;
    /// Between buttons in a row, between an icon and its label.
    pub const SM: f32 = 8.0;
    /// Between inline elements, inside a card's inner group.
    pub const MD: f32 = 12.0;
    /// Card padding, list row padding, between cards.
    pub const LG: f32 = 16.0;
    /// Page gutters, modal padding.
    pub const XL: f32 = 24.0;
    /// Page top and bottom, between the empty state and everything else.
    pub const XXL: f32 = 32.0;
}

/// Gutters and paddings that are fixed by the layout, not chosen per screen.
pub mod layout {
    /// Left and right gutter of the page.
    pub const PAGE_PAD: f32 = 24.0;
    /// Padding inside a card.
    pub const CARD_PAD: f32 = 16.0;
    pub const MODAL_PAD: f32 = 24.0;
    /// Vertical gap between blocks inside a modal.
    pub const MODAL_GAP: f32 = 16.0;
    /// The settings/new-download table's control column.
    pub const CONTROL_COL: f32 = 320.0;

    /// Width of a table-shaped modal (settings, new download).
    pub const MODAL_W: f32 = 640.0;
    /// Floor for the above when the window is small; below this the table stops making sense.
    pub const MODAL_W_MIN: f32 = 560.0;
    /// Room a modal always leaves at the left and right edges of the window.
    pub const MODAL_W_SLACK: f32 = 80.0;
    /// A confirmation's width: one sentence and two buttons. A 640px card would be mostly
    /// whitespace, and whitespace around a destructive question reads as uncertainty.
    pub const MODAL_W_NARROW: f32 = 400.0;
    /// How much room the modal leaves the window at top and bottom.
    pub const MODAL_MAX_H_SLACK: f32 = 240.0;
    pub const MODAL_MAX_H_MIN: f32 = 240.0;
    pub const MODAL_MAX_H_MAX: f32 = 560.0;
}

// ---------------------------------------------------------------------------------------------
// Type
// ---------------------------------------------------------------------------------------------

/// Four sizes, two weights, five roles. A view that picks a size by feel has left the language.
pub mod text {
    /// Page title, modal title, empty-state headline.
    pub const TITLE: f32 = 20.0;
    /// Group heading, the task name in the detail panel, confirmation title.
    pub const HEADING: f32 = 16.0;
    /// Body copy, buttons, inputs, setting labels.
    pub const BODY: f32 = 14.0;
    /// The task name in a list row: same size as body, heavier so it reads as the row's subject.
    pub const BODY_STRONG: f32 = 14.0;
    /// Badges, chips, field notes, hints, percentages.
    pub const CAPTION: f32 = 12.0;
}

// ---------------------------------------------------------------------------------------------
// Motion
// ---------------------------------------------------------------------------------------------

/// Two durations, one easing curve per direction, and no third of either.
///
/// Every animation goes through GPUI's `with_animation`, which honours `App::reduce_motion`
/// for us (`gpui-ce/src/elements/animation.rs:46`). Hand-rolled timers do not, which is the whole
/// reason this module exists.
pub mod motion {
    use std::time::Duration;

    // Hover and press are deliberately absent: they are resolved while painting, with no fade at
    // all (see `STYLE.md` §6.1). A duration here would invite someone to reintroduce the bug.
    /// Fade, tab indicator, progress fill.
    pub const BASE: Duration = Duration::from_millis(160);
    /// Modal in and out, detail panel sliding.
    pub const SLOW: Duration = Duration::from_millis(220);

    /// How far the detail panel travels, and how far a modal lifts on entry.
    pub const SHIFT: f32 = 8.0;
}
