//! `cards/enchantments/auras/mv_1/gift_of_the_woods.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn gift_of_the_woods() -> CardIndex {
    card_index("491c6f69-9b75-4d5d-b41f-a14b2fbfdd07")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn gift_of_the_woods_pumps_toughness_and_gains_life_when_enchanted_creature_becomes_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), grizzly_bears()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[gift_of_the_woods()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, gift_of_the_woods());
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

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears),
        (2, 5),
        "Gift of the Woods should give enchanted creature +0/+3 until end of turn when blocked"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "Gift of the Woods should gain 1 life when enchanted creature becomes blocked"
    );
}
