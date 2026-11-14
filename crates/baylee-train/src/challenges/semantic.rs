use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};
use baylee_core::ids::PlayerId;
use baylee_engine::event::{DamageTarget, GameEvent, LossReason};
use baylee_gamehost::Session;
use sha2::{Digest, Sha256};

use super::Family;

#[derive(Default)]
pub(super) struct Trace {
    contributors: BTreeSet<(u32, bool)>,
    unsupported: bool,
    turn: u32,
    read: usize,
    observed: usize,
}

impl Trace {
    pub(super) fn observe(&mut self, session: &Session) {
        if self.turn != session.state().turn.number {
            self.turn = session.state().turn.number;
            self.read = 0;
        }
        // Snapshots retain the actual source through sacrifice and resolution.
        for damage in session.state().per_turn.damage_log.iter().skip(self.read) {
            if damage.to != DamageTarget::Player(PlayerId::new(1)) || damage.amount == 0 {
                continue;
            }
            self.observed += 1;
            let source = &damage.snapshot;
            let Some(printed) = source
                .printed_face()
                .filter(|printed| source.card.is_some_and(|card| card.index == printed.card()))
            else {
                self.unsupported = true;
                continue;
            };
            self.contributors
                .insert((printed.card().get(), damage.combat));
        }
        self.read = session.state().per_turn.damage_log.len();
    }
}

/// Conservative contribution certificate. Only card-backed damage sources are
/// recognized. Distinct copies, order, repeated attacks, overkill and mana taps
/// cannot produce new signatures. Other victory causes remain uncertified.
pub(super) fn classify(session: &Session, trace: &Trace) -> Result<(Family, [u8; 32])> {
    let state = session.state();
    ensure!(state.players[1].life <= 0, "unrecognized non-life victory");
    ensure!(
        !trace.unsupported && !trace.contributors.is_empty(),
        "unrecognized contribution source"
    );
    let mut terminal = None;
    let mut life_drop = None;
    let mut lost_to_life = false;
    let mut damage_count = 0;
    for entry in state.journal.entries() {
        match entry.event {
            GameEvent::LifeChanged {
                player, old, new, ..
            } if player == PlayerId::new(1) => {
                life_drop = Some((old, new));
                if old > 0 || new > 0 {
                    terminal = None;
                }
            }
            GameEvent::PlayerLost {
                player,
                reason: LossReason::Life,
            } if player == PlayerId::new(1) => lost_to_life = true,
            _ => {}
        }
        let GameEvent::DamageDealt {
            source: _,
            target: DamageTarget::Player(player),
            amount,
            is_combat,
        } = &entry.event
        else {
            continue;
        };
        if *player != PlayerId::new(1) || *amount == 0 {
            continue;
        }
        damage_count += 1;
        // A life-loss effect after earlier damage is not a damage finish. The
        // ordinary damage door records its matching life change before damage.
        if life_drop.take().is_some_and(|(old, new)| {
            old > 0 && new <= 0 && i64::from(old) - i64::from(new) == i64::from(*amount)
        }) {
            terminal = Some(if *is_combat {
                Family::Combat
            } else {
                Family::DirectDamage
            });
        }
    }
    ensure!(lost_to_life, "unrecognized defeat reason");
    ensure!(
        trace.observed == damage_count,
        "damage history observation was incomplete"
    );
    let family = terminal.context("no proved lethal damage family")?;
    // Swapping the last spell and attack does not create another resource route.
    let signature = serde_json::to_vec(&trace.contributors)?;
    let digest = Sha256::new()
        .chain_update(b"baylee/challenge/contributors/v1\0")
        .chain_update(signature)
        .finalize()
        .into();
    Ok((family, digest))
}
