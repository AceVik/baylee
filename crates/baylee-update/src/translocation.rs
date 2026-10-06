//! macOS App Translocation: where a quarantined app really is.
//!
//! macOS starts a downloaded (quarantined) app that was never moved by the
//! Finder from a read-only, randomised mount under `…/AppTranslocation/<id>/`
//! (Gatekeeper's path randomisation). The running program then sees that
//! mount, not the folder the player unpacked it into, and the mount's name
//! is new each time the app is registered. The launcher keys its per-user
//! state by the **original** bundle (`launch::in_state_root_of`), so an
//! update it installs is still selected at the next start.
//!
//! The Security framework answers both questions
//! (`SecTranslocateIsTranslocatedURL`, `SecTranslocateCreateOriginalPathForURL`,
//! in `<Security/SecTranslocate.h>` since macOS 10.12). They are looked up
//! with `dlsym` rather than linked, so a system without them still starts:
//! then a path with `/AppTranslocation/` in it counts as translocated with
//! no known original, and installing stays off there as before.
//!
//! Callers take a [`Translocation`], so every decision built on it is
//! tested with [`Fixed`] on any system; [`System`] is the real one.

use std::path::{Path, PathBuf};

/// What macOS says about one bundle path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Running where it lies.
    InPlace,
    /// Running from a translocation mount of the bundle at this path.
    From(PathBuf),
    /// Running from a translocation mount; where the original is, unknown.
    Unknown,
}

/// Asks where a bundle really is.
pub trait Translocation {
    /// The status of `bundle` (a `.app` directory).
    fn status(&self, bundle: &Path) -> Status;
}

/// The answer a path gives without asking the system: a mount under
/// `/AppTranslocation/` is translocated, from an unknown original.
#[must_use]
pub fn by_name(bundle: &Path) -> Status {
    if bundle.to_string_lossy().contains("/AppTranslocation/") {
        Status::Unknown
    } else {
        Status::InPlace
    }
}

/// The running system's answer: the Security framework on macOS,
/// [`by_name`] where that is missing, and always [`Status::InPlace`] on
/// other systems.
#[derive(Clone, Copy, Debug, Default)]
pub struct System;

impl Translocation for System {
    fn status(&self, bundle: &Path) -> Status {
        #[cfg(target_os = "macos")]
        if let Some(api) = ffi::api() {
            return api.status(bundle).unwrap_or_else(|| by_name(bundle));
        }
        if cfg!(target_os = "macos") {
            by_name(bundle)
        } else {
            Status::InPlace
        }
    }
}

/// A fixed answer, for tests: what [`System`] would say on a Mac.
#[derive(Clone, Debug)]
pub struct Fixed(pub Status);

impl Translocation for Fixed {
    fn status(&self, _bundle: &Path) -> Status {
        self.0.clone()
    }
}

#[cfg(target_os = "macos")]
mod ffi {
    //! The two Security functions, found at run time.
    //!
    //! `dlopen` keeps the framework loaded for the life of the process (it is
    //! never closed), so the function pointers stay valid as long as the
    //! [`Api`] that holds them, which lives in a `static`.

    use super::Status;
    use std::ffi::{CStr, c_char, c_int, c_void};
    use std::os::unix::ffi::OsStrExt as _;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    type CfRef = *const c_void;
    type IsTranslocated = unsafe extern "C" fn(CfRef, *mut bool, *mut CfRef) -> u8;
    type CreateOriginal = unsafe extern "C" fn(CfRef, *mut CfRef) -> CfRef;

    const SECURITY: &CStr = c"/System/Library/Frameworks/Security.framework/Security";
    const RTLD_LAZY: c_int = 0x1;
    const RTLD_LOCAL: c_int = 0x4;
    /// `PATH_MAX` is 1024 on macOS; room to spare for a longer answer.
    const PATH_BYTES: usize = 4096;

    #[allow(unsafe_code)] // declarations of libSystem/CoreFoundation C functions
    unsafe extern "C" {
        fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    #[allow(unsafe_code)] // declarations of libSystem/CoreFoundation C functions
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFURLCreateFromFileSystemRepresentation(
            allocator: CfRef,
            buffer: *const u8,
            length: isize,
            is_directory: u8,
        ) -> CfRef;
        fn CFURLGetFileSystemRepresentation(
            url: CfRef,
            resolve_against_base: u8,
            buffer: *mut u8,
            max_length: isize,
        ) -> u8;
        fn CFRelease(object: CfRef);
    }

    pub(super) struct Api {
        is_translocated: IsTranslocated,
        create_original: CreateOriginal,
    }

