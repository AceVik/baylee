//! The running client's door for `baylee:` links (`docs/client.md`
//! §"Links from the website").
//!
//! Windows and Linux hand an opened link to the program they registered for
//! the scheme, which is the launcher, as its argument. When no client runs,
//! the launcher passes it on to the client it starts, as it passes every
//! argument. When one runs, the launcher meets the lifetime lock and would
//! refuse as a second instance; with a link it knocks on this door instead,
//! the running client takes the link, and no second window opens. macOS
//! sends a link to the running client itself (an Apple Event), so there the
//! door serves a client started straight from a terminal or by `cargo run`.
//!
//! One door per installation: a Unix socket in a `0700` directory of the
//! per-user state root, or a named pipe on Windows that refuses remote
//! clients, named by a hash of the installation's state directory, which the
//! launcher and the client it started share. A client started without a
//! launcher names its own ([`Door::keyed`]).
//!
//! **The frame is frozen**, because the launcher is never replaced by an
//! update (`docs/releasing.md` §"Desktop launcher and recovery"):
//!
//! | Bytes | What |
//! | --- | --- |
//! | 4 | `BLK1` |
//! | 2 | the link's length, big-endian, at most [`MAX_LINK`] |
//! | n | the link, UTF-8 |
//! | 2 | the activation token's length, big-endian, at most [`MAX_TOKEN`]; 0 for none |
//! | n | the token, UTF-8 |
//!
//! The client answers one byte: [`TAKEN`], or anything else for refused, and
//! closes. A link is a stranger's words and never a secret: the client reads
//! it strictly (`baylee_client_core::deeplink`) and asks before anything it
//! asks for happens. The activation token is the one Wayland hands a new
//! process (`XDG_ACTIVATION_TOKEN`), carried so a later client can raise its
//! window with it.

use sha2::{Digest as _, Sha256};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// The longest link a knock carries.
pub const MAX_LINK: usize = 2048;
/// The longest activation token a knock carries.
pub const MAX_TOKEN: usize = 512;
/// The client's answer when it took the link.
pub const TAKEN: u8 = 1;
const REFUSED: u8 = 0;
const MAGIC: &[u8; 4] = b"BLK1";
/// How long the client waits for a knock's frame before it hangs up.
const FRAME_WAIT: Duration = Duration::from_secs(2);
/// How long a knock waits between tries while no door answers yet.
const RETRY: Duration = Duration::from_millis(100);

/// What a knock carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Knock {
    /// The link, as the system handed it over.
    pub link: String,
    /// Wayland's activation token for the new process, if it had one.
    pub token: Option<String>,
}

/// The first `baylee:` link among a program's arguments, if any. The scheme
/// is matched in any case; whether the rest is a link the client reads is
/// the client's question.
#[must_use]
pub fn among(args: &[std::ffi::OsString]) -> Option<String> {
    args.iter()
        .filter_map(|arg| arg.to_str())
        .find(|arg| {
            arg.get(..7)
                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("baylee:"))
        })
        .map(str::to_string)
}

/// Where one client listens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Door {
    #[cfg(unix)]
    path: std::path::PathBuf,
    #[cfg(windows)]
    name: Vec<u16>,
}

impl Door {
    /// The door of the installation whose state directory is `stage`, in
    /// this user's state root.
    ///
    /// # Errors
    /// No per-user state directory.
    pub fn of_stage(stage: &Path) -> io::Result<Self> {
        Ok(Self::keyed(
            &crate::launch::state_root()?,
            stage.as_os_str().as_encoded_bytes(),
        ))
    }

    /// The door of a client started without a launcher (a terminal, `cargo
    /// run`), named by what tells it apart from another: its program and its
    /// settings.
    ///
    /// # Errors
    /// No per-user state directory.
    pub fn direct(key: &[u8]) -> io::Result<Self> {
        Ok(Self::keyed(
            &crate::launch::state_root()?,
            &[b"direct\0".as_slice(), key].concat(),
        ))
    }

    /// A door named by `key` under `root`: the installation's state
    /// directory for a launched client, or whatever tells a directly started
    /// one apart from another (its program and settings).
    #[must_use]
    pub fn keyed(root: &Path, key: &[u8]) -> Self {
        let digest = Sha256::digest(key);
        let name = crate::sign::hex_of(&digest[..8]);
        #[cfg(unix)]
        {
            let _ = root;
            Self {
                path: root
                    .join("baylee")
                    .join("links")
                    .join(format!("{name}.sock")),
            }
        }
        #[cfg(windows)]
        {
            let _ = root;
            Self {
                name: format!(r"\\.\pipe\baylee-link-{name}")
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect(),
            }
        }
    }

