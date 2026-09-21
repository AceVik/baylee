//! The platform's own text input, for the platforms that have one.
//!
//! A canvas never raises a soft keyboard. On a phone that makes the sign-in
//! form unusable, and on desktop web it costs the player autofill, password
//! managers, IME composition and paste — all of which live in the browser's
//! `<input>`, not in the key events a game engine sees.
//!
//! So on wasm the client keeps one real, invisible `<input>` over the page.
//! Focusing a lobby field focuses it; the browser does the typing, and the
//! client reads the value back and draws it itself. Everywhere else this is a
//! no-op and the ordinary key-event path handles the typing.
//!
//! The element is *invisible*, never hidden: `display:none` and
//! `visibility:hidden` cannot hold focus, and focus is the entire point.

use baylee_client_core::lobby::FieldKind;
use bevy::prelude::Resource;

/// What the platform's input reported since the last frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SoftKey {
    /// The text now in the field, with the platform's own caret in it. Not a
    /// keystroke: autofill and paste both arrive as a whole new value.
    Text {
        /// What the field holds now.
        value: String,
        /// Where the caret is, as a byte offset into `value`.
        cursor: usize,
        /// The other end of the selection, when there is one.
        anchor: Option<usize>,
    },
    /// The caret moved and the text did not — an arrow key inside the
    /// element, or ⌘A.
    ///
    /// Its own variant rather than a [`SoftKey::Text`] carrying the string it
    /// already had, because a reader that wants only the text would answer an
    /// arrow key by writing back the value it was already holding — and one
    /// of those readers narrows a list and scrolls it home again.
    Caret {
        /// Where the caret is now, as a byte offset into the unchanged text.
        cursor: usize,
        /// The other end of the selection, when there is one.
        anchor: Option<usize>,
    },
    /// The keyboard's action key ("go", "return") was pressed.
    Submit,
    /// Escape was pressed while the field held the keyboard.
    ///
    /// It has to travel this way because it cannot travel the other: the
    /// `<input>` is where the focus is, so the canvas never sees the key and
    /// the client's own `Action::Cancel` never fires. Without this, the only
    /// way out of a field in a browser is the keyboard's own action key.
    Dismiss,
}

/// The platform's text input, when it has one.
#[derive(Resource, Default)]
pub struct SoftKeyboard {
    inner: Inner,
}

impl SoftKeyboard {
    /// Whether this platform does the typing rather than the client.
    ///
    /// Where it does, the client must not also read raw key events, or a
    /// character would be entered twice.
    #[must_use]
    pub fn owns_typing() -> bool {
        cfg!(target_arch = "wasm32")
    }

    /// Points the input at a field and shows the keyboard.
    pub fn open(&mut self, kind: FieldKind, value: &str) {
        self.inner.open(kind, value);
    }

    /// Dismisses the keyboard.
    pub fn close(&mut self) {
        self.inner.close();
    }

    /// What the player has done since the last frame.
    pub fn drain(&mut self) -> Vec<SoftKey> {
        self.inner.drain()
    }
}

/// The no-op back end: every platform that already has a keyboard attached.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct Inner;

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::unused_self)] // the wasm back end below needs every one of them
impl Inner {
    fn open(&mut self, _kind: FieldKind, _value: &str) {}
    fn close(&mut self) {}
    fn drain(&mut self) -> Vec<SoftKey> {
        Vec::new()
    }
}

/// The browser back end: one `<input>`, focused and read every frame.
#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct Inner {
    /// The element, once the page has been reached. `SendWrapper` because a
    /// Bevy resource must be `Send + Sync` and a DOM handle belongs to the
    /// thread that made it — of which wasm has exactly one.
    element: Option<send_wrapper::SendWrapper<Element>>,
    /// What was last handed to the lobby, so an unchanged field is not
    /// reported every frame. `None` until the first drain after an
    /// [`Inner::open`], which must report whatever the element says even when
    /// it says what was opened with: opening puts the browser's caret at the
    /// end of the value, and a buffer still holding the caret from the last
    /// visit to that field would draw one in a place the browser will not
    /// type at.
    last: Option<Seen>,
    /// Whether the input currently holds focus.
    open: bool,
}

