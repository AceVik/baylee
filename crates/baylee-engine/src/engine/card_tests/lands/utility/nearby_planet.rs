//! `cards/lands/utility/nearby_planet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn nearby_planet_enters_tapped_and_pays_tax_to_remain_on_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[nearby_planet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Float {{1}} from Forest.
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    let planet = play_land(&mut engine, p0, nearby_planet());
    assert!(
        entered_tapped(&engine, planet),
        "nearby planet enters tapped"
    );

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });

    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!("expected PayTax prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, YesNoPrompt::PayTax { mana: 1 });

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, nearby_planet()).is_some(),
        "nearby planet remains on battlefield when tax is paid"
    );
    assert!(is_tapped(&engine, planet), "remains tapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the one mana was spent paying the tax"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.mana_abilities.contains(&planet),
        "the tapped land cannot produce mana"
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == planet),
        "no activated abilities on nearby planet"
    );
}

#[test]
fn nearby_planet_has_all_land_types_and_produces_each_color_but_not_colorless() {
    use baylee_core::generated::{index, subtypes};
    let p0 = PlayerId::new(0);
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::NEARBY_PLANET])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let planet = on_battlefield(&e, p0, index::NEARBY_PLANET).unwrap();
        assert_eq!(
            e.state().object(planet).unwrap().characteristics().subtypes,
            subtypes::ALL_LAND_TYPES
        );
        // Fixture enters tapped; explicitly untap to exercise the intrinsic offer.
        e.dev_state_mut(p0)
            .unwrap()
            .object_mut(planet)
            .unwrap()
            .status
            .remove(crate::object::Status::TAPPED);
        e.refresh_offer();
        e.apply(p0, PlayerAction::ActivateManaAbility { source: planet })
            .unwrap();
        let Pending::ChooseColor { options, .. } = e.pending() else {
            panic!("five colors")
        };
        assert_eq!(options.len(), 5);
        assert!(!options.contains(&ManaColor::Colorless));
        e.apply(p0, PlayerAction::ChooseColor(color)).unwrap();
        assert_eq!(e.state().players[0].mana_pool.available(color), 1);
        assert!(stack_is_empty(&e));
    }
}
