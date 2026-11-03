//! Which screen a test client opens on.
//!
//! A test window must not land where a person is working. The owner's rule
//! (`.claude/skills/dev-control/SKILL.md`): on the owner's laptop the **built-in
//! display**, whose native index changes with the screens attached, and on
//! the Windows workstation the screen left of the primary one, named by index.
//! So the default on macOS asks CoreGraphics which display is built in, and
//! the launch checks afterwards that `/monitors` agrees before any test runs.
use anyhow::{Result, bail};

/// The screen a client is asked to open on (`BAYLEE_DEV_MONITOR`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Monitor {
    /// The platform's choice: whatever screen the window manager picks.
    PlatformDefault,
    /// The primary screen.
    Primary,
    /// A native monitor index, as `GET /monitors` lists them.
    Index(u32),
    /// The machine's built-in display (macOS only).
    BuiltIn,
}

impl Monitor {
    /// What `BAYLEE_E2E_MONITOR` asks for: `builtin`, `primary`, `default` or
    /// an index. Unset, macOS takes the built-in display when there is one and
    /// every other platform the window manager's choice.
    pub fn from_env() -> Result<Self> {
        match std::env::var("BAYLEE_E2E_MONITOR") {
            Ok(value) => Self::parse(&value),
            Err(_) if built_in().is_some() => Ok(Self::BuiltIn),
            Err(_) => Ok(Self::PlatformDefault),
        }
    }

    /// Reads one spelling of a monitor.
    pub fn parse(value: &str) -> Result<Self> {
        Ok(match value.trim() {
            "" | "default" => Self::PlatformDefault,
            "primary" => Self::Primary,
            "builtin" | "built-in" => Self::BuiltIn,
            index => match index.parse() {
                Ok(index) => Self::Index(index),
                Err(_) => bail!(
                    "BAYLEE_E2E_MONITOR={index:?}: expected builtin, primary, default or an index"
                ),
            },
        })
    }

    /// The `BAYLEE_DEV_MONITOR` value the client takes, with the built-in
    /// display's physical size to check `/monitors` against.
    pub(crate) fn resolve(self) -> Result<(Option<String>, Option<BuiltIn>)> {
        Ok(match self {
            Self::PlatformDefault => (None, None),
            Self::Primary => (Some("primary".to_owned()), None),
            Self::Index(index) => (Some(index.to_string()), None),
            Self::BuiltIn => match built_in() {
                Some(display) => (Some(display.index.to_string()), Some(display)),
                None => bail!("this machine has no built-in display; set BAYLEE_E2E_MONITOR"),
            },
        })
    }
}

/// The built-in display: its native index and its size in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BuiltIn {
    pub index: u32,
    pub width: u32,
    pub height: u32,
}

/// winit lists macOS screens in `CGGetActiveDisplayList`'s order, which is
/// the order `MonitorSelection::Index` counts in.
#[cfg(target_os = "macos")]
fn built_in() -> Option<BuiltIn> {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
        fn CGDisplayIsBuiltin(display: u32) -> u32;
        fn CGDisplayCopyDisplayMode(display: u32) -> *mut std::ffi::c_void;
        fn CGDisplayModeGetPixelWidth(mode: *mut std::ffi::c_void) -> usize;
        fn CGDisplayModeGetPixelHeight(mode: *mut std::ffi::c_void) -> usize;
        fn CGDisplayModeRelease(mode: *mut std::ffi::c_void);
    }
    let mut displays = [0_u32; 16];
    let mut count = 0_u32;
    // SAFETY: the list has room for the 16 ids the call is told it may
    // write, and `count` is a valid place for the number it wrote.
    let status = unsafe { CGGetActiveDisplayList(16, displays.as_mut_ptr(), &raw mut count) };
    if status != 0 {
        return None;
    }
    let listed = displays.get(..count as usize)?;
    let index = listed
        .iter()
        // SAFETY: each id was just handed out by CoreGraphics itself.
        .position(|&display| unsafe { CGDisplayIsBuiltin(display) } != 0)?;
    let display = listed[index];
    // SAFETY: a live display id; the mode is released once read, and a
    // null mode (a display that went away in between) is never read.
    let (width, height) = unsafe {
        let mode = CGDisplayCopyDisplayMode(display);
        if mode.is_null() {
            return None;
        }
        let size = (
            CGDisplayModeGetPixelWidth(mode),
            CGDisplayModeGetPixelHeight(mode),
        );
        CGDisplayModeRelease(mode);
        size
    };
    Some(BuiltIn {
        index: index as u32,
        width: width as u32,
        height: height as u32,
    })
}

#[cfg(not(target_os = "macos"))]
fn built_in() -> Option<BuiltIn> {
    None
}

#[cfg(test)]
mod tests {
    use super::Monitor;

    #[test]
    fn every_spelling_of_a_monitor_is_read_and_a_wrong_one_refused() {
        assert_eq!(Monitor::parse("builtin").unwrap(), Monitor::BuiltIn);
        assert_eq!(Monitor::parse("primary").unwrap(), Monitor::Primary);
        assert_eq!(Monitor::parse("").unwrap(), Monitor::PlatformDefault);
        assert_eq!(Monitor::parse(" 2 ").unwrap(), Monitor::Index(2));
        assert!(Monitor::parse("left").is_err());
    }
}
