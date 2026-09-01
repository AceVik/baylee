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
    /// The text now in the field. Not a keystroke: autofill and paste both
    /// arrive as a whole new value.
    Text(String),
    /// The keyboard's action key ("go", "return") was pressed.
    Submit,
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
    /// The value last handed to the lobby, so an unchanged field is not
    /// reported every frame.
    last: String,
    /// Whether the input currently holds focus.
    open: bool,
}

/// The element, and the listener that has to outlive it.
#[cfg(target_arch = "wasm32")]
struct Element {
    input: web_sys::HtmlInputElement,
    /// Set by the `keydown` listener when the action key is pressed.
    submitted: std::rc::Rc<std::cell::Cell<bool>>,
    /// Dropping this detaches the callback, so it is kept for exactly as long
    /// as the element is.
    _keydown: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::KeyboardEvent)>,
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
            let flag = std::rc::Rc::clone(&submitted);
            let keydown = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                move |event: web_sys::KeyboardEvent| {
                    if event.key() == "Enter" {
                        // Otherwise the browser tries to submit a form that
                        // does not exist, which on iOS reloads the page.
                        event.prevent_default();
                        flag.set(true);
                    }
                },
            );
            let _ =
                input.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref());
            self.element = Some(send_wrapper::SendWrapper::new(Element {
                input,
                submitted,
                _keydown: keydown,
            }));
        }
        self.element.as_deref()
    }

    fn open(&mut self, kind: FieldKind, value: &str) {
        // Autofill only works when the browser is told what the field is for,
        // and the keyboard layout follows the same hint.
        let (input_type, mode, complete) = match kind {
            FieldKind::Email => ("email", "email", "username"),
            FieldKind::Name => ("text", "text", "nickname"),
            FieldKind::Password => ("password", "text", "current-password"),
        };
        self.last = value.to_string();
        let Some(element) = self.element() else {
            return;
        };
        element.input.set_type(input_type);
        let _ = element.input.set_attribute("inputmode", mode);
        let _ = element.input.set_attribute("autocomplete", complete);
        let _ = element.input.set_attribute("enterkeyhint", "go");
        element.input.set_value(value);
        element.submitted.set(false);
        let _ = element.input.focus();
        self.open = true;
    }

    fn close(&mut self) {
        if !self.open {
            return;
        }
        self.open = false;
        self.last.clear();
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
        let Some((value, submitted)) = self
            .element()
            .map(|e| (e.input.value(), e.submitted.replace(false)))
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if value != self.last {
            self.last.clone_from(&value);
            out.push(SoftKey::Text(value));
        }
        if submitted {
            out.push(SoftKey::Submit);
        }
        out
    }
}
