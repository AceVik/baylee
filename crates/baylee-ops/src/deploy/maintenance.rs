//! The owner stays alive throughout the admission hold and drain. Stopping
//! an agent ends its engines, regardless of systemd's `KillMode`.
use super::{Deploy, Run, Stop, must, say, short};

impl Deploy {
    pub(super) fn admission_held(hold: &str) -> bool {
        !hold.is_empty() && Self::health().is_some_and(|(_, _, admission)| admission == "held")
    }

    /// Called only after the verified drain, or an explicit forced finish.
    /// Check the complete immutable stage before touching the live owner.
    pub(super) fn install_agent(&self, rev: &str) -> Run {
        self.owner_staged()?;
        must(&mut self.sudo(&["systemctl", "stop", "baylee-agent"]))?;
        let mut owner = vec!["baylee-agent"];
        owner.extend(super::engine::BINARIES);
        self.install_staged(&owner)?;
        for command in ["baylee-deploy", "baylee-invite", "baylee-reboot"] {
            must(&mut self.sudo(&[
                "install",
                "-m755",
                &self.p.staged(command).display().to_string(),
                &format!("/usr/local/sbin/{command}"),
            ]))?;
        }
        say(&format!(
            "{}: drained owner replaced; agent off until the gateway follows",
            short(rev)
        ));
        Ok(())
    }

    /// Whether the stage holds every owner binary: asked before anything is
    /// stopped, so a deploy that keeps games never stops the gateway for a
    /// stage it cannot install.
    pub(super) fn owner_staged(&self) -> Run {
        for name in [
            "baylee-agent",
            "baylee-deploy",
            "baylee-invite",
            "baylee-reboot",
        ]
        .into_iter()
        .chain(super::engine::BINARIES)
        {
            if !self.p.staged(name).is_file() {
                say("incomplete staged owner binaries; restage the release before finishing");
                return Err(Stop(1));
            }
        }
        Ok(())
    }
}