/// The element's text and caret as they were last read.
#[cfg(target_arch = "wasm32")]
struct Seen {
    value: String,
    cursor: usize,
    anchor: Option<usize>,
}

/// The element, and the listener that has to outlive it.
#[cfg(target_arch = "wasm32")]
struct Element {
    input: web_sys::HtmlInputElement,
    /// Set by the `keydown` listener when the action key is pressed.
    submitted: std::rc::Rc<std::cell::Cell<bool>>,
    /// Set by the same listener when Escape is pressed.
    dismissed: std::rc::Rc<std::cell::Cell<bool>>,
    /// Dropping this detaches the callback, so it is kept for exactly as long
    /// as the element is.
    _keydown: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::KeyboardEvent)>,
}

#[cfg(target_arch = "wasm32")]
impl Element {
    /// The value and the caret together, because a caret is an offset into a
    /// particular string and the two must be read on the same frame.
    fn read(&self) -> Seen {
        let value = self.input.value();
        let units: usize = value.chars().map(char::len_utf16).sum();
        // An element that will not say — one whose type the selection API
        // does not apply to, or one the page has not laid out yet — is read
        // as a caret after the text, which is where setting a value leaves
        // it.
        let at = |offset: Option<u32>| {
            offset
                .and_then(|o| usize::try_from(o).ok())
                .unwrap_or(units)
                .min(units)
        };
        let start = at(self.input.selection_start().ok().flatten());
        let end = at(self.input.selection_end().ok().flatten());
        let backward = self
            .input
            .selection_direction()
            .ok()
            .flatten()
            .is_some_and(|d| d == "backward");
        let head = baylee_client_core::textbuf::byte_of_utf16(&value, start);
        let tail = baylee_client_core::textbuf::byte_of_utf16(&value, end);
        // `selectionStart` is always the lower of the two offsets, so which
        // end of a selection the caret sits at is a third question with an
        // answer of its own. Without it, ⇧← from the end of a field draws the
        // caret on the far side of everything it just selected.
        let (cursor, anchor) = if head == tail {
            (head, None)
        } else if backward {
            (head, Some(tail))
        } else {
            (tail, Some(head))
        };
        Seen {
            value,
            cursor,
            anchor,
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Inner {
    /// The input, created on first use.
    ///
    /// `index.html` styles `#baylee-text-input`, but the client has to run in
    /// a page it did not write too, so it creates the element when it is
    /// missing and carries its own styling for that case.
    fn element(&mut self) -> Option<&Element> {
        use wasm_bindgen::JsCast as _;

        if self.element.is_none() {
            /// The element the page is expected to provide, and the id used
            /// for the one created here when it does not.
            const ID: &str = "baylee-text-input";
            /// Invisible, focusable, and out of the way. 16px keeps iOS from
            /// zooming the page when the field takes focus.
            const STYLE: &str = "position:fixed;left:0;bottom:0;width:1px;height:1px;\
                 padding:0;border:0;outline:none;opacity:0;font-size:16px;\
                 background:transparent;color:transparent;caret-color:transparent;";

            let document = web_sys::window()?.document()?;
            let input: web_sys::HtmlInputElement = match document.get_element_by_id(ID) {
                Some(found) => found.dyn_into().ok()?,
                None => {
                    let made: web_sys::HtmlInputElement =
                        document.create_element("input").ok()?.dyn_into().ok()?;
                    made.set_id(ID);
                    let _ = made.set_attribute("style", STYLE);
                    let _ = made.set_attribute("autocapitalize", "off");
                    let _ = made.set_attribute("autocorrect", "off");
                    let _ = made.set_attribute("spellcheck", "false");
                    document.body()?.append_child(&made).ok()?;
                    made
                }
            };
            let submitted = std::rc::Rc::new(std::cell::Cell::new(false));
            let dismissed = std::rc::Rc::new(std::cell::Cell::new(false));
            let flag = std::rc::Rc::clone(&submitted);
            let away = std::rc::Rc::clone(&dismissed);
            let keydown = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |event: web_sys::KeyboardEvent| {
                    match event.key().as_str() {
                        "Enter" => {
                            // Otherwise the browser tries to submit a form
                            // that does not exist, which on iOS reloads the
                            // page.
                            event.prevent_default();
                            flag.set(true);
                        }
                        // Some browsers answer Escape in a text field by
                        // reverting it to the value it was opened with, which
                        // would fight whoever reads this.
                        "Escape" => {
                            event.prevent_default();
                            away.set(true);
                        }
                        _ => {}
                    }
                },
            );
            let _ =
                input.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref());
            self.element = Some(send_wrapper::SendWrapper::new(Element {
                input,
                submitted,
                dismissed,
                _keydown: keydown,
            }));
        }
        self.element.as_deref()
    }

    fn open(&mut self, kind: FieldKind, value: &str) {
        // Autofill only works when the browser is told what the field is for,
        // and the keyboard layout follows the same hint.
        let (input_type, mode, complete) = match kind {
            // Deliberately not `type=email`. `selectionStart` and its
            // siblings apply to the text, search, tel, url and password types
            // and to no others (HTML, "do not apply"), so an e-mail field is
            // a field whose caret cannot be read — and this client draws the
            // caret itself. `inputmode` is what actually raises the address
            // keyboard on a phone; the type was buying the browser's own
            // validation on a form that does its own.
            FieldKind::Url => ("url", "url", "off"),
            FieldKind::Email => ("text", "email", "username"),
            FieldKind::Name => ("text", "text", "nickname"),
            FieldKind::Password => ("password", "text", "current-password"),
            // A password manager offers to *make* one here rather than
            // putting the account's existing password back into the form
            // that is creating the account.
            FieldKind::NewPassword => ("password", "text", "new-password"),
            // Masked, and nobody's credential: a room's password belongs to
            // the table, so a manager has nothing to offer and should not
            // ask to remember what is typed.
            FieldKind::Secret => ("password", "text", "off"),
        };
        // Not `Some(value)`: the first drain after opening has to report,
        // whatever the element then says. See the field.
        self.last = None;
        let Some(element) = self.element() else {
            return;
        };
        element.input.set_type(input_type);
        let _ = element.input.set_attribute("inputmode", mode);
        let _ = element.input.set_attribute("autocomplete", complete);
        let _ = element.input.set_attribute("enterkeyhint", "go");
        element.input.set_value(value);
        element.submitted.set(false);
        element.dismissed.set(false);
        let _ = element.input.focus();
        self.open = true;
    }

    fn close(&mut self) {
        if !self.open {
            return;
        }
        self.open = false;
        self.last = None;
        if let Some(element) = self.element() {
            // Clearing matters: the value is a password as often as not, and
            // a blurred input keeps whatever was left in it.
            element.input.set_value("");
            let _ = element.input.blur();
        }
    }

    fn drain(&mut self) -> Vec<SoftKey> {
        if !self.open {
            return Vec::new();
        }
        // Read everything out before touching `self` again: the element is
        // borrowed from it.
        let Some((seen, submitted, dismissed)) = self.element().map(|e| {
            (
                e.read(),
                e.submitted.replace(false),
                e.dismissed.replace(false),
            )
        }) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        match &self.last {
            // The text stands and only the caret moved, which is an arrow
            // key or a ⌘A and is not a change of value to anyone.
            Some(last) if last.value == seen.value => {
                if (last.cursor, last.anchor) != (seen.cursor, seen.anchor) {
                    out.push(SoftKey::Caret {
                        cursor: seen.cursor,
                        anchor: seen.anchor,
                    });
                }
            }
            _ => out.push(SoftKey::Text {
                value: seen.value.clone(),
                cursor: seen.cursor,
                anchor: seen.anchor,
            }),
        }
        self.last = Some(seen);
        if submitted {
            out.push(SoftKey::Submit);
        }
        // Last, and after the text: Escape means "undo what I typed and let
        // go", so whoever handles it has to have been told what was typed.
        if dismissed {
            out.push(SoftKey::Dismiss);
        }
        out
    }
}
