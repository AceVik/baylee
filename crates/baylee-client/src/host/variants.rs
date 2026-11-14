//! Choose a concrete local engine once, outside its rules and AI hot paths.
use super::{GamePreset, LocalHost, PlayerId};
use baylee_core::preset::LoopPolicy;
use baylee_engine::engine::{LoopDetection, NoLoopDetection};

impl LocalHost {
    /// Legacy enabled-policy constructor for existing local integrations and fixtures.
    #[must_use]
    pub fn new(preset: &GamePreset, seat: PlayerId, names: &[&str]) -> Option<Self> {
        Self::with_detection(preset, seat, names)
    }

    /// Rebuilds an enabled-policy legacy local game.
    ///
    /// # Errors
    /// A malformed record, wrong variant or replay mismatch.
    pub fn resume(
        record: Vec<u8>,
        name: &str,
        seat: PlayerId,
        names: &[String],
    ) -> Result<Self, String> {
        Self::resume_with_detection(record, name, seat, names)
    }

    /// Installs the statically typed engine requested by a new table.
    #[must_use]
    pub fn install(
        preset: &GamePreset,
        seat: PlayerId,
        names: &[&str],
    ) -> Option<crate::InstalledHost> {
        match preset.house_rules.loop_policy {
            LoopPolicy::Off => LocalHost::<NoLoopDetection>::with_detection(preset, seat, names)
                .map(LocalHost::into_installed),
            LoopPolicy::RunOnceThenBreak | LoopPolicy::CompRulesDraw => {
                Self::new(preset, seat, names).map(LocalHost::into_installed)
            }
        }
    }

    /// Selects the recorded variant, checks the restart hash, then starts the worker.
    ///
    /// # Errors
    /// A malformed record, unsupported variant or state that differs from the restart.
    pub fn install_resumed(
        record: Vec<u8>,
        name: &str,
        seat: PlayerId,
        names: &[String],
        hash: &str,
    ) -> Result<crate::InstalledHost, String> {
        let first = record
            .split(|byte| *byte == b'\n')
            .next()
            .ok_or("the record has no header")?;
        let baylee_gamehost::record::Line::Header { preset, .. } =
            serde_json::from_slice(first).map_err(|_| "the record header is invalid")?
        else {
            return Err("the record has no header".into());
        };
        match preset.house_rules.loop_policy {
            LoopPolicy::Off => resume::<NoLoopDetection>(record, name, seat, names, hash),
            LoopPolicy::RunOnceThenBreak | LoopPolicy::CompRulesDraw => {
                resume::<super::DetectLoops>(record, name, seat, names, hash)
            }
        }
    }
}

fn resume<D: LoopDetection>(
    record: Vec<u8>,
    name: &str,
    seat: PlayerId,
    names: &[String],
    hash: &str,
) -> Result<crate::InstalledHost, String> {
    let host = LocalHost::<D>::resume_with_detection(record, name, seat, names)?;
    if host.hash() != hash {
        return Err("the rebuilt game differs from the restart".into());
    }
    Ok(host.into_installed())
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::host::{DuelHost, HostMessage};
    use baylee_core::preset::SeatController;
    use baylee_engine::choice::PlayerAction;
    use std::time::{Duration, Instant};

    fn settled(host: &mut dyn DuelHost) -> Vec<HostMessage> {
        let until = Instant::now() + Duration::from_secs(10);
        let mut messages = Vec::new();
        loop {
            messages.extend(host.poll());
            if !host.busy() {
                return messages;
            }
            assert!(
                Instant::now() < until,
                "local variant worker did not settle"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn installed_variants_play_and_resume_with_the_frozen_policy_and_hash() {
        for policy in [
            LoopPolicy::Off,
            LoopPolicy::RunOnceThenBreak,
            LoopPolicy::CompRulesDraw,
        ] {
            let mut preset = crate::host::tests::duel_preset();
            preset.house_rules.loop_policy = policy;
            // No unrelated house search: exercise the renderer's actual worker boundary.
            preset.seats[1].controller = SeatController::Open;
            let seat = PlayerId::new(0);
            let mut host = LocalHost::install(&preset, seat, &["You", "Guest"]).unwrap();
            assert!(
                settled(&mut *host.0)
                    .iter()
                    .any(|m| matches!(m, HostMessage::Choice(_)))
            );
            host.0.submit(PlayerAction::MulliganKeep);
            settled(&mut *host.0);
            let point = host.0.resume_point().unwrap();
            let record = host.0.local_record().unwrap().to_vec();
            drop(host);
            assert!(
                LocalHost::install_resumed(
                    record.clone(),
                    &point.record,
                    seat,
                    &point.names,
                    "wrong hash",
                )
                .is_err()
            );
            let mut resumed =
                LocalHost::install_resumed(record, &point.record, seat, &point.names, &point.hash)
                    .unwrap();
            settled(&mut *resumed.0);
            assert_eq!(resumed.0.resume_point().unwrap().hash, point.hash);
            resumed.0.submit(PlayerAction::Concede);
            let end = settled(&mut *resumed.0);
            assert!(!end.iter().any(|m| matches!(m, HostMessage::Failed(_))));
            assert!(end.iter().any(|m| matches!(m, HostMessage::Choice(p)
                if matches!(**p, baylee_engine::choice::Pending::GameOver(_)))));
        }
    }
}
