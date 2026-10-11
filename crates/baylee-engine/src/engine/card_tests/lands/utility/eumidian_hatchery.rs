//! `cards/lands/utility/eumidian_hatchery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eumidian Hatchery prints `{{T}}, Pay 1 life: Add {{B}}. Put a hatchling counter on this land.`
/// and `When this land is put into a graveyard from the battlefield, for each hatchling counter
/// on it, create a 1/1 black Insect creature token with flying.`
///
/// Because its activation cost includes paying life rather than tapping alone, it is not tapped
/// by automatic mana helpers. Activating ability 0 manually pays 1 life, adds one black mana,
/// places one hatchling counter, and leaves ability index 1 absent from `legal.abilities`.
#[test]
fn eumidian_hatchery_pays_life_adds_black_and_puts_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[eumidian_hatchery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hatchery =
        on_battlefield(&engine, p0, eumidian_hatchery()).expect("eumidian hatchery on battlefield");
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(counters_on(&engine, hatchery, counters::HATCHLING), 0);
    assert!(!is_tapped(&engine, hatchery));

    activate(&mut engine, p0, eumidian_hatchery(), 0);

    assert_eq!(engine.state().players[0].life, 19);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.total(), 1);
    assert_eq!(counters_on(&engine, hatchery, counters::HATCHLING), 1);
    assert!(is_tapped(&engine, hatchery));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(source, idx)| *source == hatchery && *idx == 1),
        "the graveyard trigger is not an activated ability"
    );
}

#[test]
fn hatchery_uses_its_departure_count_and_token_multipliers_after_returning() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[eumidian_hatchery(), index::DOUBLING_SEASON])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, eumidian_hatchery()).unwrap();
    for _ in 0..3 {
        e.dev_state_mut(p0).unwrap().set_tapped(land, false);
        e.refresh_offer();
        activate(&mut e, p0, eumidian_hatchery(), 0);
    }
    assert_eq!(counters_on(&e, land, counters::HATCHLING), 6);
    assert_eq!(e.state().players[0].life, 17);
    bury(&mut e, &[land]);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    {
        let s = e.dev_state_mut(p0).unwrap();
        s.object_mut(land).unwrap().kind = crate::object::ObjectKind::Permanent;
        s.move_object(
            land,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, land, counters::HATCHLING), 0);
    let tokens = tokens_of(&e, p0);
    assert_eq!(tokens.len(), 12);
    for token in tokens {
        assert_eq!(pt(&e, token), (1, 1));
        assert!(keywords(&e, token).contains(baylee_cards_dsl::KeywordSet::FLYING));
    }
}
