//! Explicit opt-in system test: native tools create and mount a real image.
//! It never starts the app, opens Finder, or detaches a pre-existing volume.
use super::*;
use image::{Rgba, RgbaImage};
use std::{fs, os::unix::fs::symlink, path::PathBuf, process::Command};

struct Mounted(PathBuf);
impl Drop for Mounted {
    fn drop(&mut self) {
        let status = Command::new("/usr/bin/hdiutil")
            .args(["detach", "-quiet"])
            .arg(&self.0)
            .status();
        if !status.is_ok_and(|s| s.success()) {
            let _ = Command::new("/usr/bin/hdiutil")
                .args(["detach", "-force", "-quiet"])
                .arg(&self.0)
                .status();
        }
    }
}
fn command(cmd: &mut Command) -> Vec<u8> {
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "{cmd:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
#[ignore = "creates and mounts a temporary disk image; run explicitly on macOS"]
fn native_dmg_preserves_background_volume_icon_layout_and_bundle_without_python() {
    let tmp = tempfile::tempdir().unwrap();
    let app = tmp.path().join("Baylee.app");
    let resources = app.join("Contents/Resources");
    fs::create_dir_all(resources.join("assets")).unwrap();
    fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
    fs::write(resources.join("assets/fixture.txt"), b"bundle fixture\n").unwrap();
    symlink("../Resources/assets", app.join("Contents/MacOS/assets")).unwrap();
    let brand =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../baylee-client/assets/brand/baylee.icns");
    let icon = resources.join("Baylee.icns");
    fs::copy(brand, &icon).unwrap();
    let background = tmp.path().join("background.png");
    RgbaImage::from_pixel(660, 420, Rgba([22, 33, 44, 255]))
        .save(&background)
        .unwrap();
    RgbaImage::from_pixel(1320, 840, Rgba([22, 33, 44, 255]))
        .save(tmp.path().join("background@2x.png"))
        .unwrap();
    let output = tmp.path().join("fixture.dmg");
    build(&app, &background, &icon, &output).unwrap();
    assert!(output.is_file());
    let mount = tmp.path().join("verification-mount");
    fs::create_dir(&mount).unwrap();
    // The mount path is exclusively inside this test's fresh temp directory.
    command(
        Command::new("/usr/bin/hdiutil")
            .args([
                "attach",
                "-readonly",
                "-nobrowse",
                "-noautoopen",
                "-mountpoint",
            ])
            .arg(&mount)
            .arg(&output),
    );
    let _mounted = Mounted(mount.clone());
    check_layout(&mount).unwrap();
    let mut names: Vec<_> = fs::read_dir(&mount)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            ".DS_Store",
            ".VolumeIcon.icns",
            ".background.tiff",
            "Applications",
            "Baylee.app"
        ]
    );
    assert_eq!(
        fs::read_link(mount.join("Applications")).unwrap(),
        Path::new("/Applications")
    );
    assert_eq!(
        fs::read_link(mount.join("Baylee.app/Contents/MacOS/assets")).unwrap(),
        Path::new("../Resources/assets")
    );
    assert_eq!(
        fs::read(mount.join("Baylee.app/Contents/Resources/assets/fixture.txt")).unwrap(),
        b"bundle fixture\n"
    );
    assert_eq!(
        fs::read(mount.join(".VolumeIcon.icns")).unwrap(),
        fs::read(icon).unwrap()
    );
    let tiff = mount.join(".background.tiff");
    assert!(fs::metadata(&tiff).unwrap().len() > 0);
    // Inspect the actual TIFF image directories; a plain 1x file fails.
    assert_eq!(
        tiff_dimensions(&fs::read(tiff).unwrap()),
        [(660, 420), (1320, 840)]
    );
    // All native subprocesses use absolute OS tool paths; no Python runtime.
    let source = include_str!("../dmg.rs");
    assert!(!source.contains("Command::new(\"python"));
}

fn tiff_dimensions(bytes: &[u8]) -> Vec<(u32, u32)> {
    let little = &bytes[..2] == b"II";
    assert!(little || &bytes[..2] == b"MM");
    let word = |at| {
        let raw = bytes[at..at + 2].try_into().unwrap();
        if little {
            u16::from_le_bytes(raw)
        } else {
            u16::from_be_bytes(raw)
        }
    };
    let long = |at| {
        let raw = bytes[at..at + 4].try_into().unwrap();
        if little {
            u32::from_le_bytes(raw)
        } else {
            u32::from_be_bytes(raw)
        }
    };
    assert_eq!(word(2), 42);
    let mut at = usize::try_from(long(4)).unwrap();
    let mut dimensions = Vec::new();
    while at != 0 {
        assert!(dimensions.len() < 3, "cyclic/unexpected TIFF directories");
        let entries = usize::from(word(at));
        let mut size = [0; 2];
        for i in 0..entries {
            let row = at + 2 + i * 12;
            let tag = word(row);
            if tag == 256 || tag == 257 {
                assert_eq!(long(row + 4), 1);
                size[usize::from(tag - 256)] = match word(row + 2) {
                    3 => u32::from(word(row + 8)),
                    4 => long(row + 8),
                    other => panic!("unexpected TIFF dimension type {other}"),
                };
            }
        }
        dimensions.push((size[0], size[1]));
        at = usize::try_from(long(at + 2 + entries * 12)).unwrap();
    }
    dimensions
}
