//! `cards/artifacts/mv_2/illusionary_mask.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mask casts as an artifact and offers its sorcery-speed X ability.
#[test]
fn illusionary_mask_casts_and_offers_its_x_ability() {
    let card = card_index("05ac866d-0405-4d25-986a-c10fcfc097e6");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);
    let mask = on_battlefield(&engine, p0, card).expect("Mask resolved");
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority");
    };
    assert!(legal.abilities.contains(&(mask, 0)));
    assert!(
        engine
            .state()
            .object(mask)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ARTIFACT)
    );
}