    /// Hands a link to the client behind this door, trying until `wait` has
    /// passed while none answers yet (the client may be starting). Answers
    /// whether it took the link.
    ///
    /// # Errors
    /// No client answered within `wait`, or the link or token is too long.
    pub fn knock(&self, knock: &Knock, wait: Duration) -> io::Result<bool> {
        let frame = encode(knock)?;
        let door = self.clone();
        let (sent, answer) = mpsc::channel();
        // A thread, so a client that never answers costs the caller `wait`
        // and no more, whatever the platform's I/O can be told to time out.
        std::thread::Builder::new()
            .name("baylee-link-knock".into())
            .spawn(move || {
                let _ = sent.send(door.knock_until(&frame, Instant::now() + wait));
            })?;
        answer
            .recv_timeout(wait + FRAME_WAIT)
            .unwrap_or_else(|_| Err(io::Error::from(io::ErrorKind::TimedOut)))
    }

    fn knock_until(&self, frame: &[u8], deadline: Instant) -> io::Result<bool> {
        loop {
            match self.connect() {
                Ok(mut stream) => {
                    stream.write_all(frame)?;
                    let mut answer = [REFUSED];
                    stream.read_exact(&mut answer)?;
                    return Ok(answer[0] == TAKEN);
                }
                Err(err) if Instant::now() >= deadline => return Err(err),
                Err(_) => std::thread::sleep(RETRY),
            }
        }
    }
}

fn encode(knock: &Knock) -> io::Result<Vec<u8>> {
    let token = knock.token.as_deref().unwrap_or("");
    if knock.link.len() > MAX_LINK || token.len() > MAX_TOKEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a link or token too long to hand over",
        ));
    }
    let mut frame = Vec::with_capacity(8 + knock.link.len() + token.len());
    frame.extend_from_slice(MAGIC);
    for part in [knock.link.as_bytes(), token.as_bytes()] {
        let len = u16::try_from(part.len()).map_err(io::Error::other)?;
        frame.extend_from_slice(&len.to_be_bytes());
        frame.extend_from_slice(part);
    }
    Ok(frame)
}

/// Reads one frame, or `None` for anything that is not exactly one.
fn decode(stream: &mut impl Read) -> Option<Knock> {
    let mut magic = [0; 4];
    stream.read_exact(&mut magic).ok()?;
    if &magic != MAGIC {
        return None;
    }
    let link = part(stream, MAX_LINK)?;
    let token = part(stream, MAX_TOKEN)?;
    Some(Knock {
        link,
        token: (!token.is_empty()).then_some(token),
    })
}

fn part(stream: &mut impl Read, max: usize) -> Option<String> {
    let mut len = [0; 2];
    stream.read_exact(&mut len).ok()?;
    let len = usize::from(u16::from_be_bytes(len));
    if len > max {
        return None;
    }
    let mut bytes = vec![0; len];
    stream.read_exact(&mut bytes).ok()?;
    String::from_utf8(bytes).ok()
}

/// Answers one knock: delivers its link and says so, or refuses a frame
/// that is not one.
fn answer(stream: &mut (impl Read + Write), deliver: &(impl Fn(Knock) + ?Sized)) {
    let reply = match decode(stream) {
        Some(knock) => {
            deliver(knock);
            TAKEN
        }
        None => REFUSED,
    };
    let _ = stream.write_all(&[reply]);
    let _ = stream.flush();
}

#[cfg(unix)]
mod unix {
    use super::{Door, FRAME_WAIT, Knock, answer};
    use std::io;
    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;

    impl Door {
        pub(super) fn connect(&self) -> io::Result<UnixStream> {
            let stream = UnixStream::connect(&self.path)?;
            stream.set_read_timeout(Some(FRAME_WAIT))?;
            stream.set_write_timeout(Some(FRAME_WAIT))?;
            Ok(stream)
        }

