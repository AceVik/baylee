//! A deploy that keeps the running games instead of waiting for them to end
//! (`docs/deploy-hooks.md` §"Keeping games across a deploy",
//! `docs/proposals/games-survive-restarts.md`). Off unless
//! `BAYLEE_DEPLOY_KEEP_GAMES=1` stands in `/etc/baylee/deploy.env` and the
//! agent's unit reads the list of engines this writes.
//!
//! A gateway that stops stores every running game's record and keeps the
//! game in its database; started again, it resumes each on the engine build
//! that began it. So the deploy, once every running game is kept:
//!
//! 1. copies the engines now installed to `engines/<deployed rev>/` when a
//!    game plays on them, before the new ones replace them;
//! 2. stops the gateway first, so the games are stored before the agent's
//!    engines go;
//! 3. replaces the agent and the gateway as before, and names the current
//!    engines and every kept build in `agent-engines.env`
//!    (`BAYLEE_ENGINE_BINS`);
//! 4. once the new gateway answers, removes the kept builds no game plays on.
//!
//! It waits as before, and says why, while a running game would not be kept,
//! while one has a seat bridge (bridges are replaced with the release and
//! speak its protocol), while a game plays on a build that is no longer
//! installed or that the release does not descend from (a rollback: an agent
//! refuses a newer protocol than its own, and would start none at all),
//! while the agent's sixteen engines would not hold them all, and once a hook
//! stopped keeping games for this release.
//!
//! Only the two standard engines of an older build are kept: a Tower run is
//! never kept, and an older Tower build advertised beside the current one
//! could be given a new run.
use super::{Deploy, Run, Stop, engine, must, quiet, read_trim, say, short};
use crate::gateway;
use std::{fs, path::PathBuf, process::Command};

/// How many engine executables an agent takes (`MAX_CAPABILITIES`).
const AGENT_ENGINES: usize = 16;

/// The engines of an older build that are kept: the two standard ones.
const KEPT: [&str; 2] = [engine::BINARIES[0], engine::BINARIES[1]];

/// Beside the stage: a hook stopped keeping games for this release, so its
/// deploy waits for them as before rather than stop the gateway again.
const DECLINED: &str = "keep-declined";

impl Deploy {
    /// Whether this deploy keeps games at all: the setting, and an agent
    /// unit that reads the list of engines (else the kept builds would be
    /// installed for nothing, and the games would end anyway).
    pub(super) fn keeps_games(&self) -> bool {
        if !self.p.keep_games {
            return false;
        }
        let path = self.p.agent_engines_env.display().to_string();
        let reads = crate::sh::out_quiet(Command::new("systemctl").args([
            "show",
            "baylee-agent",
            "-p",
            "EnvironmentFiles",
            "--value",
        ]))
        .is_some_and(|files| files.split_whitespace().any(|file| file == path));
        if !reads {
            say(&format!(
                "BAYLEE_DEPLOY_KEEP_GAMES is set, but the agent's unit does not read {path}: running games are waited for"
            ));
        }
        reads
    }

    /// The revisions of the kept games' engine builds, when every running
    /// game can be kept across replacing `rev`'s gateway; `None` when the
    /// deploy waits for them as before (said why), or when nothing runs.
    pub(super) fn keeping(&self, rev: &str) -> Option<Vec<String>> {
        let games = gateway::fetch("health", 5).and_then(|body| gateway::kept(&body))?;
        if games.running == 0 {
            return None;
        }
        let wait = |why: &str| {
            say(&format!("waiting for {}: {why}", short(rev)));
            None
        };
        if self.p.staged(DECLINED).exists() {
            return wait("a deploy hook stopped keeping games for this release");
        }
        if games.kept < games.running {
            return wait(&format!(
                "{} of {} running game(s) would not be kept",
                games.running - games.kept,
                games.running
            ));
        }
        if games.bridged > 0 {
            return wait(&format!(
                "{} game(s) with a seat bridge, which the release replaces",
                games.bridged
            ));
        }
        let deployed = self.deployed_rev();
        let kept: Vec<&String> = games.revisions.iter().filter(|r| *r != rev).collect();
        if let Some(gone) = kept
            .iter()
            .find(|r| Some(r.as_str()) != deployed.as_deref() && !self.kept_build(r).is_dir())
        {
            return wait(&format!(
                "a game plays on {}, whose engines are no longer installed",
                short(gone)
            ));
        }
        if let Some(newer) = kept.iter().find(|r| {
            !quiet(&mut self.in_src(&["git", "merge-base", "--is-ancestor", r.as_str(), rev]))
        }) {
            return wait(&format!(
                "a game plays on {}, which this release does not descend from",
                short(newer)
            ));
        }
        if engine::BINARIES.len() + KEPT.len() * kept.len() > AGENT_ENGINES {
            return wait(&format!(
                "{} older builds would not fit the agent's {AGENT_ENGINES} engines",
                kept.len()
            ));
        }
        say(&format!(
            "{}: keeping {} running game(s) across the switch",
            short(rev),
            games.running
        ));
        Some(games.revisions)
    }

