//! The one text input.
//!
//! It owns its buffer and its focus, so a call site creates one with `cx.new` and otherwise treats
//! it as an opaque widget. That matters more than it sounds: the two halves of a working field are
//! the two halves that are easy to get wrong, and both used to be the caller's problem.
//!
//! * **The platform side.** `Window::handle_input` with an `EntityInputHandler` is the only path
//!   IME and `WM_CHAR` take. Without it the Windows backend drops every composed character — the
//!   user types Chinese and literally nothing arrives — and `key_char` from `WM_KEYDOWN` is the
//!   only text that ever reaches the app.
//! * **The caret arithmetic.** Turning a click into a byte offset needs the same font and the same
//!   leftward scroll the line was drawn with, or the caret lands beside the character the user
//!   pointed at. `handle_input` asserts it runs during paint, so the platform registration is a
//!   `canvas` overlaid on the field rather than a handler on the field's own `div`.
//!
//! Anything a call site could get wrong here it no longer can: `track_focus` is on the element that
//! measures the caret (a field that does not track its handle still *looks* focused while owning no
//! node in the dispatch tree, so keystrokes are dispatched from the window root and dropped — which
//! reads as "backspace does nothing"), and the font is resolved once, from `Look`.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    App, Bounds, ClipboardItem, ColorExt, Context, DispatchPhase, ElementInputHandler,
    EntityInputHandler, FocusHandle, Font, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Render, SharedString, TextRun, UTF16Selection, Window,
    canvas, div, point, px, size,
};

use super::Sizing;
use super::text_edit::{self, TextEdit, one_line};
use crate::theme::{Look, Theme};
use crate::tokens::{CARET, CARET_RATIO, RADIUS, space, text};

/// How long the caret stays on or off. The Windows default, so a Nexus-look field blinks in step
/// with every other field on the machine.
const BLINK: Duration = Duration::from_millis(530);

/// What a keystroke meant, so the widget knows what to tell its owner.
pub enum Outcome {
    /// The field consumed it: caret movement, editing, clipboard.
    Handled,
    /// Enter — the caller's "go" key.
    Submit,
    /// Escape or Tab — let go of the field, or close the layer that owns it.
    Dismiss,
    Ignored,
}

pub type ChangeHandler = Box<dyn Fn(&str, &mut Window, &mut App) + 'static>;

/// Escape or Tab. The owner usually closes whatever layer the field is in.
pub type DismissHandler = Box<dyn Fn(&mut Window, &mut App) + 'static>;

/// A single-line editable field.
pub struct TextInput {
    edit: TextEdit,
    focus: FocusHandle,
    placeholder: SharedString,
    sizing: Sizing,
    /// How much room the surrounding layout leaves for the *text*. Only the caret's scroll offset
    /// depends on it, and erring small only ever keeps the caret visible. Callers that know their
    /// column width pass it; the rest get a viewport-based estimate.
    text_width: Option<f32>,
    caret_on: bool,
    focused: bool,
    dragging: bool,
    on_change: Option<ChangeHandler>,
    on_submit: Option<ChangeHandler>,
    on_dismiss: Option<DismissHandler>,
}

impl TextInput {
    /// Create a field. Needs a `Context` because it owns a focus handle and starts its own caret
    /// blink, so it has to be an entity rather than a value.
    pub fn new(cx: &mut Context<Self>, placeholder: impl Into<SharedString>) -> Self {
        let focus = cx.focus_handle();
        // One timer per field, for as long as the field lives. It only does work while focused,
        // and it beats on the entity's own clock rather than piggy-backing on the host's repaints:
        // a field that stops blinking because some unrelated view stopped polling is a field that
        // looks broken.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(BLINK).await;
                if this
                    .update(cx, |this, cx| {
                        if this.focused {
                            this.caret_on = !this.caret_on;
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    return;
                }
            }
        })
        .detach();

