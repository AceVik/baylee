//! `cards/creatures/mv_4/mortivore.rs`, played.
//!
//! Oracle:
//! - "Mortivore's power and toughness are each equal to the number of creature
//!   cards in all graveyards."
//! - "{B}: Regenerate this creature."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mortivore() -> CardIndex {
    card_index("2f554218-2072-4d9b-b4d4-34f161f1f2a9")
}

fn cda_pt(engine: &Engine<RegistryLookup>, id: ObjectId) -> (i16, i16) {
    let chars =
        baylee_engine::layers::recompute(engine.state(), engine.state().object(id).unwrap())
            .characteristics;
    (chars.power.unwrap_or(0), chars.toughness.unwrap_or(0))
}

fn tap_swamps(engine: &mut Engine<RegistryLookup>, count: usize) {
    let swamps = all_on_battlefield(engine, P0, swamp());
    for &s in &swamps[..count] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
}

#[test]
fn mortivore_counts_creatures_in_all_graveyards_and_regenerates() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
            ],
        )
        .battlefield(1, &[index::LLANOWAR_ELVES])
        .hand(
            0,
            &[
                mortivore(),
                index::DARK_BANISHING,
                lightning_bolt(),
                index::GRIZZLY_BEARS,
                index::SHOCK,
                bojuka_bog(),
            ],
        )
        .hand(1, &[index::SAVANNAH_LIONS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // 1 creature in P0 yard + 1 creature in P1 yard; noncreature (Shock) & live Elves do not count.
    hand_to_graveyard(&mut engine, P0, index::GRIZZLY_BEARS);
    hand_to_graveyard(&mut engine, P0, index::SHOCK);
    hand_to_graveyard(&mut engine, P1, index::SAVANNAH_LIONS);
    let mort_hand = in_hand(&engine, P0, mortivore()).unwrap();
    assert_eq!(cda_pt(&engine, mort_hand), (2, 2));

    // Cast Mortivore with 4 Swamps -> 2/2.
    tap_swamps(&mut engine, 4);
    cast_with_floating(&mut engine, P0, mortivore());
    pass_until(&mut engine, stack_is_empty);
    let mort = on_battlefield(&engine, P0, mortivore()).unwrap();
    assert_eq!(pt(&engine, mort), (2, 2));

    // Kill opponent's Llanowar Elves with Dark Banishing -> grows dynamically to 3/3.
    let swamps = all_on_battlefield(&engine, P0, swamp());
    for &s in &swamps[4..7] {
        engine
            .apply(P0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    let elves = on_battlefield(&engine, P1, index::LLANOWAR_ELVES).unwrap();
    cast_with_floating(&mut engine, P0, index::DARK_BANISHING);
    aim_at(&mut engine, P0, elves);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, mort), (3, 3));

    // Activate {B}: Regenerate, then Lightning Bolt (3 lethal damage) -> shield saves & taps Mortivore.
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: swamps[7] })
        .unwrap();
    activate(&mut engine, P0, mortivore(), 1);
    pass_until(&mut engine, stack_is_empty);

    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: mtn })
        .unwrap();
    cast_with_floating(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, mort);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, mortivore()).is_some());
    assert!(is_tapped(&engine, mort));
    assert_eq!(engine.state().object(mort).unwrap().damage, 0);

    // Exiling P1's graveyard removes 2 creatures -> shrinks 3/3 -> 1/1.
    play_land(&mut engine, P0, bojuka_bog());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, mort), (1, 1));
}
