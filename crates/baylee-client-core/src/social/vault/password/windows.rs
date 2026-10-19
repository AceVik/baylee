//! Windows private DACLs: only the process user's SID and `LocalSystem`.
//! Directories are created with their descriptor, never chmodded afterward.
//! Existing owners and ACLs are checked, including inherited file ACEs.

use super::Error;
use std::ffi::c_void;
use std::os::windows::{
    ffi::OsStrExt as _,
    io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle},
};
use std::path::Path;
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{
        Authorization::{
            ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
            ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW,
            SDDL_REVISION_1, SE_FILE_OBJECT,
        },
        DACL_SECURITY_INFORMATION, GetTokenInformation, OWNER_SECURITY_INFORMATION, PSID,
        SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
    },
    Storage::FileSystem::{
        CreateDirectoryW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

/// Allocation owned by the Windows `LocalAlloc` family.
struct Local(*mut c_void);
impl Drop for Local {
    fn drop(&mut self) {
        // SAFETY: each Local owns exactly one successful Win32 allocation;
        // references into it never outlive this value, and it is freed once.
        unsafe {
            LocalFree(self.0);
        }
    }
}

fn wide(path: &Path) -> Result<Vec<u16>, Error> {
    let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
    if value.contains(&0) {
        return Err(Error::Io);
    }
    value.push(0);
    Ok(value)
}

fn current_sid() -> Result<String, Error> {
    let mut token = null_mut();
    // SAFETY: current-process pseudo handle is valid; token is an initialized
    // output pointer. On success ownership transfers into OwnedHandle.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) } == 0 {
        return Err(Error::Io);
    }
    // SAFETY: OpenProcessToken succeeded, returning one owned, valid handle.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    // usize storage supplies TOKEN_USER's pointer alignment; this exceeds
    // TOKEN_USER plus SECURITY_MAX_SID_SIZE, and the API gets the actual size.
    let mut buffer = [0usize; 128];
    let mut needed = 0;
    // SAFETY: buffer is aligned, writable for the supplied size, and live until
    // SID conversion completes. The API validates the output length.
    if unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            u32::try_from(std::mem::size_of_val(&buffer)).map_err(|_| Error::Io)?,
            &raw mut needed,
        )
    } == 0
        || needed as usize > std::mem::size_of_val(&buffer)
        || (needed as usize) < std::mem::size_of::<TOKEN_USER>()
    {
        return Err(Error::Io);
    }
    // SAFETY: successful TokenUser query initialized this aligned TOKEN_USER;
    // its SID points into the buffer, which remains live during conversion.
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    sid_text(sid)
}

fn sid_text(sid: PSID) -> Result<String, Error> {
    let mut text = null_mut();
    // SAFETY: callers provide a SID returned by a successful security API and
    // keep its owner alive; text receives a LocalAlloc-owned NUL-terminated string.
    if unsafe { ConvertSidToStringSidW(sid, &raw mut text) } == 0 {
        return Err(Error::Io);
    }
    let allocated = Local(text.cast());
    let value = local_text(&allocated);
    Ok(value)
}

fn local_text(text: &Local) -> String {
    let ptr = text.0.cast::<u16>();
    let mut length = 0;
    // SAFETY: Local contains a live, NUL-terminated UTF-16 string returned by
    // Convert*ToString*W. The terminator is within that allocation.
    unsafe {
        while *ptr.add(length) != 0 {
            length += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr, length))
    }
}

fn descriptor(sddl: &str) -> Result<Local, Error> {
    let text: Vec<u16> = sddl.encode_utf16().chain([0]).collect();
    let mut descriptor = null_mut();
    // SAFETY: text is terminated and live; descriptor is a writable output
    // pointer. On success the allocation is owned by Local and freed once.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text.as_ptr(),
            SDDL_REVISION_1,
            &raw mut descriptor,
            null_mut(),
        )
    } == 0
    {
        return Err(Error::Io);
    }
    Ok(Local(descriptor))
}

pub(super) fn create_dir(dir: &Path) -> Result<(), Error> {
    let sid = current_sid()?;
    let sd = descriptor(&format!("D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;SY)"))?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(std::mem::size_of::<SECURITY_ATTRIBUTES>())
            .map_err(|_| Error::Io)?,
        lpSecurityDescriptor: sd.0,
        bInheritHandle: 0,
    };
    // Create missing ancestors from top to bottom, each with a protected ACL.
    let mut missing = Vec::new();
    for path in dir.ancestors() {
        if path.try_exists().map_err(|_| Error::Io)? {
            break;
        }
        missing.push(path);
    }
    for path in missing.into_iter().rev() {
        let name = wide(path)?;
        // SAFETY: path and SECURITY_ATTRIBUTES/descriptor are valid and remain
        // live throughout the synchronous call. No handle ownership escapes.
        if unsafe { CreateDirectoryW(name.as_ptr(), &raw const attributes) } == 0 && !path.is_dir()
        {
            return Err(Error::Io);
        }
        check(path)?;
    }
    check(dir)
}

