//! `cards/sorceries/mv_1/kaervek_s_torch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c629feae-084c-452f-9c09-4b9716331498"
fn kaervek_s_torch() -> CardIndex {
    card_index("c629feae-084c-452f-9c09-4b9716331498")
}

#[test]
fn kaervek_s_torch_deals_x_damage_to_any_target() {
    let mut engine = lk::game(SEED, &[], &[], &[kaervek_s_torch()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 4), (ManaColor::Red, 1)],
    );
    cast_with_floating(&mut engine, P0, kaervek_s_torch());
    engine.apply(P0, PlayerAction::ChooseNumber(4)).unwrap();
    lk::aim(&mut engine, &[], &[P1]);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 16);
}

/// "X damage to any target": a creature is a target too, and the damage is
/// exactly X. X = 2 marks a Hill Giant (3/3) without killing it, X = 3 does.
#[test]
fn kaervek_s_torch_deals_exactly_x_damage_to_a_creature() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::HILL_GIANT],
        &[kaervek_s_torch(), kaervek_s_torch()],
        &[],
    );
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
    );
    cast_with_floating(&mut engine, P0, kaervek_s_torch());
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    lk::aim(&mut engine, &[giant], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(lk::damage_on(&engine, giant), 2);
    assert_eq!(
        lk::life(&engine, P1),
        20,
        "the creature, not the player, was hit"
    );

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
    );
    cast_with_floating(&mut engine, P0, kaervek_s_torch());
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    lk::aim(&mut engine, &[giant], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
}
