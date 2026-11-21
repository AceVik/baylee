//! `cards/instants/mv_2/nightcreep.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::{index, subtypes};
use baylee_core::types::SubtypeSet;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "8b4b677e-3b18-454c-b385-b3ffb65f916e"
fn nightcreep() -> CardIndex {
    card_index("8b4b677e-3b18-454c-b385-b3ffb65f916e")
}

fn obj_colors(engine: &Engine<RegistryLookup>, id: ObjectId) -> ColorSet {
    engine.state().object(id).unwrap().characteristics().colors
}

fn obj_subtypes(engine: &Engine<RegistryLookup>, id: ObjectId) -> SubtypeSet {
    engine
        .state()
        .object(id)
        .unwrap()
        .characteristics()
        .subtypes
}

/// Nightcreep ({B}{B}): until end of turn, all creatures (yours and opponent's)
/// become black (losing prior colors) and all lands become Swamps (tapping only
/// for {B}), then revert at cleanup.
#[test]
fn nightcreep_makes_all_creatures_black_and_all_lands_swamps_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, forest()],
        &[index::SERRA_ANGEL, plains()],
        &[nightcreep()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let my_forest = lk::obj(&engine, P0, forest());
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);
    let their_plains = lk::obj(&engine, P1, plains());

    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, nightcreep());
    lk::settle(&mut engine);

    let black = ColorSet::from_slice(&[Color::Black]);
    let swamp_sub = SubtypeSet::from_slice(&[subtypes::land::SWAMP]);
    assert_eq!(obj_colors(&engine, bears), black);
    assert_eq!(obj_colors(&engine, angel), black);
    assert_eq!(obj_subtypes(&engine, my_forest), swamp_sub);
    assert_eq!(obj_subtypes(&engine, their_plains), swamp_sub);

    // Forest now taps for {B}, not {G}.
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: my_forest })
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.available(ManaColor::Green), 0);

    // Next turn: colors and land types have reverted.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        obj_colors(&engine, bears),
        ColorSet::from_slice(&[Color::Green])
    );
    assert_eq!(
        obj_colors(&engine, angel),
        ColorSet::from_slice(&[Color::White])
    );
    assert_eq!(
        obj_subtypes(&engine, my_forest),
        SubtypeSet::from_slice(&[subtypes::land::FOREST])
    );
    assert_eq!(
        obj_subtypes(&engine, their_plains),
        SubtypeSet::from_slice(&[subtypes::land::PLAINS])
    );
}
