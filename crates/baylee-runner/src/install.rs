//! Installation of the native runner as the user's `LaunchAgent`.

use crate::process::{Operations, Spec, require_success};
use anyhow::{Context, Result};
use std::path::Path;

const PLIST: &str = include_str!("../../../scripts/macos-runner/org.baylee.macos-runner.plist");
const LABEL: &str = "org.baylee.macos-runner";

fn plist_text(home: &Path) -> String {
    let home = home
        .to_string_lossy()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    PLIST.replace("@HOME@", &home)
}

/// Copies the current native binary and loads the user's `LaunchAgent`.
pub fn install_agent(
    operations: &mut dyn Operations,
    home: &Path,
    current_exe: &Path,
) -> Result<()> {
    let directory = home.join("baylee-ci");
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("creating {}", directory.display()))?;
    let installed = directory.join("baylee-macos-runner");
    let replacement =
        tempfile::NamedTempFile::new_in(&directory).context("staging the native runner")?;
    std::fs::copy(current_exe, replacement.path()).with_context(|| {
        format!(
            "copying {} to {}",
            current_exe.display(),
            installed.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(replacement.path(), std::fs::Permissions::from_mode(0o755))
            .context("making the runner executable")?;
    }

    replacement
        .persist(&installed)
        .context("replacing the installed runner")?;
    std::fs::create_dir_all(home.join("Library/Logs")).context("creating the log directory")?;

    let agents = home.join("Library/LaunchAgents");
    std::fs::create_dir_all(&agents).with_context(|| format!("creating {}", agents.display()))?;
    let plist = agents.join(format!("{LABEL}.plist"));
    std::fs::write(&plist, plist_text(home))
        .with_context(|| format!("writing {}", plist.display()))?;

    let output = operations.output(Spec::new("id", ["-u"]))?;
    require_success(&output, "id -u")?;
    let uid = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let domain = format!("gui/{uid}");
    let plist_arg = plist.to_string_lossy().to_string();
    let _ = operations.output(Spec::new("launchctl", ["bootout", &domain, &plist_arg]));
    let output = operations.output(Spec::new("launchctl", ["bootstrap", &domain, &plist_arg]))?;
    require_success(&output, "launchctl bootstrap")?;
    crate::log(format!("loaded {}", plist.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::test_support::{MockOperations, output};
    use std::path::PathBuf;

    #[test]
    fn install_copies_the_binary_and_writes_the_native_plist() {
        let home = tempfile::tempdir().unwrap();
        let source = home.path().join("source-binary");
        std::fs::write(&source, b"native runner").unwrap();
        let uid = "501";
        let domain = format!("gui/{uid}");
        let plist = home
            .path()
            .join("Library/LaunchAgents/org.baylee.macos-runner.plist")
            .to_string_lossy()
            .to_string();
        let mut operations = MockOperations::default();
        operations.push_output(Spec::new("id", ["-u"]), output(0, "501\n"));
        operations.push_output(
            Spec::new("launchctl", ["bootout", &domain, &plist]),
            output(1, "not loaded"),
        );
        operations.push_output(
            Spec::new("launchctl", ["bootstrap", &domain, &plist]),
            output(0, ""),
        );
        install_agent(&mut operations, home.path(), &source).unwrap();
        operations.assert_done();

        let installed = home.path().join("baylee-ci/baylee-macos-runner");
        assert_eq!(std::fs::read(&installed).unwrap(), b"native runner");
        assert!(home.path().join("Library/Logs").is_dir());
        // Reinstalling via the installed executable must not truncate its source.
        operations.push_output(Spec::new("id", ["-u"]), output(0, "501\n"));
        operations.push_output(
            Spec::new("launchctl", ["bootout", &domain, &plist]),
            output(0, ""),
        );
        operations.push_output(
            Spec::new("launchctl", ["bootstrap", &domain, &plist]),
            output(0, ""),
        );
        install_agent(&mut operations, home.path(), &installed).unwrap();
        assert_eq!(std::fs::read(&installed).unwrap(), b"native runner");
        operations.assert_done();
        let text = std::fs::read_to_string(
            home.path()
                .join("Library/LaunchAgents/org.baylee.macos-runner.plist"),
        )
        .unwrap();
        assert!(text.contains("/baylee-ci/baylee-macos-runner"));
        assert!(!text.contains("runner.sh"));
        assert!(!text.contains("@HOME@"));
    }

    #[test]
    fn plist_escapes_home_for_xml() {
        let text = plist_text(Path::new("/Users/A&B<C>"));
        assert!(text.contains("/Users/A&amp;B&lt;C&gt;/baylee-ci"));
    }

    #[test]
    fn plist_home_substitution_is_exact() {
        let home = PathBuf::from("/Users/example");
        let text = plist_text(&home);
        assert!(text.contains("/Users/example/baylee-ci/baylee-macos-runner"));
        assert!(!text.contains("@HOME@"));
    }
}