        Self {
            edit: TextEdit::default(),
            focus,
            placeholder: placeholder.into(),
            sizing: Sizing::Md,
            text_width: None,
            caret_on: true,
            focused: false,
            dragging: false,
            on_change: None,
            on_submit: None,
            on_dismiss: None,
        }
    }

    pub fn large(mut self) -> Self {
        self.sizing = Sizing::Lg;
        self
    }

    /// How much room the layout leaves for the text. See the field docs.
    pub fn text_width(mut self, width: f32) -> Self {
        self.text_width = Some(width);
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    pub fn on_submit(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_submit = Some(Box::new(handler));
        self
    }

    /// Escape or Tab. The owner usually closes whatever layer the field is in.
    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Box::new(handler));
        self
    }

    pub fn text(&self) -> &str {
        self.edit.text()
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// Replace the whole value. Notifies, so the caller does not have to remember to.
    pub fn set_text(&mut self, value: impl Into<String>, cx: &mut Context<Self>) {
        self.edit.set(value);
        self.caret_on = true;
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.set_text(String::new(), cx);
    }

    fn notify_change(&self, window: &mut Window, cx: &mut App) {
        if let Some(handler) = &self.on_change {
            handler(self.edit.text(), window, cx);
        }
    }

    /// How much room the line has before it must scroll.
    fn available(&self, window: &Window) -> f32 {
        self.text_width
            .unwrap_or_else(|| (window.viewport_size().width.as_f32() - 160.0).max(60.0))
    }
}

/// Apply a keystroke to the buffer.
///
/// **Printable characters are deliberately not inserted here.** The platform delivers them through
/// `ElementInputHandler::replace_text_in_range` — Windows sends plain ASCII typing as `WM_CHAR`,
/// not only IME composition — so also inserting `key_char` would double every letter.
fn apply_key(edit: &mut TextEdit, event: &KeyDownEvent, cx: &mut App) -> Outcome {
    let modifiers = event.keystroke.modifiers;
    let key = event.keystroke.key.as_str();

    if modifiers.control || modifiers.platform {
        return match key {
            "a" => {
                edit.select_all();
                Outcome::Handled
            }
            "c" => {
                if let Some(text) = edit.selected_text() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
                Outcome::Handled
            }
            "x" => {
                if let Some(text) = edit.cut() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
                Outcome::Handled
            }
            "v" => {
                if let Some(pasted) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    edit.insert(&one_line(&pasted));
                }
                Outcome::Handled
            }
            _ => Outcome::Ignored,
        };
    }

    let extend = modifiers.shift;
    match key {
        "backspace" => edit.backspace(),
        "delete" => edit.delete_forward(),
        "left" => edit.move_left(extend),
        "right" => edit.move_right(extend),
        "home" => edit.move_home(extend),
        "end" => edit.move_end(extend),
        "enter" => return Outcome::Submit,
        "escape" | "tab" => return Outcome::Dismiss,
        _ => return Outcome::Ignored,
    }
    Outcome::Handled
}

/// The caret height for a font of `size` — a touch taller than the em box, the way a text cursor
/// reads next to glyphs of any script.
fn caret_height(size: f32) -> f32 {
    (size * CARET_RATIO).max(12.0)
}

/// Shape one line with an explicit font. Used for hit-testing and for the IME's caret query, both
/// of which run outside `render` and so cannot rely on the window's inherited style.
fn shape(text: &str, size: f32, window: &Window, font: &Font) -> Option<gpui::ShapedLine> {
    let run = TextRun {
        len: text.len(),
        font: font.clone(),
        ..Default::default()
    };
    Some(window.text_system().shape_line(
        SharedString::from(text.to_string()),
        px(size),
        std::slice::from_ref(&run),
        None,
    ))
}

/// Where the caret sits along the line, in pixels from the text's own left edge. Shaped whole,
/// exactly as hit-testing and the IME do it, so the quad lands on the same glyph boundary a click
/// would.
fn caret_offset(text: &str, cursor: usize, size: f32, window: &Window, font: &Font) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    match shape(text, size, window, font) {
        Some(line) => line.split_at(cursor.min(text.len())).0.width().as_f32(),
        None => 0.0,
    }
}

/// How far the line must slide left for the caret to stay in view.
fn caret_shift(
    text: &str,
    cursor: usize,
    size: f32,
    window: &Window,
    font: &Font,
    available: f32,
) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let Some(line) = shape(text, size, window, font) else {
        return 0.0;
    };
    let prefix = line.split_at(cursor.min(text.len())).0.width().as_f32();
    (prefix + CARET - available).max(0.0)
}

