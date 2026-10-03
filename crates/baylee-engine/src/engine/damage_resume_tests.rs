//! Suspension, concession and replay at the shared damage continuation.

use super::*;
use crate::choice::{DamageEffectKind, PlayerAction};
use crate::damage::{Assignment, DamageWork};
use crate::engine::synthetic::{self, SyntheticLookup};
use crate::event::DamageTarget;
use crate::prevention::{Shield, ShieldKind, Shielded};
use baylee_cards_dsl::{AbilityDef, Amount, Effect, PlayerRel, TargetSpec};

const SOURCE: u32 = 98_090;
const A: PlayerId = PlayerId::new(0);
const B: PlayerId = PlayerId::new(1);
const C: PlayerId = PlayerId::new(2);
const D: PlayerId = PlayerId::new(3);

fn fixture() -> Engine<SyntheticLookup> {
    static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!("", TapSelf),
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::Player(PlayerRel::EachOpponent),
            },
            Effect::GainLife {
                amount: Amount::Fixed(1),
            },
        ]
    )];
    let definition = synthetic::land(SOURCE, "Damage continuation probe", ABILITIES);
    let mut preset = synthetic::preset(98, &[SOURCE]);
    preset.seats.push(preset.seats[1].clone());
    preset.seats.push(preset.seats[1].clone());
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition])).unwrap();
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == A),
    );
    engine
}

fn walk_until(
    engine: &mut Engine<SyntheticLookup>,
    ready: impl Fn(&Engine<SyntheticLookup>) -> bool,
) {
    for _ in 0..100 {
        if ready(engine) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected {pending:?}"
        );
    }
    panic!("condition not reached: {:?}", engine.pending());
}

fn shield(engine: &mut Engine<SyntheticLookup>, player: PlayerId, n: u32) {
    engine.state.shields.push(Shield {
        protects: Shielded::Player(player),
        kind: ShieldKind::Next(n),
        controller: player,
    });
}

fn open_resolution() -> Engine<SyntheticLookup> {
    let mut engine = fixture();
    for player in [B, C] {
        shield(&mut engine, player, 1);
        shield(&mut engine, player, 1);
    }
    let source = synthetic::permanents(&engine, SOURCE)[0];
    engine.refresh_offer();
    engine
        .apply(
            A,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageEffect { .. })
    });
    assert_eq!(engine.pending().asked(), Some(B));
    engine
}

fn first_shield(engine: &Engine<SyntheticLookup>) -> (PlayerId, PlayerAction) {
    let Pending::ChooseDamageEffect {
        player,
        choice,
        options,
        ..
    } = engine.pending()
    else {
        panic!("effect choice: {:?}", engine.pending());
    };
    let option = options
        .iter()
        .find(|o| matches!(o.kind, DamageEffectKind::PreventNext { .. }))
        .unwrap();
    (
        *player,
        PlayerAction::ChooseDamageEffect {
            choice: *choice,
            effect: option.id,
        },
    )
}

#[test]
fn damage_answers_replay_identically_and_stale_answers_are_atomic() {
    let mut played = open_resolution();
    let mut replay = open_resolution();
    assert_eq!(played.snapshot_hash(), replay.snapshot_hash());
    let (player, first) = first_shield(&played);
    played.apply(player, first.clone()).unwrap();
    replay.apply(player, first.clone()).unwrap();
    assert_eq!(played.snapshot_hash(), replay.snapshot_hash());
    assert_eq!(played.fingerprint(), replay.fingerprint());
    assert_eq!(played.pending().asked(), Some(C));
    assert!(played.state.players.iter().all(|p| p.life == 20));
    let before = played.fingerprint();
    let snapshot = played.snapshot_hash();
    assert!(played.apply(C, first).is_err());
    assert_eq!(played.fingerprint(), before);
    assert_eq!(played.snapshot_hash(), snapshot);
    let (player, second) = first_shield(&played);
    played.apply(player, second.clone()).unwrap();
    replay.apply(player, second).unwrap();
    assert_eq!(played.snapshot_hash(), replay.snapshot_hash());
    assert_eq!(played.fingerprint(), replay.fingerprint());
    assert!(played.resolution.is_none());
    assert_eq!(
        played
            .state
            .players
            .iter()
            .map(|p| p.life)
            .collect::<Vec<_>>(),
        vec![21, 19, 19, 17]
    );
}

#[test]
fn concession_refreshes_damage_decisions_without_restarting_the_instruction() {
    for leaver in [A, B, D] {
        let mut engine = open_resolution();
        let (_, obsolete) = first_shield(&engine);
        engine.apply(leaver, PlayerAction::Concede).unwrap();
        assert!(engine.resolution.is_some());
        let chooser = if leaver == B { C } else { B };
        assert_eq!(engine.pending().asked(), Some(chooser));
        let before = engine.fingerprint();
        assert!(engine.apply(chooser, obsolete).is_err());
        assert_eq!(engine.fingerprint(), before);
        while matches!(engine.pending(), Pending::ChooseDamageEffect { .. }) {
            let (player, answer) = first_shield(&engine);
            engine.apply(player, answer).unwrap();
        }
        assert!(engine.resolution.is_none());
        assert_eq!(
            engine.state.players[0].life,
            if leaver == A { 20 } else { 21 },
            "following instruction happens once and preserves departed-player LKI"
        );
        assert_eq!(
            engine.state.players[2].life, 19,
            "one damage, once, then next instruction"
        );
        assert_eq!(
            engine.state.players[1].life,
            if leaver == B { 20 } else { 19 }
        );
        assert_eq!(
            engine.state.players[3].life,
            if leaver == D { 20 } else { 17 }
        );
    }
}

#[test]
fn a_recipient_conceding_during_combat_allocation_drops_only_its_damage() {
    let mut engine = fixture();
    let source = synthetic::permanents(&engine, SOURCE)[0];
    let name = engine.state.names.intern("Second damage source");
    let other = engine.state.create_bare(
        A,
        crate::object::ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    shield(&mut engine, B, 2);
    let assignment = |source, recipient, amount| Assignment {
        source,
        source_version: None,
        recipient: DamageTarget::Player(recipient),
        amount,
        is_combat: true,
    };
    let mut work = DamageWork::new(
        &mut engine.state,
        vec![
            assignment(source, B, 2),
            assignment(other, B, 3),
            assignment(source, C, 3),
        ],
    );
    let pending = work.advance(&mut engine.state).unwrap();
    assert!(matches!(
        pending,
        Pending::AllocatePrevention { player: B, .. }
    ));
    engine.combat_damage = Some(work);
    engine.pending = pending;
    engine.awaiting_answer = true;
    engine.apply(B, PlayerAction::Concede).unwrap();
    assert!(engine.combat_damage.is_none());
    assert_eq!(engine.state.players[1].life, 20);
    assert_eq!(engine.state.players[2].life, 17);
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
}

#[test]
fn departed_player_life_is_last_known_information_and_changes_record_nothing() {
    let mut engine = fixture();
    engine.apply(B, PlayerAction::Concede).unwrap();
    let before = engine.fingerprint();
    let sequence = engine.state.journal.last_seq();
    for amount in [-7, 0, 9] {
        engine
            .state
            .change_life(B, amount, crate::event::Cause::Effect);
    }
    assert_eq!(engine.state.players[1].life, 20);
    assert_eq!(
        engine.state.journal.last_seq(),
        sequence,
        "no life event from a departed seat"
    );
    assert_eq!(
        engine.fingerprint(),
        before,
        "including per-turn life-loss history"
    );
}
