//! `cards/instants/mv_3/fevered_strength.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fevered_strength() -> CardIndex {
    card_index("52512d7a-9b1c-4475-80aa-c95f4efa1015")
}

#[test]
fn fevered_strength_pumps_power_by_two_and_draws_on_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), grizzly_bears()])
        .hand(0, &[fevered_strength()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, fevered_strength());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears),
        (4, 2),
        "Fevered Strength should give target creature +2/+0 until end of turn"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "Fevered Strength should draw a card at the beginning of the next turn's upkeep"
    );
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// "Target creature": a land is refused, and the other player's creature is a target like any other
/// (+2/+0), while the card drawn at the next upkeep is the caster's and not the creature's
/// controller's.
#[test]
fn fevered_strength_targets_any_creature_but_draws_for_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[fevered_strength()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let land = on_battlefield(&engine, p0, swamp()).unwrap();
    let hand_size =
        |e: &Engine<RegistryLookup>, seat| e.state().zones.list(ZoneLocation::Hand(seat)).len();
    let (mine, theirs) = (hand_size(&engine, p0), hand_size(&engine, p1));

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, fevered_strength());
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![land],
                    players: vec![],
                },
            )
            .is_err(),
        "a land is no creature"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (5, 3));

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine, p0),
        mine,
        "the caster drew, replacing the spell"
    );
    assert_eq!(
        hand_size(&engine, p1),
        theirs,
        "the creature's controller did not"
    );
}

/// "Until end of turn": the +2/+0 is gone in the next turn, and the card is drawn once only (the
/// turn after brings just the ordinary draw).
#[test]
fn fevered_strength_pump_ends_with_the_turn_and_the_card_comes_once() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), grizzly_bears()])
        .hand(0, &[fevered_strength()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, fevered_strength());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (4, 2));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bears), (2, 2));
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1,
        "spell out, one card in, and the ordinary draw: no second Fevered Strength draw"
    );
}
