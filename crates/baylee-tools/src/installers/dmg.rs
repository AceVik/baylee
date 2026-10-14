//! Native macOS hdiutil packaging. No Finder UI is opened.
use anyhow::Result;
use std::path::Path;

#[cfg(target_os = "macos")]
mod native {
    use super::super::layout::{self, BACKGROUND, BackgroundIdentity};
    use super::{Path, Result};
    use anyhow::{Context, ensure};
    use std::fmt::Write as _;
    use std::{
        fs,
        os::unix::fs::{MetadataExt, symlink},
        path::PathBuf,
        process::Command,
        time::UNIX_EPOCH,
    };

    fn command(command: &mut Command) -> Result<()> {
        let output = command
            .output()
            .with_context(|| format!("starting {command:?}"))?;
        // package-installers stdout is a list of paths, never tool chatter.
        eprint!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        ensure!(
            output.status.success(),
            "{command:?} exited {}",
            output.status
        );
        Ok(())
    }
    struct Mount {
        path: PathBuf,
        attached: bool,
    }
    impl Mount {
        fn detach(&mut self) -> Result<()> {
            command(
                Command::new("/usr/bin/hdiutil")
                    .args(["detach", "-quiet"])
                    .arg(&self.path),
            )?;
            self.attached = false;
            Ok(())
        }
    }
    impl Drop for Mount {
        fn drop(&mut self) {
            if self.attached {
                // A failed build must not leave a mounted writable image.
                let _ = command(
                    Command::new("/usr/bin/hdiutil")
                        .args(["detach", "-force", "-quiet"])
                        .arg(&self.path),
                );
            }
        }
    }
    fn created(metadata: &fs::Metadata) -> Result<u32> {
        Ok(u32::try_from(
            metadata
                .created()?
                .duration_since(UNIX_EPOCH)?
                .as_secs()
                .checked_add(2_082_844_800)
                .context("HFS creation time overflow")?,
        )?)
    }
    fn identity(mount: &Path) -> Result<BackgroundIdentity> {
        let volume = fs::metadata(mount)?;
        let file = fs::metadata(mount.join(BACKGROUND))?;
        Ok(BackgroundIdentity {
            volume_created: created(&volume)?,
            root_cnid: u32::try_from(volume.ino())?,
            file_created: created(&file)?,
            file_cnid: u32::try_from(file.ino())?,
        })
    }
    fn tree_size(path: &Path) -> Result<u64> {
        let metadata = fs::symlink_metadata(path)?;
        let mut size = metadata.len();
        if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                size = size
                    .checked_add(tree_size(&entry?.path())?)
                    .context("app size overflow")?;
            }
        }
        Ok(size)
    }
    fn volume_icon(mount: &Path, icon: &Path) -> Result<()> {
        fs::copy(icon, mount.join(".VolumeIcon.icns"))?;
        // Fresh HFS+ root: preserve existing FinderInfo and set kHasCustomIcon.
        let old = Command::new("/usr/bin/xattr")
            .args(["-px", "com.apple.FinderInfo"])
            .arg(mount)
            .output()?;
        let mut info = [0_u8; 32];
        if old.status.success() {
            let hex: String = String::from_utf8(old.stdout)?.split_whitespace().collect();
            ensure!(hex.len() == 64, "unexpected FinderInfo length");
            for (i, byte) in info.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16)?;
            }
        }
        info[8] |= 4;
        let mut hex = String::with_capacity(64);
        for byte in info {
            write!(hex, "{byte:02x}")?;
        }
        command(
            Command::new("/usr/bin/xattr")
                .args(["-wx", "com.apple.FinderInfo"])
                .arg(hex)
                .arg(mount),
        )?;
        Ok(())
    }
    pub(super) fn build(app: &Path, background: &Path, icon: &Path, output: &Path) -> Result<()> {
        ensure!(
            app.file_name().is_some_and(|n| n == "Baylee.app") && app.is_dir(),
            "expected a staged Baylee.app"
        );
        ensure!(
            background.is_file() && icon.is_file(),
            "missing installer background or volume icon"
        );
        let retina = background.with_file_name(format!(
            "{}@2x.png",
            background
                .file_stem()
                .context("background filename")?
                .to_string_lossy()
        ));
        ensure!(
            retina.is_file(),
            "missing HiDPI background {}",
            retina.display()
        );
        let parent = output.parent().context("DMG output directory")?;
        fs::create_dir_all(parent)?;
        let scratch = tempfile::tempdir_in(parent)?;
        let writable = scratch.path().join("writable.dmg");
        let converted = scratch.path().join("finished.dmg");
        let mount_path = scratch.path().join("mount");
        fs::create_dir(&mount_path)?;
        let kib = tree_size(app)?
            .div_ceil(1024)
            .checked_mul(2)
            .and_then(|n| n.checked_add(32 * 1024))
            .context("DMG size overflow")?;
        command(
            Command::new("/usr/bin/hdiutil")
                .args(["create", "-size"])
                .arg(format!("{kib}k"))
                .args(["-fs", "HFS+", "-volname", "Baylee", "-type", "UDIF"])
                .arg(&writable),
        )?;
        command(
            Command::new("/usr/bin/hdiutil")
                .args(["attach", "-nobrowse", "-noautoopen", "-mountpoint"])
                .arg(&mount_path)
                .arg(&writable),
        )?;
        let mut mounted = Mount {
            path: mount_path,
            attached: true,
        };
        command(
            Command::new("/usr/bin/ditto")
                .arg(app)
                .arg(mounted.path.join("Baylee.app")),
        )?;
        symlink("/Applications", mounted.path.join("Applications"))?;
        command(
            Command::new("/usr/bin/tiffutil")
                .arg("-cathidpicheck")
                .arg(background)
                .arg(&retina)
                .arg("-out")
                .arg(mounted.path.join(BACKGROUND)),
        )?;
        volume_icon(&mounted.path, icon)?;
        let id = identity(&mounted.path)?;
        let store = layout::encode(id)?;
        layout::verify(&store, id)?;
        fs::write(mounted.path.join(".DS_Store"), store)?;
        // hdiutil/Finder services can create housekeeping folders, even with
        // -nobrowse. Remove only these known entries on our own mounted image.
        for name in [".Trashes", ".fseventsd", ".Spotlight-V100"] {
            let path = mounted.path.join(name);
            if path.exists() {
                fs::remove_dir_all(path)?;
            }
        }
        mounted.detach()?;
        command(
            Command::new("/usr/bin/hdiutil")
                .arg("convert")
                .arg(&writable)
                .args(["-format", "ULFO", "-o"])
                .arg(&converted),
        )?;
        command(
            Command::new("/usr/bin/hdiutil")
                .arg("verify")
                .arg(&converted),
        )?;
        fs::rename(converted, output)?;
        Ok(())
    }
    pub(super) fn check_layout(mount: &Path) -> Result<()> {
        layout::verify(&fs::read(mount.join(".DS_Store"))?, identity(mount)?)?;
        let info = Command::new("/usr/bin/xattr")
            .args(["-px", "com.apple.FinderInfo"])
            .arg(mount)
            .output()?;
        ensure!(info.status.success(), "missing custom volume icon flag");
        let hex: String = String::from_utf8(info.stdout)?.split_whitespace().collect();
        ensure!(
            hex.len() == 64 && u8::from_str_radix(&hex[16..18], 16)? & 4 != 0,
            "volume icon flag is absent"
        );
        Ok(())
    }
}

pub(super) fn build(app: &Path, background: &Path, icon: &Path, output: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        native::build(app, background, icon, output)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, background, icon, output);
        anyhow::bail!("native DMG packaging requires macOS")
    }
}
pub(super) fn check_layout(mount: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        native::check_layout(mount)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = mount;
        anyhow::bail!("mounted DMG verification requires macOS")
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests;
