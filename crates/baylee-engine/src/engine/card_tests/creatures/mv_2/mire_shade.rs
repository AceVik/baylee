//! `cards/creatures/mv_2/mire_shade.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "b77a7f4a-1b3a-440e-977c-1ba39e450581"
fn mire_shade() -> CardIndex {
    card_index("b77a7f4a-1b3a-440e-977c-1ba39e450581")
}

#[test]
fn mire_shade_sacrifices_swamp_at_sorcery_speed_for_p1p1_counter() {
    let mut engine = lk::game(SEED, &[mire_shade(), swamp()], &[], &[], &[]);
    let shade = lk::obj(&engine, P0, mire_shade());
    let sw = lk::obj(&engine, P0, swamp());

    tap_all_mana(&mut engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: shade,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![sw] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, shade), (2, 2));
    assert!(on_battlefield(&engine, P0, swamp()).is_none());
}
