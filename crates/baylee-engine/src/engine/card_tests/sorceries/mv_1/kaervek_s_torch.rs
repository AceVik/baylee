//! `cards/sorceries/mv_1/kaervek_s_torch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

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
