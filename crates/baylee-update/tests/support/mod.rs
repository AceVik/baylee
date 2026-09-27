//! What the integration tests share: installation trees per system as the
//! release workflow packs them, archives of them, and a stub HTTP server.

#![allow(dead_code)] // each test binary uses its own part

use baylee_update::apply::Install;
use baylee_update::plan::{Os, STAGE};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{BufRead as _, BufReader, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// A fresh directory of its own: nextest and `cargo test` run tests at once.
pub fn scratch(tag: &str) -> PathBuf {
    static RUN: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "baylee-update-{tag}-{}-{}",
        std::process::id(),
        RUN.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// One entry of a tree.
#[derive(Clone, Debug)]
pub enum Node {
    /// A file, its bytes and unix mode.
    File(Vec<u8>, u32),
    /// A symlink and its target.
    Link(String),
}

/// A tree, by relative path with `/`.
pub type Tree = BTreeMap<String, Node>;

fn file(text: String) -> Node {
    Node::File(text.into_bytes(), 0o644)
}

fn program(text: String) -> Node {
    Node::File(text.into_bytes(), 0o755)
}

/// What `scripts/package-client.sh` packs for `os` at `version`, the
/// versioned top folder left out.
pub fn release_tree(os: Os, version: &str) -> Tree {
    let mut tree = Tree::new();
    tree.insert("LICENSE".into(), file("AGPL".into()));
    tree.insert("NOTICE".into(), file(format!("notice {version}")));
    tree.insert("README.txt".into(), file(format!("Baylee {version}")));
    match os {
        Os::MacOs => {
            let app = "Baylee.app/Contents";
            tree.insert(
                format!("{app}/MacOS/baylee-client"),
                program(format!("mac program {version}")),
            );
            tree.insert(
                format!("{app}/MacOS/assets"),
                Node::Link("../Resources/assets".into()),
            );
            tree.insert(
                format!("{app}/Resources/assets/fonts/AlegreyaSans.ttf"),
                file(format!("font {version}")),
            );
            tree.insert(format!("{app}/Resources/Baylee.icns"), file("icon".into()));
            tree.insert(
                format!("{app}/Info.plist"),
                file(format!("plist {version}")),
            );
            tree.insert(
                "baylee-client.dSYM/Contents/Info.plist".into(),
                file(format!("symbols {version}")),
            );
        }
        Os::Windows => {
            tree.insert(
                "baylee-client.exe".into(),
                program(format!("windows program {version}")),
            );
            tree.insert("baylee_client.pdb".into(), file(format!("pdb {version}")));
            tree.insert(
                "assets/fonts/AlegreyaSans.ttf".into(),
                file(format!("font {version}")),
            );
        }
        Os::Linux => {
            tree.insert(
                "baylee-client".into(),
                program(format!("linux program {version}")),
            );
            tree.insert(
                "assets/fonts/AlegreyaSans.ttf".into(),
                file(format!("font {version}")),
            );
        }
    }
    if version.ends_with('3') {
        // A file only the newer release has, and (below) one only the
        // older one had, so "replaced" is told apart from "merged".
        let at = match os {
            Os::MacOs => "Baylee.app/Contents/Resources/assets/fonts/New.ttf",
            _ => "assets/fonts/New.ttf",
        };
        tree.insert(at.into(), file("new".into()));
    } else {
        let at = match os {
            Os::MacOs => "Baylee.app/Contents/Resources/assets/fonts/Gone.ttf",
            _ => "assets/fonts/Gone.ttf",
        };
        tree.insert(at.into(), file("gone".into()));
    }
    tree
}

/// Writes `tree` under `root`.
pub fn write_tree(root: &Path, tree: &Tree) {
    for (rel, node) in tree {
        let at = root.join(rel);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        match node {
            Node::File(bytes, mode) => {
                std::fs::write(&at, bytes).unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt as _;
                    std::fs::set_permissions(&at, std::fs::Permissions::from_mode(*mode)).unwrap();
                }
                #[cfg(not(unix))]
                let _ = mode;
            }
            Node::Link(target) => {
                #[cfg(unix)]
                std::os::unix::fs::symlink(target, &at).unwrap();
                #[cfg(not(unix))]
                let _ = target;
            }
        }
    }
}

/// Everything under `root` but the staging directory, links not followed:
/// path → `file <mode> <bytes>` or `link <target>`.
pub fn snapshot(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if rel == STAGE {
                continue;
            }
            let meta = std::fs::symlink_metadata(&path).unwrap();
            if meta.file_type().is_symlink() {
                let target = std::fs::read_link(&path).unwrap();
                out.insert(rel, format!("link {}", target.display()));
            } else if meta.is_dir() {
                walk(root, &path, out);
            } else {
                #[cfg(unix)]
                let mode = {
                    use std::os::unix::fs::PermissionsExt as _;
                    meta.permissions().mode() & 0o777
                };
                #[cfg(not(unix))]
                let mode = 0o644;
                let bytes = std::fs::read(&path).unwrap();
                out.insert(
                    rel,
                    format!("file {mode:o} {}", String::from_utf8_lossy(&bytes)),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// What [`snapshot`] would say about `tree` written at `root`.
pub fn expected(tree: &Tree) -> BTreeMap<String, String> {
    tree.iter()
        .map(|(rel, node)| {
            let text = match node {
                Node::File(bytes, mode) => {
                    let mode = if cfg!(unix) { *mode } else { 0o644 };
                    format!("file {mode:o} {}", String::from_utf8_lossy(bytes))
                }
                Node::Link(target) => format!("link {target}"),
            };
            (rel.clone(), text)
        })
        .collect()
}

/// The running program's path in an installation at `base`.
pub fn exe(os: Os, base: &Path) -> PathBuf {
    match os {
        Os::MacOs => base.join("Baylee.app/Contents/MacOS/baylee-client"),
        Os::Windows => base.join("baylee-client.exe"),
        Os::Linux => base.join("baylee-client"),
    }
}

/// An installation of `os` at `version` in a fresh `base`, with a file of
/// the player's beside it that no update may touch.
pub fn installed(os: Os, version: &str, tag: &str) -> (PathBuf, Install, Tree) {
    let base = scratch(tag).join("baylee");
    let mut tree = release_tree(os, version);
    if os == Os::MacOs {
        // The unpacked folder has the dSYM only if the player kept it.
        tree.retain(|rel, _| !rel.starts_with("baylee-client.dSYM"));
    }
    tree.insert("my-notes.txt".into(), file("mine".into()));
    write_tree(&base, &tree);
    let install = Install::around(&exe(os, &base), os).unwrap();
    (base, install, tree)
}

/// The target triple an archive for `os` is named after in these tests.
pub fn target(os: Os) -> &'static str {
    match os {
        Os::MacOs => "aarch64-apple-darwin",
        Os::Windows => "x86_64-pc-windows-msvc",
        Os::Linux => "x86_64-unknown-linux-gnu",
    }
}

/// The archive the release workflow would publish for `os` at `version`:
/// its name and bytes, the tree inside a versioned top folder.
pub fn archive(os: Os, version: &str) -> (String, Vec<u8>) {
    let target = target(os);
    let name = baylee_update::release::archive_name(version, target);
    let top = format!("baylee-client-{version}-{target}");
    let tree = release_tree(os, version);
    let bytes = if baylee_update::archive::Format::of_name(&name)
        == Some(baylee_update::archive::Format::Zip)
    {
        let mut out = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut out);
        let options = zip::write::SimpleFileOptions::default();
        for (rel, node) in &tree {
            let path = format!("{top}/{rel}");
            match node {
                Node::File(bytes, mode) => {
                    zip.start_file(path, options.unix_permissions(*mode))
                        .unwrap();
                    zip.write_all(bytes).unwrap();
                }
                Node::Link(target) => zip.add_symlink(path, target, options).unwrap(),
            }
        }
        zip.finish().unwrap();
        out.into_inner()
    } else {
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut tar = tar::Builder::new(gz);
        for (rel, node) in &tree {
            let path = format!("{top}/{rel}");
            let mut header = tar::Header::new_gnu();
            match node {
                Node::File(bytes, mode) => {
                    header.set_size(bytes.len() as u64);
                    header.set_mode(*mode);
                    header.set_entry_type(tar::EntryType::Regular);
                    tar.append_data(&mut header, path, &bytes[..]).unwrap();
                }
                Node::Link(target) => {
                    header.set_entry_type(tar::EntryType::Symlink);
                    header.set_size(0);
                    tar.append_link(&mut header, path, target).unwrap();
                }
            }
        }
        tar.into_inner().unwrap().finish().unwrap()
    };
    (name, bytes)
}

/// One canned answer.
#[derive(Clone, Debug)]
pub struct Answer {
    /// The status code.
    pub status: u16,
    /// Extra headers.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl Answer {
    /// `200` with `body`.
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: body.into(),
        }
    }
}