pub(super) fn check(path: &Path) -> Result<(), Error> {
    use std::os::windows::fs::MetadataExt as _;
    let meta = std::fs::symlink_metadata(path).map_err(|_| Error::Io)?;
    if meta.file_attributes() & 0x400 != 0 {
        return Err(Error::Io);
    } // FILE_ATTRIBUTE_REPARSE_POINT
    let name = wide(path)?;
    let mut owner = null_mut();
    let mut sd = null_mut();
    // SAFETY: name is terminated; outputs are initialized writable pointers.
    // Successful output descriptor owns owner SID's backing storage.
    if unsafe {
        GetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &raw mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &raw mut sd,
        )
    } != 0
    {
        return Err(Error::Io);
    }
    let sd = Local(sd);
    let sid = current_sid()?;
    if sid_text(owner)? != sid {
        return Err(Error::Io);
    }
    let mut text = null_mut();
    // SAFETY: sd owns a valid descriptor from GetNamedSecurityInfoW; the output
    // string is a new LocalAlloc allocation, independently owned below.
    if unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            sd.0,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &raw mut text,
            null_mut(),
        )
    } == 0
    {
        return Err(Error::Io);
    }
    let text = local_text(&Local(text.cast()));
    if !private_dacl(&text, &sid) {
        return Err(Error::Io);
    }
    Ok(())
}

fn private_dacl(text: &str, sid: &str) -> bool {
    // AI records automatic propagation, not whether an ACE is inherited (ID).
    // Windows also returns inherited ACEs without AI. Both representations
    // must still contain exactly our two full-access allows, in either order.
    // See Microsoft's "Security Descriptor String Format" and "ACE Strings".
    let Some((control, entries)) = text.split_once('(') else {
        return false;
    };
    let flags: &[&str] = match control {
        "D:P" | "D:PAI" => &["OICI"],
        "D:" | "D:AI" => &["ID", "OICIID"],
        _ => return false,
    };
    flags.iter().any(|flags| {
        entries == format!("A;{flags};FA;;;{sid})(A;{flags};FA;;;SY)")
            || entries == format!("A;{flags};FA;;;SY)(A;{flags};FA;;;{sid})")
    })
}

pub(super) fn replace(from: &Path, to: &Path) -> Result<(), Error> {
    let from = wide(from)?;
    let to = wide(to)?;
    // SAFETY: both terminated path buffers remain live for the call. The
    // source file is closed and synced; WRITE_THROUGH waits for the move.
    if unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(Error::Io);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Security::{
        Authorization::SetNamedSecurityInfoW, GetSecurityDescriptorDacl,
        PROTECTED_DACL_SECURITY_INFORMATION,
    };
    #[test]
    fn inherited_flags_and_order_do_not_relax_the_private_allow_list() {
        let sid = "S-1-5-21-1-2-3-1001";
        for (control, flags) in [
            ("P", "OICI"),
            ("PAI", "OICI"),
            ("", "ID"),
            ("AI", "ID"),
            ("", "OICIID"),
            ("AI", "OICIID"),
        ] {
            let owner = format!("(A;{flags};FA;;;{sid})");
            let system = format!("(A;{flags};FA;;;SY)");
            for body in [format!("{owner}{system}"), format!("{system}{owner}")] {
                let valid = format!("D:{control}{body}");
                assert!(private_dacl(&valid, sid));
                assert!(!private_dacl(&valid.replace(";;;SY", ";;;WD"), sid));
                assert!(!private_dacl(&format!("{valid}(A;;FA;;;WD)"), sid));
                assert!(!private_dacl(&valid.replace(";FA;", ";FR;"), sid));
                assert!(!private_dacl(&valid.replace("(A;", "(D;"), sid));
                assert!(!private_dacl(&valid, "S-1-5-21-4-5-6-1001"));
            }
            assert!(!private_dacl(&format!("D:{control}{owner}{owner}"), sid));
            assert!(!private_dacl(&format!("D:{control}{owner}"), sid));
        }
        for invalid in ["", "D:", "D:NO_ACCESS_CONTROL", "D:(A;;FA;;;WD)"] {
            assert!(!private_dacl(invalid, sid));
        }
    }

    #[test]
    fn private_descriptor_roundtrip_inheritance_and_public_acl_refusal() {
        let root = std::env::temp_dir().join(format!("baylee-acl-{}", uuid::Uuid::now_v7()));
        create_dir(&root).unwrap();
        check(&root).unwrap();
        let file = root.join("test");
        std::fs::write(&file, b"ciphertext").unwrap();
        check(&file).unwrap();
        let nested = root.join("inherited");
        std::fs::create_dir(&nested).unwrap();
        check(&nested).unwrap();
        let replacement = root.join("replacement");
        std::fs::write(&replacement, b"new ciphertext").unwrap();
        replace(&replacement, &file).unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"new ciphertext");
        assert!(!replacement.exists());
        check(&file).unwrap();
        assert!(wide(Path::new("bad\0path")).is_err());
        let public = descriptor("D:P(A;;FA;;;WD)").unwrap();
        let mut present = 0;
        let mut defaulted = 0;
        let mut dacl = null_mut();
        // SAFETY: descriptor is valid; output pointers live through the call.
        assert_ne!(
            unsafe {
                GetSecurityDescriptorDacl(
                    public.0,
                    &raw mut present,
                    &raw mut dacl,
                    &raw mut defaulted,
                )
            },
            0
        );
        let name = wide(&file).unwrap();
        // SAFETY: name is terminated and dacl points into the still-live public
        // descriptor. The test changes only its own temporary file's ACL.
        assert_eq!(
            unsafe {
                SetNamedSecurityInfoW(
                    name.as_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                    null_mut(),
                    null_mut(),
                    dacl,
                    std::ptr::null(),
                )
            },
            0
        );
        assert!(check(&file).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
