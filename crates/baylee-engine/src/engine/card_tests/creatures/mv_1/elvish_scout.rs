//! `cards/creatures/mv_1/elvish_scout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn fill_pools(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

/// Player 0 attacks with the Bears, player 1 blocks with a Hill Giant (3/3).
/// Stops in the declare-blockers step with player 0 holding priority.
fn bears_blocked_by_giant() -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card_index("d39a492e-a599-4283-bfc1-8e99065a9303"),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let scout = on_battlefield(&e, P0, card_index("d39a492e-a599-4283-bfc1-8e99065a9303"))
        .expect("the Scout is out");
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("the Bears are out");
    let giant = on_battlefield(&e, P1, hill_giant()).expect("the Giant is out");

    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareAttackers);
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P1))],
        },
    )
    .expect("the Bears attack");
    pass_until(&mut e, |e| e.state().turn.step == Step::DeclareBlockers);
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(giant, bears)],
        },
    )
    .expect("the Giant blocks");
    (e, scout, bears, giant)
}

/// Elvish Scout (d39a492e-a599-4283-bfc1-8e99065a9303) — "{G}, {T}: Untap
/// target attacking creature you control. Prevent all combat damage that
/// would be dealt to and dealt by it this turn."
#[test]
fn elvish_scout_untaps_the_attacker_and_prevents_combat_damage_both_ways() {
    let (mut e, scout, bears, giant) = bears_blocked_by_giant();
    assert!(
        e.state()
            .object(bears)
            .expect("the Bears are out")
            .status
            .contains(Status::TAPPED),
        "attacking tapped the Bears"
    );

    fill_pools(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: scout,
            ability_index: 0,
        },
    )
    .expect("{G}, {T} is paid");
    let menu = aim_at(&mut e, P0, bears);
    assert_eq!(menu, vec![bears], "the only attacking creature of mine");
    pass_until(&mut e, stack_is_empty);
    assert!(
        !e.state()
            .object(bears)
            .expect("the Bears are out")
            .status
            .contains(Status::TAPPED),
        "untapped by the Scout"
    );
    assert!(
        e.state()
            .object(scout)
            .expect("the Scout is out")
            .status
            .contains(Status::TAPPED),
        "the Scout paid {{T}}"
    );

    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    let bears_now = e.state().object(bears).expect("the Bears are out");
    assert_eq!(
        bears_now.zone,
        Zone::Battlefield,
        "the Giant's damage was prevented"
    );
    assert_eq!(bears_now.damage, 0);
    assert_eq!(
        e.state().object(giant).expect("the Giant is out").damage,
        0,
        "the Bears' damage was prevented too"
    );
}

#[test]
fn elvish_scout_unused_leaves_the_blocked_bears_to_die() {
    let (mut e, _scout, _bears, giant) = bears_blocked_by_giant();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_none(),
        "3 damage kills the Bears"
    );
    assert_eq!(
        e.state().object(giant).expect("the Giant is out").damage,
        2,
        "and the Bears hit back"
    );
}

#[test]
fn elvish_scout_has_no_target_before_anything_attacks() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card_index("d39a492e-a599-4283-bfc1-8e99065a9303"),
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let scout = on_battlefield(&e, P0, card_index("d39a492e-a599-4283-bfc1-8e99065a9303"))
        .expect("the Scout is out");
    fill_pools(&mut e);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority")
    };
    assert!(
        !legal.abilities.contains(&(scout, 0)),
        "no attacking creature, no activation"
    );
}
