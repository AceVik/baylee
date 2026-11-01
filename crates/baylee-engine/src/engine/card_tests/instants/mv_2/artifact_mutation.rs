//! Artifact Mutation — {R}{G} — Instant
//! Oracle: Destroy target artifact. It can't be regenerated. Create X 1/1 green Saproling creature tokens, where X is that artifact's mana value.

use super::*;
use baylee_core::generated::index;

#[test]
fn artifact_mutation_destroys_artifact_and_creates_saprolings_equal_to_its_mana_value() {
    let mutation = card_index("e77e8768-3ea8-44f9-aba6-1e023e006946");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[mountain(), forest()])
        .battlefield(1, &[index::JAYEMDAE_TOME])
        .hand(0, &[mutation])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let tome = on_battlefield(&engine, p1, index::JAYEMDAE_TOME).unwrap();
    engine
        .dev_state_mut(p1)
        .unwrap()
        .object_mut(tome)
        .unwrap()
        .regeneration_shields = 1;
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, mutation);
    aim_at(&mut engine, p0, tome);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::JAYEMDAE_TOME).is_none());
    let saprolings: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == p0 && o.card.is_none())
        })
        .collect();
    assert_eq!(saprolings.len(), 4);
    for saproling in saprolings {
        assert_eq!(pt(&engine, saproling), (1, 1));
        let characteristics = engine.state().object(saproling).unwrap().characteristics();
        assert!(
            characteristics
                .colors
                .contains(baylee_core::color::Color::Green)
        );
        assert!(
            characteristics
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::SAPROLING)
        );
    }
}
