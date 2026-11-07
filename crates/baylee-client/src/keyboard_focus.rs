//! Keeps the keyboard with the window in front on Windows, which is what stops
//! the system's beep on every key.
//!
//! The window is created hidden and shown once frames are drawn
//! (`standalone::show_the_window`). On the way winit shows it for a moment:
//! `ShowWindow(SW_MAXIMIZE)` on a hidden window makes it visible, and winit
//! hides it again with `SW_HIDE`. A process with the right to the foreground
//! loses its active window there, and the last `SW_SHOW` activates it afresh,
//! which gives it the focus. A process without that right, because the player
//! went on working in another program while the client loaded, keeps the
//! window active in its own queue and loses only the focus, so the last
//! `SW_SHOW` has nothing to activate and the focus stays empty. A click then
//! brings the window to the front and still gives it no focus: it was active
//! already.
//!
//! Windows sends every key of an active window without focus as a system key,
//! and answers a character that came that way without Alt with the default
//! beep. The client reads those keys like any others, so the game works and
//! beeps. Minimising and restoring the window, or leaving it and coming back,
//! ends it; nothing in the client did. Measured in `docs/client.md`
//! §"Driving the client without its window".

use bevy::ecs::system::NonSendMarker;
use windows_sys::Win32::{
    Foundation::HWND,
    UI::{
        Input::KeyboardAndMouse::{GetActiveWindow, GetFocus, SetFocus},
        WindowsAndMessaging::GetForegroundWindow,
    },
};

/// Each frame, because no event says the focus is missing: the click that
/// brings the window to the front changes neither the active window nor the
/// focus. On the main thread, because the active window and the focus are
/// the calling thread's, and only that thread owns the window.
pub(crate) fn keep_the_keyboard(_main_thread: NonSendMarker) {
    // SAFETY: neither takes an argument or touches memory of ours.
    let (active, foreground) = unsafe { (GetActiveWindow(), GetForegroundWindow()) };
    give_the_keyboard(active, foreground);
}

/// Gives this thread's active window the focus if it stands in front and no
/// window of this thread has any.
///
/// Only in front. Behind another program the window waits, though it could
/// take the focus there: winit reports that as `Focused(true)`, bevy answers
/// with `focus_window`, and winit's way to the front for a window behind
/// others is a made-up Alt press and `SetForegroundWindow`, which took the
/// foreground from the program the player was using. In front that call does
/// nothing.
///
/// Two cases need no check of their own, and a test holds each: Windows gives
/// a minimised window no focus, and with no window on either side this sets
/// an empty focus to empty.
fn give_the_keyboard(active: HWND, foreground: HWND) {
    // SAFETY: neither reads or writes memory of ours. `SetFocus` takes a window
    // handle by value, and one that went stale since it was read makes the
    // call fail, which leaves the state we started from.
    unsafe {
        if active == foreground && GetFocus().is_null() {
            SetFocus(active);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, IsIconic, SHOW_WINDOW_CMD, SW_SHOW, SW_SHOWMINIMIZED,
        ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_CHILD, WS_EX_TOOLWINDOW, WS_POPUP,
        WS_VISIBLE,
    };

    /// A real window of this thread. The tests hand [`give_the_keyboard`] the
    /// foreground themselves, so they do not depend on what stands in front
    /// on the machine running them.
    struct Window(HWND);

    impl Window {
        fn create(ex_style: WINDOW_EX_STYLE, style: WINDOW_STYLE, parent: HWND) -> Self {
            let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            // SAFETY: `class` is a NUL-terminated wide string that outlives the
            // call; a null name, menu and instance are valid, and `parent` is
            // null or a live window of this thread.
            let window = unsafe {
                CreateWindowExW(
                    ex_style,
                    class.as_ptr(),
                    std::ptr::null(),
                    style,
                    -32_000,
                    -32_000,
                    1,
                    1,
                    parent,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null(),
                )
            };
            assert!(!window.is_null(), "the test window was not created");
            Self(window)
        }

        /// Off every screen and off the taskbar, and this thread's active
        /// window once shown.
        fn top_level(how: SHOW_WINDOW_CMD) -> Self {
            let window = Self::create(WS_EX_TOOLWINDOW, WS_POPUP, std::ptr::null_mut());
            // SAFETY: `window` is the live window created above.
            unsafe { ShowWindow(window.0, how) };
            window
        }

        fn child_of(parent: &Self) -> Self {
            Self::create(0, WS_CHILD | WS_VISIBLE, parent.0)
        }

        /// This window, active and with the focus empty: where the client
        /// stood when it beeped.
        fn without_the_focus(how: SHOW_WINDOW_CMD) -> Self {
            let window = Self::top_level(how);
            // SAFETY: a null window is `SetFocus`'s documented way to clear it.
            unsafe { SetFocus(std::ptr::null_mut()) };
            assert_eq!(active(), window.0, "the test needs an active window");
            assert!(focus().is_null(), "the test needs the focus to be empty");
            window
        }
    }

    impl Drop for Window {
        fn drop(&mut self) {
            // SAFETY: the window is this thread's. A parent destroys its
            // children, and destroying a dead handle only fails.
            unsafe { DestroyWindow(self.0) };
        }
    }

    fn active() -> HWND {
        // SAFETY: takes nothing.
        unsafe { GetActiveWindow() }
    }

    fn focus() -> HWND {
        // SAFETY: takes nothing.
        unsafe { GetFocus() }
    }

    /// Red if the repair is taken out.
    #[test]
    fn an_active_window_in_front_gets_the_focus_back() {
        let window = Window::without_the_focus(SW_SHOW);

        give_the_keyboard(window.0, window.0);

        assert_eq!(focus(), window.0);
    }

    /// Behind another program the focus would bring the window to the front
    /// through winit's made-up Alt press. Red if the check on the foreground
    /// goes.
    #[test]
    fn an_active_window_behind_another_program_waits() {
        let window = Window::without_the_focus(SW_SHOW);
        let other = Window::create(WS_EX_TOOLWINDOW, WS_POPUP, std::ptr::null_mut());

        give_the_keyboard(window.0, other.0);
        assert!(focus().is_null());

        give_the_keyboard(window.0, std::ptr::null_mut());
        assert!(focus().is_null());
    }

    /// The focus is left where it is, here on a child. Red if the window takes
    /// it whether or not it is missing.
    #[test]
    fn a_focus_held_is_left_alone() {
        let window = Window::top_level(SW_SHOW);
        let child = Window::child_of(&window);
        // SAFETY: `child` is a live window of this thread.
        unsafe { SetFocus(child.0) };
        assert_eq!(focus(), child.0, "the test needs the child to have it");

        give_the_keyboard(window.0, window.0);

        assert_eq!(focus(), child.0);
    }

    /// What lets [`give_the_keyboard`] go without a check for it: Windows
    /// refuses a minimised window the focus.
    #[test]
    fn a_minimised_window_gets_no_focus() {
        let window = Window::without_the_focus(SW_SHOWMINIMIZED);
        // SAFETY: `window` is the live window of this thread.
        assert_ne!(
            unsafe { IsIconic(window.0) },
            0,
            "the test needs it minimised"
        );

        give_the_keyboard(window.0, window.0);

        assert!(focus().is_null());
    }

    /// With no active window there is nothing to give the focus to, and a
    /// foreground of null is what Windows reports between two windows.
    #[test]
    fn without_an_active_window_nothing_happens() {
        assert!(
            active().is_null(),
            "the test needs a thread without windows"
        );

        give_the_keyboard(std::ptr::null_mut(), std::ptr::null_mut());

        assert!(focus().is_null());
    }
}