/// Every request a [`Stub`] saw: its path and its headers.
pub type Seen = Arc<Mutex<Vec<(String, BTreeMap<String, String>)>>>;

/// A stub HTTP/1.1 server on the loopback: canned answers by path, every
/// request recorded. An answer with an `ETag` header answers `304` to a
/// request that sends it back.
pub struct Stub {
    /// `http://127.0.0.1:<port>`.
    pub base: String,
    routes: Arc<Mutex<BTreeMap<String, Answer>>>,
    /// Every request: its path and its headers, lower-cased names.
    pub seen: Seen,
}

impl Stub {
    /// Starts one, answering until the test process ends.
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let routes: Arc<Mutex<BTreeMap<String, Answer>>> = Arc::default();
        let seen: Seen = Arc::default();
        let (r, s) = (routes.clone(), seen.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                let path = line.split_whitespace().nth(1).unwrap_or("/").to_owned();
                let mut headers = BTreeMap::new();
                loop {
                    let mut header = String::new();
                    if reader.read_line(&mut header).unwrap_or(0) == 0 || header.trim().is_empty() {
                        break;
                    }
                    if let Some((k, v)) = header.split_once(':') {
                        headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
                    }
                }
                // Every request here is a GET: no body to read.
                s.lock().unwrap().push((path.clone(), headers.clone()));
                let answer = r.lock().unwrap().get(&path).cloned().unwrap_or(Answer {
                    status: 404,
                    headers: Vec::new(),
                    body: b"not here".to_vec(),
                });
                let etag = answer
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("etag"))
                    .map(|(_, v)| v.clone());
                let not_modified = etag.is_some() && headers.get("if-none-match") == etag.as_ref();
                let (status, body) = if not_modified {
                    (304, Vec::new())
                } else {
                    (answer.status, answer.body.clone())
                };
                let mut head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n",
                    body.len()
                );
                for (k, v) in &answer.headers {
                    let _ = write!(head, "{k}: {v}\r\n");
                }
                head.push_str("\r\n");
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
                let _ = stream.flush();
            }
        });
        Self { base, routes, seen }
    }

    /// Answers `path` with `answer` from now on.
    pub fn route(&self, path: &str, answer: Answer) {
        self.routes.lock().unwrap().insert(path.to_owned(), answer);
    }

    /// How many requests arrived.
    pub fn count(&self) -> usize {
        self.seen.lock().unwrap().len()
    }

    /// The paths asked for, in order.
    pub fn paths(&self) -> Vec<String> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .map(|(p, _)| p.clone())
            .collect()
    }
}
