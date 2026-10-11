//! `cards/lands/utility/hellion_crucible.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hellion Crucible: "{T}: Add {C}." / "{1}{R}, {T}: Put a pressure counter on this land..." / "{1}{R}, {T}, Remove two pressure counters..."
/// Under `Coverage::Partial`, pressure counter abilities are omitted because pressure counters have no assigned id.
/// Playing this land allows it to enter untapped and immediately tap for colorless mana.
#[test]
fn hellion_crucible_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(335, forest())
        .hand(0, &[hellion_crucible()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hellion_crucible());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, hellion_crucible(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn pressure_counters_and_hellions_both_use_replacement_effects() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::HELLION_CRUCIBLE, index::DOUBLING_SEASON])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::HELLION_CRUCIBLE).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 4);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!legal.abilities.contains(&(land, 2)));
    activate(&mut e, p0, index::HELLION_CRUCIBLE, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state()
            .object(land)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::PRESSURE),
        2
    );
    e.dev_state_mut(p0).unwrap().set_tapped(land, false);
    e.refresh_offer();
    activate(&mut e, p0, index::HELLION_CRUCIBLE, 2);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
    pass_until(&mut e, stack_is_empty);
    let tokens = crate::engine::card_tests::token_helpers::named(&e, "Hellion");
    assert_eq!(tokens.len(), 2);
    for token in tokens {
        assert_eq!(pt(&e, token), (4, 4));
        assert!(
            e.state()
                .object(token)
                .unwrap()
                .characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::HASTE)
        );
    }
}
