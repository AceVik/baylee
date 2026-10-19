//! `cards/creatures/mv_3/farrel_s_zealot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn farrel_s_zealot() -> CardIndex {
    card_index("0d063c0e-66bc-4a9c-98aa-a381b9cea120")
}

/// Turn one: the Zealot attacks, nobody blocks, and the trigger has asked for
/// its target. Returns the engine with the `YesNo` ("you may") pending, the
/// Zealot and the chosen target.
fn unblocked_attack(target_card: CardIndex) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[farrel_s_zealot()])
        .battlefield(1, &[target_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let zealot = on_battlefield(&engine, P0, farrel_s_zealot()).expect("the Zealot is out");
    let target = on_battlefield(&engine, P1, target_card).expect("their creature is out");

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(zealot, Defender::Player(P1))],
            },
        )
        .expect("the Zealot attacks");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let menu = aim_at(&mut engine, P0, target);
    assert!(menu.contains(&target), "their creature is a legal target");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    (engine, zealot, target)
}

/// Farrel's Zealot (0d063c0e-66bc-4a9c-98aa-a381b9cea120) — "Whenever this
/// creature attacks and isn't blocked, you may have it deal 3 damage to
/// target creature. If you do, this creature assigns no combat damage this
/// turn." Accepted: three damage (not a kill) to a bigger creature, and the
/// player takes nothing.
#[test]
fn farrel_s_zealot_deals_three_to_a_creature_and_no_combat_damage() {
    let (mut engine, zealot, target) = unblocked_attack(earth_elemental());
    engine.apply(P0, PlayerAction::YesNo(true)).expect("yes");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    let hit = engine.state().object(target).expect("the 4/5 survives 3");
    assert_eq!(hit.damage, 3, "exactly three damage");
    assert_eq!(hit.zone, Zone::Battlefield);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the Zealot assigned no combat damage"
    );
    assert_eq!(
        engine.state().object(zealot).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
}

/// Three damage kills a smaller creature.
#[test]
fn farrel_s_zealot_kills_a_small_creature_with_its_three_damage() {
    let (mut engine, _zealot, _target) = unblocked_attack(gray_ogre());
    engine.apply(P0, PlayerAction::YesNo(true)).expect("yes");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(on_battlefield(&engine, P1, gray_ogre()).is_none());
    assert!(in_graveyard(&engine, P1, gray_ogre()).is_some());
    assert_eq!(engine.state().players[1].life, 20);
}

/// Declined: nothing is dealt to the creature, and the Zealot hits the player
/// for its two.
#[test]
fn farrel_s_zealot_declined_deals_its_combat_damage_to_the_player() {
    let (mut engine, _zealot, target) = unblocked_attack(earth_elemental());
    engine.apply(P0, PlayerAction::YesNo(false)).expect("no");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().object(target).expect("still there").damage,
        0
    );
    assert_eq!(engine.state().players[1].life, 18);
}
