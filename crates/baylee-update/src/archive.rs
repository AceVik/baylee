//! Unpacking a release archive into the staging directory.
//!
//! Only after its signature verified ([`crate::sign`]), so what is unpacked
//! is ours; the checks here are still the ones any unpacker owes. No entry
//! may land outside `dest` (no absolute path, no `..` past the top, no
//! writing through a link), and no symlink may point outside it either.
//!
//! What the archive must keep for the program to start: the executable bit
//! on Linux and macOS, and on macOS the symlink `package-client.sh` puts in
//! the bundle (`Contents/MacOS/assets -> ../Resources/assets`). Written out
//! as a file, that link would break the bundle's seal and macOS would call
//! the app damaged, so a symlink entry is made a symlink.

use std::fs::{self, File};
use std::io::{self, Read as _};
use std::path::{Component, Path, PathBuf};

/// The two formats `scripts/package-client.sh` writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Windows and macOS (`ditto`, `7z` or `Compress-Archive`).
    Zip,
    /// Linux (`tar -czf`).
    TarGz,
}

impl Format {
    /// The format an archive's name says it is in.
    #[must_use]
    pub fn of_name(name: &str) -> Option<Self> {
        let path = Path::new(name);
        let ext =
            |p: &Path, want: &str| p.extension().is_some_and(|e| e.eq_ignore_ascii_case(want));
        if ext(path, "zip") {
            Some(Self::Zip)
        } else if ext(path, "gz") && ext(Path::new(path.file_stem()?), "tar") {
            Some(Self::TarGz)
        } else {
            None
        }
    }
}

fn refused(what: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_string())
}

/// Unpacks `archive` into `dest`, which must exist and should be empty.
///
/// # Errors
///
/// An I/O error, or `InvalidData` for an entry that would leave `dest`.
pub fn unpack(archive: &Path, format: Format, dest: &Path) -> io::Result<()> {
    match format {
        Format::Zip => unpack_zip(archive, dest),
        Format::TarGz => unpack_tar_gz(archive, dest),
    }
}

