//! `cards/artifacts/mv_2/crown_of_the_ages.rs`, played.
//!
//! Oracle:
//! - "{4}, {T}: Attach target Aura attached to a creature to another creature."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::aura_kit as kit;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "0b772ef2-b760-400b-8e5b-f4b98e6c935d"
fn crown_of_the_ages() -> CardIndex {
    card_index("0b772ef2-b760-400b-8e5b-f4b98e6c935d")
}

#[test]
fn crown_of_the_ages_moves_target_aura_from_one_creature_to_another() {
    let mut engine = lk::game(
        SEED,
        &[crown_of_the_ages(), index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[index::WALL_OF_WOOD],
        &[index::HOLY_STRENGTH],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let wood = lk::obj(&engine, P1, index::WALL_OF_WOOD);
    let strength = kit::cast_aura(
        &mut engine,
        P0,
        index::HOLY_STRENGTH,
        bears,
        &[(ManaColor::White, 1)],
    );
    assert_eq!(pt(&engine, bears), (3, 4));

    lk::float(&mut engine, P0, &[(ManaColor::Colorless, 4)]);
    activate(&mut engine, P0, crown_of_the_ages(), 0);
    lk::aim(&mut engine, &[strength], &[]);
    pass_until(&mut engine, |e| {
        kit::asks_for_a_permanent(e) || stack_is_empty(e)
    });
    assert!(
        kit::asks_for_a_permanent(&engine),
        "asks which other creature to attach the Aura to"
    );
    let _ = kit::choose_among(&mut engine, P0, wood);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(kit::attached_to(&engine, strength), Some(wood));
    assert_eq!(pt(&engine, wood), (1, 5));
    assert_eq!(pt(&engine, bears), (2, 2));
}
