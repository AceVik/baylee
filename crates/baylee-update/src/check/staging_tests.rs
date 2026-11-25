//! The staging directory between two checks, and what an archive has to look
//! like before it may stand as the next version.

use super::*;
use crate::plan::Os;

struct Dir(std::path::PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "baylee-staging-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn file(&self, name: &str) -> std::path::PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"x").unwrap();
        path
    }

    fn left(&self) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn clearing_the_staging_takes_downloads_and_keeps_what_recovery_reads() {
    let dir = Dir::new("clear");
    for name in [
        "new/baylee",
        "unpack/baylee/baylee",
        "baylee-0.1.0-linux.tar.gz",
        "baylee-0.1.0-windows.zip",
        "baylee-0.1.0-linux.tar.part",
        "download.PART",
        "staged.json",
        "journal.json",
        "failed.json",
        "old/baylee",
        "notes.txt",
    ] {
        dir.file(name);
    }
    clear_staging(&dir.0);
    assert_eq!(
        dir.left(),
        ["failed.json", "journal.json", "notes.txt", "old"]
    );
    // Nothing there at all is no failure.
    clear_staging(&dir.0.join("nowhere"));
}

/// Packs `files` as a tar.gz and unpacks it as the version `baylee-2`.
fn unpacked(name: &str, files: &[&str]) -> (Dir, io::Result<()>) {
    let dir = Dir::new(name);
    let archive = dir.0.join("baylee-2.tar.gz");
    let gz = flate2::write::GzEncoder::new(
        fs::File::create(&archive).unwrap(),
        flate2::Compression::fast(),
    );
    let mut tar = tar::Builder::new(gz);
    for path in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(1);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, path, &b"x"[..]).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap();
    let install = Install {
        state: None,
        os: Os::Linux,
        base: dir.0.clone(),
        program: "baylee-client".into(),
    };
    let result = unpack_into_new(
        &archive,
        Format::of_name("baylee-2.tar.gz").unwrap(),
        &dir.0.join("unpack"),
        &dir.0.join("new"),
        &install,
        "baylee-2",
    );
    (dir, result)
}

#[test]
fn the_one_folder_of_the_archive_becomes_the_new_version() {
    let files = ["baylee-2/baylee-client", "baylee-2/assets/a.png"];
    let (dir, result) = unpacked("good", &files);
    result.unwrap();
    assert!(dir.0.join("new/baylee-client").is_file());
    assert!(dir.0.join("new/assets/a.png").is_file());
}

#[test]
fn what_a_mac_packs_beside_the_folder_is_not_a_second_one() {
    let files = ["__MACOSX/._baylee-2", "baylee-2/baylee-client"];
    let (dir, result) = unpacked("macosx", &files);
    result.unwrap();
    assert_eq!(
        fs::read_dir(dir.0.join("new")).unwrap().count(),
        1,
        "and it is not installed"
    );
}

#[test]
fn an_archive_of_another_shape_never_becomes_the_new_version() {
    for (name, files) in [
        ("two", &["baylee-2/baylee-client", "extra/file"][..]),
        ("loose", &["baylee-2/baylee-client", "readme.txt"]),
        ("flat", &["baylee-client"]),
        ("relabelled", &["baylee-3/baylee-client"]),
        ("empty", &[]),
        ("foreign", &["baylee-2/baylee-client.exe"]),
        ("buried", &["baylee-2/bin/baylee-client"]),
    ] {
        let (dir, result) = unpacked(name, files);
        assert!(result.is_err(), "{name}");
        assert!(!dir.0.join("new").exists(), "{name}");
    }
}
