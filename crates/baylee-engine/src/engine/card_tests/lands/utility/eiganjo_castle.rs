//! `cards/lands/utility/eiganjo_castle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eiganjo Castle: "{T}: Add {W}." / "{W}, {T}: Prevent the next 2 damage that would be dealt to target legendary creature this turn."
/// The legendary land taps for its base mana ability to add {W} to the mana pool.
#[test]
fn eiganjo_castle_taps_for_white_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(139, forest())
        .battlefield(0, &[eiganjo_castle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let castle = on_battlefield(&engine, p0, eiganjo_castle()).expect("Castle deployed");
    activate(&mut engine, p0, eiganjo_castle(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, castle));
}

#[test]
fn eiganjo_prevents_exactly_two_damage_to_a_legendary_creature() {
    use crate::engine::card_tests::legends_kit as kit;
    let emry = card_index("da3e7d3d-2ca0-40c3-9602-fca37c92f507");
    let mut engine = kit::game(
        918,
        &[eiganjo_castle(), emry, llanowar_elves()],
        &[],
        &[lightning_bolt(), lightning_bolt()],
        &[],
    );
    let target = kit::obj(&engine, kit::P0, emry);
    let elf = kit::obj(&engine, kit::P0, llanowar_elves());
    kit::float(
        &mut engine,
        kit::P0,
        &[(ManaColor::White, 1), (ManaColor::Red, 2)],
    );
    activate(&mut engine, kit::P0, eiganjo_castle(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("target")
    };
    assert!(options.contains(&target));
    assert!(!options.contains(&elf));
    kit::aim(&mut engine, &[target], &[]);
    kit::settle(&mut engine);
    kit::cast_at(&mut engine, kit::P0, lightning_bolt(), &[target], &[]);
    assert_eq!(kit::damage_on(&engine, target), 1);
    kit::cast_at(&mut engine, kit::P0, lightning_bolt(), &[target], &[]);
    assert_eq!(engine.state().object(target).unwrap().zone, Zone::Graveyard);
}
