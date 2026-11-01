//! Aura Mutation — {G}{W} — Instant
//! Oracle: Destroy target enchantment. Create X 1/1 green Saproling creature tokens, where X is that enchantment's mana value.

use super::*;
use baylee_core::generated::index;

#[test]
fn aura_mutation_destroys_enchantment_and_creates_saprolings_equal_to_its_mana_value() {
    let mutation = card_index("19bb51dd-2b93-45f0-927d-bbafddadd0a3");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[forest(), plains()])
        .battlefield(1, &[index::CRUSADE])
        .hand(0, &[mutation])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let crusade = on_battlefield(&engine, p1, index::CRUSADE).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, mutation);
    aim_at(&mut engine, p0, crusade);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::CRUSADE).is_none());
    let saprolings = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == p0 && o.card.is_none())
        })
        .count();
    assert_eq!(saprolings, 2);
}