    /// Before the switch: keeps the engines now installed when a kept game
    /// plays on them, and stops the gateway, which stores every game.
    pub(super) fn before_keeping(&self, revisions: &[String], rev: &str) -> Run {
        if let Some(deployed) = self.deployed_rev()
            && deployed != rev
            && revisions.contains(&deployed)
        {
            let into = self.kept_build(&deployed).display().to_string();
            must(&mut self.sudo(&["install", "-d", "-m755", &into]))?;
            for name in KEPT {
                let from = self.p.bin.join(name).display().to_string();
                must(&mut self.sudo(&["install", "-m755", &from, &format!("{into}/")]))?;
            }
            say(&format!(
                "engines of {} kept for its games",
                short(&deployed)
            ));
        }
        must(&mut self.sudo(&["systemctl", "stop", "baylee-gateway"]))?;
        say("gateway stopped: its running games are stored and kept");
        Ok(())
    }

    /// A deploy hook stopped a keeping deploy before anything was installed:
    /// the old gateway and agent start again, and the games they stored go
    /// on where they were.
    pub(super) fn resume_where_it_was(&self) {
        // Once: the next run waits for the games instead (`keeping`).
        let _ = fs::write(self.p.staged(DECLINED), b"");
        for unit in ["baylee-gateway", "baylee-agent"] {
            if must(&mut self.sudo(&["systemctl", "start", unit])).is_err() {
                say(&format!(
                    "{unit} did not start again; its games wait for it"
                ));
            }
        }
        say("the deploy stopped: the old gateway and agent run again");
    }

    /// Names the current engines and the kept builds of `revisions` in the
    /// agent's list, before it starts.
    pub(super) fn name_engines(&self, revisions: &[String], rev: &str) -> Run {
        let mut bins: Vec<PathBuf> = engine::BINARIES[1..]
            .iter()
            .map(|name| self.p.bin.join(name))
            .collect();
        for kept in revisions.iter().filter(|r| *r != rev) {
            bins.extend(KEPT.iter().map(|name| self.kept_build(kept).join(name)));
        }
        let list: Vec<String> = bins.iter().map(|bin| bin.display().to_string()).collect();
        let text = format!(
            "# Written by baylee-deploy (docs/deploy-hooks.md): this release's engines\n\
             # beside baylee-engine-server, and the older builds kept for running games.\n\
             BAYLEE_ENGINE_BINS={}\n",
            list.join(":")
        );
        let staged = self.p.state.join("agent-engines.env");
        fs::write(&staged, text).map_err(|_| Stop(1))?;
        must(&mut self.sudo(&[
            "install",
            "-m644",
            &staged.display().to_string(),
            &self.p.agent_engines_env.display().to_string(),
        ]))
    }

    /// After the switch: removes every kept build no game of `revisions`
    /// plays on.
    pub(super) fn prune_engines(&self, revisions: &[String]) {
        let Ok(entries) = fs::read_dir(&self.p.engines) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let a_revision = matches!(name.len(), 40 | 64)
                && name
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            if a_revision && !revisions.contains(&name) {
                let path = entry.path().display().to_string();
                if must(&mut self.sudo(&["rm", "-rf", &path])).is_ok() {
                    say(&format!(
                        "engines of {} removed: no game plays on them",
                        short(&name)
                    ));
                }
            }
        }
    }

    fn kept_build(&self, rev: &str) -> PathBuf {
        self.p.engines.join(rev)
    }

    fn deployed_rev(&self) -> Option<String> {
        Some(read_trim(&self.p.state.join("deployed"))).filter(|rev| !rev.is_empty())
    }
}