    /// The functions, or `None` on a system without them.
    pub(super) fn api() -> Option<&'static Api> {
        static API: OnceLock<Option<Api>> = OnceLock::new();
        API.get_or_init(load).as_ref()
    }

    #[allow(unsafe_code)] // dlopen/dlsym: no safe binding without a new dependency.
    fn load() -> Option<Api> {
        // SAFETY: both arguments are NUL-terminated C strings that live for
        // the call; dlopen returns null on failure, which is checked.
        let handle = unsafe { dlopen(SECURITY.as_ptr(), RTLD_LAZY | RTLD_LOCAL) };
        if handle.is_null() {
            return None;
        }
        // SAFETY: `handle` is a live dlopen handle (never closed) and the
        // names are NUL-terminated; a null answer is checked.
        let is = unsafe { dlsym(handle, c"SecTranslocateIsTranslocatedURL".as_ptr()) };
        // SAFETY: as above.
        let create = unsafe { dlsym(handle, c"SecTranslocateCreateOriginalPathForURL".as_ptr()) };
        if is.is_null() || create.is_null() {
            return None;
        }
        // SAFETY: the symbols are the C functions SecTranslocate.h declares:
        //   Boolean SecTranslocateIsTranslocatedURL(CFURLRef, bool *, CFErrorRef *);
        //   CFURLRef SecTranslocateCreateOriginalPathForURL(CFURLRef, CFErrorRef *);
        // `Boolean` is `unsigned char` (u8) and C `bool` is one byte, as is
        // Rust's. The types above spell exactly those signatures, and the
        // pointers stay valid because the library is never unloaded.
        // `the_binding_round_trips_a_real_folder` calls both.
        unsafe {
            Some(Api {
                is_translocated: std::mem::transmute::<*mut c_void, IsTranslocated>(is),
                create_original: std::mem::transmute::<*mut c_void, CreateOriginal>(create),
            })
        }
    }

    /// An owned CoreFoundation object, released once.
    struct Owned(CfRef);

    impl Drop for Owned {
        #[allow(unsafe_code)]
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: `Owned` is only built from a +1 (Create rule)
                // reference or an out-error the callee retained for us, and
                // is released exactly once, here.
                unsafe { CFRelease(self.0) };
            }
        }
    }

    #[allow(unsafe_code)]
    fn url(path: &Path) -> Option<Owned> {
        let bytes = path.as_os_str().as_bytes();
        let length = isize::try_from(bytes.len()).ok()?;
        // SAFETY: `bytes` is valid for `length` bytes during the call; the
        // allocator null means the default one; the result follows the
        // Create rule (+1) and is owned by `Owned`, or is null (checked).
        let url = unsafe {
            CFURLCreateFromFileSystemRepresentation(std::ptr::null(), bytes.as_ptr(), length, 1)
        };
        (!url.is_null()).then_some(Owned(url))
    }

    #[allow(unsafe_code)]
    fn path_of(url: &Owned) -> Option<PathBuf> {
        let mut buffer = vec![0u8; PATH_BYTES];
        let length = isize::try_from(buffer.len()).ok()?;
        // SAFETY: `url.0` is a live CFURL; `buffer` is writable for `length`
        // bytes and CF writes a NUL-terminated path no longer than that,
        // answering false (checked) when it does not fit.
        let ok = unsafe { CFURLGetFileSystemRepresentation(url.0, 1, buffer.as_mut_ptr(), length) };
        if ok == 0 {
            return None;
        }
        let end = buffer.iter().position(|b| *b == 0)?;
        buffer.truncate(end);
        Some(PathBuf::from(std::ffi::OsString::from_vec(buffer)))
    }

    use std::os::unix::ffi::OsStringExt as _;

    impl Api {
        /// `None` when the system could not answer.
        #[allow(unsafe_code)]
        pub(super) fn status(&self, bundle: &Path) -> Option<Status> {
            let url = url(bundle)?;
            let mut translocated = false;
            let mut error: CfRef = std::ptr::null();
            // SAFETY: `url.0` is a live CFURL, `translocated` a writable
            // bool and `error` a writable CFErrorRef slot (left null or set
            // to a +1 error we release).
            let ok =
                unsafe { (self.is_translocated)(url.0, &raw mut translocated, &raw mut error) };
            drop(Owned(error));
            if ok == 0 {
                return None;
            }
            if !translocated {
                return Some(Status::InPlace);
            }
            let mut error: CfRef = std::ptr::null();
            // SAFETY: as above; the answer follows the Create rule (+1) and
            // is owned by `Owned`, or is null (checked).
            let original = Owned(unsafe { (self.create_original)(url.0, &raw mut error) });
            drop(Owned(error));
            if original.0.is_null() {
                return Some(Status::Unknown);
            }
            Some(path_of(&original).map_or(Status::Unknown, Status::From))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The binding round-trips a real folder: the framework is found,
        /// a CFURL is made and read back, and a folder nobody translocated
        /// is in place. A wrong signature or a wrong release crashes here.
        #[test]
        fn the_binding_round_trips_a_real_folder() {
            let api =
                api().expect("Security.framework has SecTranslocate on every supported macOS");
            let dir = std::env::temp_dir().join(format!("baylee-transloc-{}", std::process::id()));
            let bundle = dir.join("Baylee Ü.app");
            std::fs::create_dir_all(&bundle).unwrap();
            assert_eq!(api.status(&bundle), Some(Status::InPlace));
            let url = url(&bundle).unwrap();
            let back = path_of(&url).unwrap();
            assert_eq!(
                std::fs::canonicalize(&back).unwrap(),
                std::fs::canonicalize(&bundle).unwrap(),
                "a non-ASCII path survives the CFURL round trip"
            );
            // Asking about a path that does not exist answers, not crashes.
            let _ = api.status(&dir.join("nothing.app"));
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_translocation_mount_is_known_by_its_name() {
        let mount = Path::new("/private/var/folders/xy/T/AppTranslocation/1234-5678/d/Baylee.app");
        assert_eq!(by_name(mount), Status::Unknown);
        assert_eq!(
            by_name(Path::new("/Applications/Baylee.app")),
            Status::InPlace
        );
        assert_eq!(
            by_name(Path::new(
                "/Users/p/Downloads/AppTranslocation-notes/Baylee.app"
            )),
            Status::InPlace,
            "only a whole path component counts"
        );
    }

    #[test]
    fn the_system_answers_in_place_for_an_ordinary_folder() {
        let dir = std::env::temp_dir();
        assert_eq!(System.status(&dir), Status::InPlace);
    }
}
