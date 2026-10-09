//! The console a Windows release build does not open, unless asked.
//!
//! `main.rs` builds the runtime as a windows-subsystem program in release
//! builds (the owner, 08.10.2026): a player starting Baylee, directly or
//! through the launcher, gets the game's window and no console beside it.
//! A debug build stays a console program, so `cargo run` keeps its log in
//! the terminal it was run from.
//!
//! `--console` (`baylee-client.exe --console`; the launcher passes its
//! arguments on) asks for the log anyway: the client attaches to the console
//! it was started from, or opens a console of its own when there is none,
//! and points its standard handles at it — only those it was not handed, so
//! a log redirected to a file stays in the file. Everywhere else the
//! argument is accepted and does nothing: a terminal already has the log.

use std::ffi::OsString;

/// The argument that asks for a console.
pub const CONSOLE_ARG: &str = "--console";

/// Whether these arguments (the program's, without its own name) ask for a
/// console.
#[must_use]
pub fn asked(args: impl IntoIterator<Item = OsString>) -> bool {
    args.into_iter().any(|arg| arg == CONSOLE_ARG)
}

/// Opens the console when this process was asked for one. First, before
/// anything is logged: what is written before has nowhere to go.
pub fn open_if_asked() {
    #[cfg(windows)]
    if asked(std::env::args_os().skip(1)) {
        attach();
    }
}

/// Attaches to the parent's console, else opens one, and gives the process
/// the console's handles where it holds none.
///
/// A program of the windows subsystem starts with no standard handles, and
/// attaching to a console does not hand it the console's: they are opened by
/// name (`CONOUT$`, `CONIN$`). Rust's standard streams ask for the handle on
/// every write, so setting it here is enough for `println!`, the log and the
/// panic message alike. In a debug build, a console program already, both
/// calls fail and every handle is already there: nothing changes.
#[cfg(windows)]
#[allow(unsafe_code)] // Win32 console calls, no safe binding exists.
fn attach() {
    use std::os::windows::io::IntoRawHandle as _;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AllocConsole, AttachConsole, GetStdHandle, STD_ERROR_HANDLE,
        STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
    };
    // SAFETY: plain Win32 calls with no pointers; a failure is a return
    // value, and a process that already has a console keeps it.
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            AllocConsole();
        }
    }
    for (which, name, write) in [
        (STD_OUTPUT_HANDLE, "CONOUT$", true),
        (STD_ERROR_HANDLE, "CONOUT$", true),
        (STD_INPUT_HANDLE, "CONIN$", false),
    ] {
        // SAFETY: as above.
        let held = unsafe { GetStdHandle(which) };
        if held != 0 && held != INVALID_HANDLE_VALUE {
            continue;
        }
        let Ok(file) = std::fs::OpenOptions::new()
            .read(!write)
            .write(write)
            .open(name)
        else {
            continue;
        };
        // The handle is the process's from here on, as a standard handle is:
        // never closed.
        let handle = file.into_raw_handle() as isize;
        // SAFETY: `handle` is an open console handle this process owns.
        unsafe {
            SetStdHandle(which, handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only the exact argument asks, wherever it stands.
    #[test]
    fn only_console_asks_for_a_console() {
        let args = |list: &[&str]| list.iter().map(OsString::from).collect::<Vec<_>>();
        assert!(asked(args(&["--console"])));
        assert!(asked(args(&["--other", "--console"])));
        assert!(!asked(args(&[])));
        assert!(!asked(args(&["--consoles", "console", "-console"])));
    }

    /// A Windows release build is a windows-subsystem program and a debug
    /// build a console one: the attribute in `main.rs` says both, and this
    /// fails if it is dropped or loses its debug half.
    #[test]
    fn a_release_build_opens_no_console_on_windows() {
        let main = include_str!("main.rs");
        assert!(
            main.contains(
                r#"#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]"#
            ),
            "main.rs no longer hides the console of a Windows release build"
        );
    }
}
