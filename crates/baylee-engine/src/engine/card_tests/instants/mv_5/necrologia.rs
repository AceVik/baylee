//! `cards/instants/mv_5/necrologia.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn necrologia() -> CardIndex {
    card_index("925f11c3-fd32-4aa7-a130-6554b81ad3eb")
}

#[test]
fn necrologia_pays_x_life_and_draws_x_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[necrologia()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    let card = in_hand(&engine, p0, necrologia()).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before - 1 + 3
    );
}