/// `Bounds` for a byte range, in window coordinates. Used by the input handler to tell the IME
/// where the caret is, so its candidate window appears next to the text.
fn range_bounds(
    line: &gpui::ShapedLine,
    bytes: std::ops::Range<usize>,
    element: Bounds<Pixels>,
    shift: f32,
    height: Pixels,
) -> Bounds<Pixels> {
    let start = line.split_at(bytes.start).0.width();
    let end = line.split_at(bytes.end).0.width();
    let origin = element.origin - point(px(shift), px(0.0));
    Bounds::new(
        origin + point(start, px(0.0)),
        size((end - start).max(px(CARET)), height),
    )
}

/// The caret itself: an overlay, never a flex child.
///
/// A `div` in the row pushes every glyph after it sideways each time it disappears and comes back —
/// the line twitches on every blink. Positioned absolutely at the measured glyph boundary it cannot
/// take part in layout at all.
fn caret(x: f32, size: f32, color: gpui::Rgba) -> impl IntoElement + use<> {
    div()
        .absolute()
        .left(px(x))
        .top_0()
        .bottom_0()
        .w(px(CARET))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w(px(CARET))
                .h(px(caret_height(size)))
                .rounded_full()
                .bg(color),
        )
}

/// The inside of a field: the selection behind the text, the IME composition underlined, and the
/// caret. `shift` is how far the line is slid left so the caret stays in view.
fn contents(
    edit: &TextEdit,
    shift: f32,
    caret_at: Option<f32>,
    size: f32,
    focused: bool,
    placeholder: &SharedString,
    theme: &Theme,
) -> impl IntoElement + use<> {
    let mut row = div()
        .relative()
        .flex()
        .flex_row()
        .items_center()
        .flex_none()
        .ml(px(-shift))
        .text_size(px(size))
        .text_color(theme.text);

    if edit.is_empty() {
        // The placeholder steps aside the moment the field is in use, but keeps holding the row's
        // height while hidden — otherwise an empty focused box would collapse to nothing and take
        // the caret's centring with it.
        row = row.child(
            div()
                .text_color(if focused {
                    theme.text_faint.opacity(0.0)
                } else {
                    theme.text_faint
                })
                .child(placeholder.clone()),
        );
        return match caret_at {
            Some(x) => row.child(caret(x, size, theme.accent)),
            None => row,
        };
    }

    let body = edit.text();
    let len = edit.len();
    let selection = edit.selection();
    let marked = edit.marked();
    // Split at every edge that changes how a run is drawn, so each piece is a plain span with one
    // background and one decoration.
    let mut edges = vec![0, len];
    for range in [selection.as_ref(), marked.as_ref()].into_iter().flatten() {
        edges.push(range.start);
        edges.push(range.end);
    }
    edges.sort_unstable();
    edges.dedup();

    for pair in edges.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        if start == end {
            continue;
        }
        let selected = selection
            .as_ref()
            .is_some_and(|range| start >= range.start && end <= range.end);
        let composing = marked
            .as_ref()
            .is_some_and(|range| start >= range.start && end <= range.end);
        row = row.child(
            div()
                .flex_none()
                .when(selected, |span| span.bg(theme.selection))
                .when(composing, |span| span.underline())
                .child(SharedString::from(body[start..end].to_string())),
        );
    }

    // Last, so it paints over the glyph it sits beside; out of flow, so the segments around it keep
    // exactly the spacing they have while it is hidden.
    if let Some(x) = caret_at {
        row = row.child(caret(x, size, theme.accent));
    }

    row
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let font = Look::of(cx).font(window);

        // Focus is only knowable from a window, and render is the one place a widget reliably has
        // one. The blink timer reads the flag this sets.
        let focused = self.focus.is_focused(window);
        if focused != self.focused {
            self.focused = focused;
            self.caret_on = true;
        }

        let size = text::BODY;
        let height = self.sizing.height();
        let available = self.available(window);
        let shift = caret_shift(
            self.edit.text(),
            self.edit.cursor(),
            size,
            window,
            &font,
            available,
        );
        let caret_at = (focused && self.caret_on)
            .then(|| caret_offset(self.edit.text(), self.edit.cursor(), size, window, &font));

        let key_listener = cx.listener(|this, event: &KeyDownEvent, window, cx| {
            match apply_key(&mut this.edit, event, cx) {
                Outcome::Handled => {
                    this.caret_on = true;
                    let text = this.edit.text().to_string();
                    if let Some(handler) = &this.on_change {
                        handler(&text, window, cx);
                    }
                    cx.notify();
                }
                Outcome::Submit => {
                    let text = this.edit.text().to_string();
                    if let Some(handler) = &this.on_submit {
                        handler(&text, window, cx);
                    }
                }
                Outcome::Dismiss => {
                    if let Some(handler) = &this.on_dismiss {
                        handler(window, cx);
                    }
                }
                Outcome::Ignored => {}
            }
        });

        let activation = self.focus.clone();
        let dismiss = self.focus.clone();
        let entity = cx.entity();

        div()
            .id(&self.focus)
            // The handle has to be tracked here rather than at the call site: this is the element
            // the caret is measured in and the one the platform sends the input to.
            .track_focus(&self.focus)
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(space::SM))
            .h(px(height))
            .px(px(space::MD))
            .flex_none()
            .rounded(px(RADIUS))
            .bg(theme.field)
            .border_1()
            .border_color(if focused { theme.accent } else { theme.border })
            .font(font)
            .text_size(px(size))
            .cursor_text()
            .on_key_down(key_listener)
            // Clicking outside the box drops its focus. This is the framework's intended idiom for
            // "clicked elsewhere": `on_mouse_down_out` runs in the **capture** phase, before the
            // box's own focus handlers, so a click that *does* land in a box re-focuses it in the
            // same dispatch. A handler on the root would not work — every mouse handler shares one
            // listener list dispatched in reverse registration order, so an ancestor runs *last*
            // and would undo the focus it just set.
            .on_mouse_down_out(cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                if dismiss.is_focused(window) {
                    this.focused = false;
                    window.blur();
                    cx.notify();
                }
            }))
            // Clicking the chrome — the padding, the icon — focuses the box; only the text row
            // knows where inside it the pointer landed.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    this.focused = true;
                    this.caret_on = true;
                    window.focus(&activation, cx);
                    cx.notify();
                }),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_row()
                    .items_center()
                    .overflow_hidden()
                    .child(contents(
                        &self.edit,
                        shift,
                        caret_at,
                        size,
                        focused,
                        &self.placeholder,
                        &theme,
                    ))
                    .child(surface(&self.focus, entity, shift)),
            )
    }
}