        /// Opens this door and answers every knock on a thread of its own,
        /// for the life of the process.
        ///
        /// # Errors
        /// Another client already listens here, the directory is not this
        /// user's alone, or the socket cannot be made.
        pub fn open(&self, deliver: impl Fn(Knock) + Send + 'static) -> io::Result<()> {
            let dir = self
                .path
                .parent()
                .ok_or_else(|| io::Error::other("no folder"))?;
            private(dir)?;
            let listener = match UnixListener::bind(&self.path) {
                Ok(listener) => listener,
                Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
                    if UnixStream::connect(&self.path).is_ok() {
                        return Err(io::Error::new(
                            io::ErrorKind::AddrInUse,
                            "another client listens at this door",
                        ));
                    }
                    // A socket left by a client that ended without closing it.
                    std::fs::remove_file(&self.path)?;
                    UnixListener::bind(&self.path)?
                }
                Err(err) => return Err(err),
            };
            std::thread::Builder::new()
                .name("baylee-link-door".into())
                .spawn(move || {
                    for stream in listener.incoming() {
                        let Ok(mut stream) = stream else { continue };
                        if stream.set_read_timeout(Some(FRAME_WAIT)).is_err()
                            || stream.set_write_timeout(Some(FRAME_WAIT)).is_err()
                        {
                            continue;
                        }
                        answer(&mut stream, &deliver);
                    }
                })?;
            Ok(())
        }
    }

    /// The doors' directory, `0700`: only this user (and the superuser) can
    /// knock. One that others may enter is closed to them first, and refused
    /// when that fails, never used as it is.
    fn private(dir: &Path) -> io::Result<()> {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        let open_to_others = |dir: &Path| -> io::Result<bool> {
            let meta = std::fs::symlink_metadata(dir)?;
            if !meta.is_dir() {
                return Err(io::Error::other("the doors' folder is not a folder"));
            }
            Ok(meta.permissions().mode() & 0o077 != 0)
        };
        if open_to_others(dir)? {
            std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
            if open_to_others(dir)? {
                return Err(io::Error::other("the doors' folder is open to others"));
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
mod windows {
    use super::{Door, Knock, answer};
    use std::fs::File;
    use std::io;
    use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle};
    use windows_sys::Win32::Foundation::{ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX,
    };
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeServerProcessId, PIPE_READMODE_BYTE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

    impl Door {
        /// Connects as a client, and lets the client behind the door take
        /// the foreground: this process was started by the one in front
        /// (the browser), which is what Windows asks before it lets a
        /// window come forward.
        pub(super) fn connect(&self) -> io::Result<File> {
            let name = String::from_utf16_lossy(&self.name[..self.name.len() - 1]);
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(name)?;
            let mut server = 0_u32;
            // SAFETY: `file` is an open pipe client handle for the call's
            // length, and `server` a valid place for the one `u32` written.
            if unsafe { GetNamedPipeServerProcessId(file.as_raw_handle(), &raw mut server) } != 0 {
                // SAFETY: no pointers; a refusal only means no foreground.
                unsafe { AllowSetForegroundWindow(server) };
            }
            Ok(file)
        }

        /// Opens this door and answers every knock, each on a thread of its
        /// own, for the life of the process.
        ///
        /// # Errors
        /// Another client already listens here (its pipe was first), or the
        /// pipe cannot be made.
        pub fn open(&self, deliver: impl Fn(Knock) + Send + Sync + 'static) -> io::Result<()> {
            let name = self.name.clone();
            let mut waiting = instance(&name, true)?;
            let deliver = std::sync::Arc::new(deliver);
            std::thread::Builder::new()
                .name("baylee-link-door".into())
                .spawn(move || {
                    loop {
                        if connected(&waiting).is_err() {
                            return;
                        }
                        let Ok(next) = instance(&name, false) else {
                            return;
                        };
                        let knocked = std::mem::replace(&mut waiting, next);
                        let deliver = std::sync::Arc::clone(&deliver);
                        // A thread per knock: a pipe read has no timeout, so
                        // a knock that never finishes holds only its own.
                        let _ = std::thread::Builder::new()
                            .name("baylee-link-knocked".into())
                            .spawn(move || {
                                let mut file = File::from(knocked);
                                answer(&mut file, &*deliver);
                                // The answer is read before the pipe closes.
                                let _ = file.sync_all();
                            });
                    }
                })?;
            Ok(())
        }
    }

    /// One instance of the pipe; the first fails when anyone else already
    /// made it, which is how a second listener (or a squatter) is noticed.
    fn instance(name: &[u16], first: bool) -> io::Result<OwnedHandle> {
        let open_mode = PIPE_ACCESS_DUPLEX
            | if first {
                FILE_FLAG_FIRST_PIPE_INSTANCE
            } else {
                0
            };
        let pipe_mode =
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS;
        // SAFETY: `name` is a NUL-terminated UTF-16 string that outlives the
        // call; a null security-attributes pointer asks for the default
        // descriptor (full control for this user, the system and the
        // administrators). No other pointer is passed.
        let handle = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                open_mode,
                pipe_mode,
                PIPE_UNLIMITED_INSTANCES,
                4096,
                4096,
                0,
                std::ptr::null(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `handle` is valid (checked above) and owned by nobody
        // else; the `OwnedHandle` closes it exactly once.
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }

    /// Waits for a client on this instance.
    fn connected(pipe: &OwnedHandle) -> io::Result<()> {
        // SAFETY: `pipe` is a live pipe handle opened without
        // FILE_FLAG_OVERLAPPED, so a null OVERLAPPED is the documented
        // blocking form.
        let ok = unsafe { ConnectNamedPipe(pipe.as_raw_handle(), std::ptr::null_mut()) };
        if ok != 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        // A client that connected between creation and this call.
        if err.raw_os_error() == i32::try_from(ERROR_PIPE_CONNECTED).ok() {
            return Ok(());
        }
        Err(err)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn scratch(tag: &str) -> std::path::PathBuf {
        // Short on purpose: a socket's path is limited to about 100 bytes.
        let dir = std::path::PathBuf::from("/tmp").join(format!(
            "bl-{tag}-{}",
            &uuid::Uuid::now_v7().simple().to_string()[24..]
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn knock(link: &str) -> Knock {
        Knock {
            link: link.into(),
            token: None,
        }
    }

    #[test]
    fn a_knock_hands_the_link_to_the_client_behind_the_door() {
        let root = scratch("hand");
        let door = Door::keyed(&root, b"one installation");
        let got = Arc::new(Mutex::new(Vec::new()));
        let into = Arc::clone(&got);
        door.open(move |knock| into.lock().unwrap().push(knock))
            .unwrap();
        let sent = Knock {
            link: "baylee://open".into(),
            token: Some("wayland-token".into()),
        };
        assert!(door.knock(&sent, Duration::from_secs(2)).unwrap());
        assert_eq!(*got.lock().unwrap(), vec![sent]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn two_installations_have_two_doors() {
        let root = Path::new("/state");
        assert_ne!(Door::keyed(root, b"a"), Door::keyed(root, b"b"));
        assert_eq!(Door::keyed(root, b"a"), Door::keyed(root, b"a"));
    }

    #[test]
    fn only_one_client_listens_at_a_door() {
        let root = scratch("one");
        let door = Door::keyed(&root, b"k");
        door.open(|_| {}).unwrap();
        let err = door.open(|_| {}).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AddrInUse);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_socket_left_behind_is_replaced() {
        let root = scratch("stale");
        let door = Door::keyed(&root, b"k");
        // A socket nobody listens at, as a crashed client leaves it.
        std::fs::create_dir_all(door.path.parent().unwrap()).unwrap();
        drop(std::os::unix::net::UnixListener::bind(&door.path).unwrap());
        assert!(door.path.exists());
        let got = Arc::new(Mutex::new(0));
        let into = Arc::clone(&got);
        door.open(move |_| *into.lock().unwrap() += 1).unwrap();
        assert!(
            door.knock(&knock("baylee://open"), Duration::from_secs(2))
                .unwrap()
        );
        assert_eq!(*got.lock().unwrap(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nobody_behind_the_door_is_an_error_after_the_wait() {
        let root = scratch("none");
        let door = Door::keyed(&root, b"k");
        let started = Instant::now();
        assert!(
            door.knock(&knock("baylee://open"), Duration::from_millis(300))
                .is_err()
        );
        assert!(started.elapsed() >= Duration::from_millis(300));
        assert!(started.elapsed() < Duration::from_secs(3));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_doors_folder_is_this_users_alone() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = scratch("mode");
        let door = Door::keyed(&root, b"k");
        let dir = door.path.parent().unwrap().to_path_buf();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o777)).unwrap();
        door.open(|_| {}).unwrap();
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_frame_that_is_not_one_is_refused_and_delivers_nothing() {
        let root = scratch("bad");
        let door = Door::keyed(&root, b"k");
        let got = Arc::new(Mutex::new(0));
        let into = Arc::clone(&got);
        door.open(move |_| *into.lock().unwrap() += 1).unwrap();
        let too_long = u16::try_from(MAX_LINK + 1).unwrap().to_be_bytes();
        for frame in [
            b"BLK2\0\x01x\0\0".to_vec(),
            [b"BLK1".as_slice(), &too_long].concat(),
            b"BLK1\0\x02\xff\xfe\0\0".to_vec(),
            b"BLK1\0\x05short".to_vec(),
        ] {
            let mut stream = door.connect().unwrap();
            stream.write_all(&frame).unwrap();
            let mut reply = [9];
            // Refused with a byte, or hung up on after the frame's wait.
            let read = stream.read(&mut reply).unwrap_or(0);
            assert!(read == 0 || reply[0] == REFUSED, "{frame:?}");
        }
        assert_eq!(*got.lock().unwrap(), 0);
        // The door still answers a good knock afterwards.
        assert!(
            door.knock(&knock("baylee://open"), Duration::from_secs(2))
                .unwrap()
        );
        assert_eq!(*got.lock().unwrap(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_link_too_long_is_not_sent() {
        let door = Door::keyed(Path::new("/nowhere"), b"k");
        let long = knock(&"x".repeat(MAX_LINK + 1));
        assert_eq!(
            door.knock(&long, Duration::ZERO).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn the_first_link_among_the_arguments_is_found() {
        let args: Vec<std::ffi::OsString> = ["--console", "BAYLEE://open", "baylee://signin"]
            .into_iter()
            .map(Into::into)
            .collect();
        assert_eq!(among(&args).as_deref(), Some("BAYLEE://open"));
        assert_eq!(among(&["--resume".into()]), None);
    }
}
