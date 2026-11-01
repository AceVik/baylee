//! Vitalizing Wind — {8}{G} — Instant
//! Oracle: Creatures you control get +7/+7 until end of turn.

use super::*;
use baylee_core::generated::index;

#[test]
fn vitalizing_wind_gives_controlled_creatures_plus_seven_plus_seven() {
    let vitalizing_wind = card_index("98b802f3-4785-41af-adfe-e77d0ae4648c");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[vitalizing_wind])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let elf_id = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on board");
    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears on board");
    let giant_id = on_battlefield(&engine, p1, index::HILL_GIANT).expect("Hill Giant on board");

    // Tap the 9 Forests manually so Llanowar Elves isn't tapped by cast_from_hand
    for &land in &all_on_battlefield(&engine, p0, forest()) {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 0,
                },
            )
            .expect("tap Forest");
    }
    let card = in_hand(&engine, p0, vitalizing_wind).expect("Vitalizing Wind in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("cast Vitalizing Wind");
    pass_until(&mut engine, stack_is_empty);

    let elf_ch = engine.state().object(elf_id).unwrap().characteristics();
    assert_eq!((elf_ch.power, elf_ch.toughness), (Some(8), Some(8)));
    let bears_ch = engine.state().object(bears_id).unwrap().characteristics();
    assert_eq!((bears_ch.power, bears_ch.toughness), (Some(9), Some(9)));
    let giant_ch = engine.state().object(giant_id).unwrap().characteristics();
    assert_eq!((giant_ch.power, giant_ch.toughness), (Some(3), Some(3)));
}