/// Register a field with the platform and with the mouse.
///
/// Must be laid out during paint (`handle_input` asserts as much), which is why this is a `canvas`
/// overlay rather than a plain handler on the field's `div` — only an `Element` is handed its own
/// bounds, and the text input handler needs them to put the IME candidate window in the right
/// place. The parent must be `relative`, and the canvas covers it edge to edge.
fn surface(
    focus: &FocusHandle,
    entity: gpui::Entity<TextInput>,
    shift: f32,
) -> impl IntoElement + use<> {
    let focus = focus.clone();
    canvas(
        move |_bounds, _window, _cx| {},
        move |bounds, (), window, app| {
            if focus.is_focused(window) {
                window.handle_input(
                    &focus,
                    ElementInputHandler::new(bounds, entity.clone()),
                    app,
                );
            }
            mouse(focus.clone(), entity.clone(), bounds, shift, window);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A click puts the caret where it landed and starts a selection; a drag with the button held
/// extends it; letting go ends it.
fn mouse(
    focus: FocusHandle,
    entity: gpui::Entity<TextInput>,
    bounds: Bounds<Pixels>,
    shift: f32,
    window: &mut Window,
) {
    // The offset along the line that a pointer at `x` is over: the text is drawn `shift` pixels to
    // the left, so put that back before measuring.
    let along = {
        let origin = bounds.origin.x - px(shift);
        move |x: Pixels| (x - origin).as_f32()
    };

    window.on_mouse_event({
        let focus = focus.clone();
        let entity = entity.clone();
        move |event: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble
                || event.button != MouseButton::Left
                || !bounds.contains(&event.position)
            {
                return;
            }
            let along = along(event.position.x);
            entity.update(cx, |this, cx| {
                this.focused = true;
                this.caret_on = true;
                window.focus(&focus, cx);
                this.press(along, window, cx);
            });
        }
    });

    window.on_mouse_event({
        let entity = entity.clone();
        move |event: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble
                || event.pressed_button != Some(MouseButton::Left)
                || !bounds.contains(&event.position)
            {
                return;
            }
            let along = along(event.position.x);
            entity.update(cx, |this, cx| this.drag(along, window, cx));
        }
    });

    window.on_mouse_event(move |_: &MouseUpEvent, phase, _window, cx| {
        if phase != DispatchPhase::Bubble {
            return;
        }
        entity.update(cx, |this, _| this.dragging = false);
    });
}

impl TextInput {
    /// A press inside the field: put the caret where the click landed and start a selection.
    fn press(&mut self, along: f32, window: &Window, cx: &mut Context<Self>) {
        let index = self.index_for_x(along, window, cx);
        self.edit.set_cursor(index, false);
        self.dragging = true;
        cx.notify();
    }

    /// Extend a selection while the button is held down.
    fn drag(&mut self, along: f32, window: &Window, cx: &mut Context<Self>) {
        if !self.dragging {
            return;
        }
        let index = self.index_for_x(along, window, cx);
        self.edit.set_cursor(index, true);
        cx.notify();
    }

    /// The byte offset a point `along` the line lands on. Shaped with the font the field is drawn
    /// with, so the answer matches the pixels the user is pointing at.
    fn index_for_x(&self, along: f32, window: &Window, cx: &App) -> usize {
        let font = Look::of(cx).font(window);
        match shape(self.edit.text(), text::BODY, window, &font) {
            Some(line) => line.closest_index_for_x(px(along)),
            None => self.edit.len(),
        }
    }
}

/// The platform's view of the field.
///
/// This is not optional decoration: on Windows the IME delivers both its composition and its
/// committed characters **only** through the input handler, and `WM_CHAR` is routed there too. A
/// windowed app with no `EntityInputHandler` therefore types nothing at all in Chinese.
impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range: std::ops::Range<usize>,
        adjusted: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let bytes = self.edit.utf16_to_bytes(range);
        adjusted.replace(
            text_edit::byte_to_utf16(self.edit.text(), bytes.start)
                ..text_edit::byte_to_utf16(self.edit.text(), bytes.end),
        );
        self.edit.text().get(bytes).map(str::to_string)
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (range, reversed) = self.edit.utf16_range();
        Some(UTF16Selection { range, reversed })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        let marked = self.edit.marked()?;
        Some(
            text_edit::byte_to_utf16(self.edit.text(), marked.start)
                ..text_edit::byte_to_utf16(self.edit.text(), marked.end),
        )
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.edit.unmark();
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<std::ops::Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range.map(|range| self.edit.utf16_to_bytes(range));
        self.edit.insert_at(range, &one_line(text));
        self.caret_on = true;
        self.notify_change(window, cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<std::ops::Range<usize>>,
        new_text: &str,
        new_selected_range: Option<std::ops::Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range.map(|range| self.edit.utf16_to_bytes(range));
        let caret = new_selected_range.map(|range| text_edit::utf16_to_byte(new_text, range.start));
        self.edit
            .insert_marked_at(range, &one_line(new_text), caret);
        self.caret_on = true;
        self.notify_change(window, cx);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        element_bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let font = Look::of(cx).font(window);
        let line = shape(self.edit.text(), text::BODY, window, &font)?;
        // The scroll offset is recomputed from the element's real width instead of read back from
        // the frame: `render` can only estimate the width, and a few pixels of disagreement moves
        // nothing but the IME's candidate window.
        let bytes = self.edit.utf16_to_bytes(range_utf16);
        let available = element_bounds.size.width.as_f32().max(0.0);
        let shift = caret_shift(
            self.edit.text(),
            self.edit.cursor(),
            text::BODY,
            window,
            &font,
            available,
        );
        Some(range_bounds(
            &line,
            bytes,
            element_bounds,
            shift,
            window.line_height(),
        ))
    }

    /// Not called by the Windows backend, which positions its candidate window from
    /// `bounds_for_range` alone. `None` is the honest answer rather than a wrong offset.
    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }

    /// The IME moves the caret through this one.
    fn set_selected_text_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let bytes = self.edit.utf16_to_bytes(range_utf16);
        self.edit.set_cursor(bytes.start, false);
        cx.notify();
    }

    fn text_length_utf16(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.edit.utf16_len())
    }
}