/// Whether `path`, relative and read lexically, stays inside its root.
fn is_inside(path: &Path) -> bool {
    let mut depth = 0usize;
    for part in path.components() {
        match part {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => match depth.checked_sub(1) {
                Some(up) => depth = up,
                None => return false,
            },
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

/// Whether a link at `link` (relative to the root) pointing at `target`
/// stays inside the root.
fn link_stays_inside(link: &Path, target: &Path) -> bool {
    let base = link.parent().unwrap_or(Path::new(""));
    target.is_relative() && is_inside(&base.join(target))
}

/// Refuses to write `rel` when a directory on its way is a symlink, which
/// would carry the write somewhere the entry's own name does not say.
fn no_link_on_the_way(dest: &Path, rel: &Path) -> io::Result<()> {
    let mut at = dest.to_path_buf();
    let parts: Vec<_> = rel.components().collect();
    for part in parts.iter().take(parts.len().saturating_sub(1)) {
        at.push(part);
        if let Ok(meta) = fs::symlink_metadata(&at)
            && meta.file_type().is_symlink()
        {
            return Err(refused(format!("{} passes through a link", rel.display())));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_link(target: &Path, at: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}

#[cfg(not(unix))]
fn make_link(target: &Path, at: &Path) -> io::Result<()> {
    let _ = (target, at);
    Err(refused("a symlink in an archive for a system without them"))
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o777))
}

#[cfg(not(unix))]
#[allow(clippy::unnecessary_wraps)] // the unix twin's signature: the caller is shared
fn set_mode(_: &Path, _: u32) -> io::Result<()> {
    Ok(())
}

const S_IFMT: u32 = 0o170_000;
const S_IFLNK: u32 = 0o120_000;

fn unpack_zip(archive: &Path, dest: &Path) -> io::Result<()> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?).map_err(refused)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(refused)?;
        let rel: PathBuf = entry
            .enclosed_name()
            .ok_or_else(|| refused(format!("{} leaves the archive", entry.name())))?;
        if !is_inside(&rel) {
            return Err(refused(format!("{} leaves the archive", rel.display())));
        }
        no_link_on_the_way(dest, &rel)?;
        let out = dest.join(&rel);
        let mode = entry.unix_mode();
        if entry.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        if mode.is_some_and(|m| m & S_IFMT == S_IFLNK) {
            let mut target = String::new();
            entry.read_to_string(&mut target)?;
            let target = PathBuf::from(target);
            if !link_stays_inside(&rel, &target) {
                return Err(refused(format!(
                    "{} links outside the archive ({})",
                    rel.display(),
                    target.display()
                )));
            }
            make_link(&target, &out)?;
            continue;
        }
        let mut file = File::create(&out)?;
        io::copy(&mut entry, &mut file)?;
        drop(file);
        if let Some(mode) = mode {
            set_mode(&out, mode)?;
        }
    }
    Ok(())
}

fn unpack_tar_gz(archive: &Path, dest: &Path) -> io::Result<()> {
    let gz = flate2::read::GzDecoder::new(File::open(archive)?);
    let mut tar = tar::Archive::new(gz);
    tar.set_preserve_permissions(true);
    tar.set_overwrite(false);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let rel = entry.path()?.into_owned();
        if !is_inside(&rel) {
            return Err(refused(format!("{} leaves the archive", rel.display())));
        }
        no_link_on_the_way(dest, &rel)?;
        let kind = entry.header().entry_type();
        if kind.is_symlink() || kind.is_hard_link() {
            let target = entry
                .link_name()?
                .ok_or_else(|| refused(format!("{} links nowhere", rel.display())))?
                .into_owned();
            let inside = if kind.is_symlink() {
                link_stays_inside(&rel, &target)
            } else {
                target.is_relative() && is_inside(&target)
            };
            if !inside {
                return Err(refused(format!(
                    "{} links outside the archive ({})",
                    rel.display(),
                    target.display()
                )));
            }
        }
        if !entry.unpack_in(dest)? {
            return Err(refused(format!("{} leaves the archive", rel.display())));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn scratch(tag: &str) -> PathBuf {
        static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "baylee-archive-{tag}-{}-{}",
            std::process::id(),
            RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A zip as `ditto -c -k --keepParent` writes the macOS bundle: a
    /// directory tree, an executable, and a symlink stored with its unix
    /// mode.
    fn bundle_zip(path: &Path, extra: &[(&str, Option<&str>)]) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        let file = zip::write::SimpleFileOptions::default();
        zip.add_directory("pkg/Baylee.app/Contents/MacOS/", file)
            .unwrap();
        zip.start_file(
            "pkg/Baylee.app/Contents/MacOS/baylee-client",
            file.unix_permissions(0o755),
        )
        .unwrap();
        zip.write_all(b"#!new program").unwrap();
        zip.start_file(
            "pkg/Baylee.app/Contents/Resources/assets/font.ttf",
            file.unix_permissions(0o644),
        )
        .unwrap();
        zip.write_all(b"font").unwrap();
        zip.add_symlink(
            "pkg/Baylee.app/Contents/MacOS/assets",
            "../Resources/assets",
            file,
        )
        .unwrap();
        for (name, link) in extra {
            if let Some(target) = link {
                zip.add_symlink(*name, *target, file).unwrap();
            } else {
                zip.start_file(*name, file).unwrap();
                zip.write_all(b"x").unwrap();
            }
        }
        zip.finish().unwrap();
    }

    // Unix only: its archive holds a link, which a system without them
    // refuses (`make_link`).
    #[cfg(unix)]
    #[test]
    fn a_zip_keeps_its_tree_its_program_bit_and_its_link() {
        let dir = scratch("zip");
        let archive = dir.join("a.zip");
        bundle_zip(&archive, &[]);
        let dest = dir.join("out");
        fs::create_dir_all(&dest).unwrap();
        unpack(&archive, Format::Zip, &dest).unwrap();
        let program = dest.join("pkg/Baylee.app/Contents/MacOS/baylee-client");
        assert_eq!(fs::read(&program).unwrap(), b"#!new program");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let link = dest.join("pkg/Baylee.app/Contents/MacOS/assets");
            assert_eq!(
                fs::metadata(&program).unwrap().permissions().mode() & 0o777,
                0o755
            );
            assert!(
                fs::symlink_metadata(&link)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
            assert_eq!(
                fs::read_link(&link).unwrap(),
                PathBuf::from("../Resources/assets")
            );
            // Through the link, the font.
            assert_eq!(fs::read(link.join("font.ttf")).unwrap(), b"font");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_zip_entry_that_climbs_out_is_refused() {
        let dir = scratch("zip-climb");
        let archive = dir.join("a.zip");
        bundle_zip(&archive, &[("pkg/../../evil", None)]);
        let dest = dir.join("out");
        fs::create_dir_all(&dest).unwrap();
        let err = unpack(&archive, Format::Zip, &dest).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(!dir.join("evil").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_zip_link_out_of_the_archive_is_refused() {
        for target in ["../../../etc", "/etc"] {
            let dir = scratch("zip-link");
            let archive = dir.join("a.zip");
            bundle_zip(&archive, &[("pkg/escape", Some(target))]);
            let dest = dir.join("out");
            fs::create_dir_all(&dest).unwrap();
            let err = unpack(&archive, Format::Zip, &dest).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidData, "{target}");
            assert!(fs::symlink_metadata(dest.join("pkg/escape")).is_err());
            let _ = fs::remove_dir_all(&dir);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_zip_write_through_a_link_is_refused() {
        let dir = scratch("zip-through");
        let archive = dir.join("a.zip");
        // The link stays inside, but a later entry writes through it.
        bundle_zip(
            &archive,
            &[("pkg/Baylee.app/Contents/MacOS/assets/sneaked", None)],
        );
        let dest = dir.join("out");
        fs::create_dir_all(&dest).unwrap();
        let err = unpack(&archive, Format::Zip, &dest).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(
            !dest
                .join("pkg/Baylee.app/Contents/Resources/assets/sneaked")
                .exists()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    fn tar_gz(path: &Path, add: impl FnOnce(&mut tar::Builder<flate2::write::GzEncoder<File>>)) {
        let gz =
            flate2::write::GzEncoder::new(File::create(path).unwrap(), flate2::Compression::fast());
        let mut tar = tar::Builder::new(gz);
        add(&mut tar);
        tar.into_inner().unwrap().finish().unwrap();
    }

    fn file_header(size: u64, mode: u32) -> tar::Header {
        let mut header = tar::Header::new_gnu();
        header.set_size(size);
        header.set_mode(mode);
        header.set_entry_type(tar::EntryType::Regular);
        header
    }

    #[test]
    fn a_tar_gz_keeps_its_tree_and_its_program_bit() {
        let dir = scratch("tgz");
        let archive = dir.join("a.tar.gz");
        tar_gz(&archive, |tar| {
            tar.append_data(
                &mut file_header(8, 0o755),
                "pkg/baylee-client",
                &b"#!linux!"[..],
            )
            .unwrap();
            tar.append_data(
                &mut file_header(4, 0o644),
                "pkg/assets/font.ttf",
                &b"font"[..],
            )
            .unwrap();
        });
        let dest = dir.join("out");
        fs::create_dir_all(&dest).unwrap();
        unpack(&archive, Format::TarGz, &dest).unwrap();
        assert_eq!(
            fs::read(dest.join("pkg/baylee-client")).unwrap(),
            b"#!linux!"
        );
        assert_eq!(fs::read(dest.join("pkg/assets/font.ttf")).unwrap(), b"font");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = fs::metadata(dest.join("pkg/baylee-client"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o755);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tar_gz_link_out_of_the_archive_is_refused() {
        let dir = scratch("tgz-link");
        let archive = dir.join("a.tar.gz");
        tar_gz(&archive, |tar| {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);
            tar.append_link(&mut header, "pkg/escape", "../../outside")
                .unwrap();
        });
        let dest = dir.join("out");
        fs::create_dir_all(&dest).unwrap();
        let err = unpack(&archive, Format::TarGz, &dest).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        let _ = fs::remove_dir_all(&dir);
    }

    /// `tar`'s own `append_data` refuses a `..` path, so the header is
    /// written by hand, as a hostile archive would be.
    #[test]
    fn a_tar_gz_entry_that_climbs_out_is_refused() {
        let dir = scratch("tgz-climb");
        let archive = dir.join("a.tar.gz");
        tar_gz(&archive, |tar| {
            let mut header = file_header(1, 0o644);
            let name = b"pkg/../../evil";
            header.as_gnu_mut().unwrap().name[..name.len()].copy_from_slice(name);
            header.set_cksum();
            tar.append(&header, &b"x"[..]).unwrap();
        });
        let dest = dir.join("out");
        fs::create_dir_all(&dest).unwrap();
        let err = unpack(&archive, Format::TarGz, &dest).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(!dir.join("evil").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_format_is_read_off_the_name() {
        assert_eq!(
            Format::of_name("a-x86_64-pc-windows-msvc.zip"),
            Some(Format::Zip)
        );
        assert_eq!(Format::of_name("a.tar.gz"), Some(Format::TarGz));
        assert_eq!(Format::of_name("a.tar.gz.sig"), None);
    }

    #[test]
    fn inside_is_read_lexically() {
        assert!(is_inside(Path::new("a/b/../c")));
        assert!(!is_inside(Path::new("a/../../c")));
        assert!(!is_inside(Path::new("/abs")));
        assert!(link_stays_inside(
            Path::new("Baylee.app/Contents/MacOS/assets"),
            Path::new("../Resources/assets")
        ));
        assert!(!link_stays_inside(Path::new("top"), Path::new("../x")));
    }
}
