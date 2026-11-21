//! `cards/creatures/mv_4/false_prophet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn false_prophet() -> CardIndex {
    card_index("87c44c23-f209-494a-ab66-c9f5cf7bb4ea")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn in_exile_card(engine: &Engine<RegistryLookup>, owner: PlayerId, card: CardIndex) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(owner))
        .iter()
        .any(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// False Prophet (87c44c23-f209-494a-ab66-c9f5cf7bb4ea) — `{2}{W}{W}` 2/2
/// Creature — Human Cleric:
/// "When this creature dies, exile all creatures."
#[test]
fn false_prophet_exiles_all_creatures_on_both_sides_when_it_dies() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                grizzly_bears(),
                sol_ring(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .hand(0, &[false_prophet(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, Some(mountain()));
    cast_with_floating(&mut engine, P0, false_prophet());
    pass_until(&mut engine, stack_is_empty);

    let prophet = on_battlefield(&engine, P0, false_prophet()).expect("False Prophet entered");
    assert_eq!(pt(&engine, prophet), (2, 2));

    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, prophet);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, false_prophet()).is_some());
    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_none());
    assert!(on_battlefield(&engine, P1, hill_giant()).is_none());
    assert!(
        in_exile_card(&engine, P0, grizzly_bears()) && in_exile_card(&engine, P1, hill_giant()),
        "all creatures on the battlefield are exiled, not destroyed"
    );
    assert!(
        on_battlefield(&engine, P0, sol_ring()).is_some(),
        "noncreature permanents remain on the battlefield"
    );
}

#[test]
fn false_prophet_exiled_directly_does_not_die_and_leaves_creatures_alone() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[false_prophet(), grizzly_bears(), plains()])
        .battlefield(1, &[hill_giant()])
        .hand(0, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let prophet = on_battlefield(&engine, P0, false_prophet()).unwrap();
    cast_from_hand(&mut engine, P0, swords_to_plowshares());
    aim_at(&mut engine, P0, prophet);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_exile_card(&engine, P0, false_prophet()));
    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, P1, hill_giant()).is_some());
}
