//! `cards/lands/check/shifting_woodland.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn shifting_woodland_enters_untapped_with_forest_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(305, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[shifting_woodland()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shifting_woodland());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, shifting_woodland(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn woodland_checks_distinct_types_then_restores_itself_at_cleanup() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[shifting_woodland()])
        .hand(
            0,
            &[
                index::ORNITHOPTER,
                forest(),
                island(),
                swords_to_plowshares(),
            ],
        )
        .hand(1, &[index::HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, shifting_woodland()).unwrap();
    let model = bury_from_hand(&mut e, p0, index::ORNITHOPTER);
    bury_from_hand(&mut e, p0, forest());
    bury_from_hand(&mut e, p0, island());
    bury_from_hand(&mut e, PlayerId::new(1), index::HOLY_STRENGTH);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 4);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(
        !legal.abilities.contains(&(land, 1)),
        "only three distinct types in our graveyard"
    );
    let instant = bury_from_hand(&mut e, p0, swords_to_plowshares());
    activate(&mut e, p0, shifting_woodland(), 1);
    let offered = aim_at(&mut e, p0, model);
    assert!(!offered.contains(&instant), "permanent cards only");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, land), (0, 2));
    assert!(keywords(&e, land).contains(KeywordSet::FLYING));
    assert_eq!(e.state().object(model).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, |e| e.state().turn.active != p0);
    assert_eq!(types(&e, land), TypeSet::LAND);
    assert!(!keywords(&e, land).contains(KeywordSet::FLYING));
}

/// "target permanent card in your graveyard": the menu is the permanent cards
/// of *our* graveyard and nothing else — not an instant of ours (not a
/// permanent card), not a permanent card in the opponent's graveyard — and the
/// ability on the stack names exactly the card that was chosen.
#[test]
fn woodland_offers_only_our_own_permanent_cards_and_targets_exactly_the_chosen_one() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[shifting_woodland()])
        .hand(
            0,
            &[
                index::GRIZZLY_BEARS,
                index::ORNITHOPTER,
                forest(),
                swords_to_plowshares(),
            ],
        )
        .hand(1, &[index::GRIZZLY_BEARS, index::MIND_STONE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, shifting_woodland()).unwrap();
    // Four card types in our graveyard (creature, artifact, land, instant):
    // delirium is met by our graveyard alone.
    let chosen = bury_from_hand(&mut e, p0, index::GRIZZLY_BEARS);
    let own_artifact = bury_from_hand(&mut e, p0, index::ORNITHOPTER);
    let own_land = bury_from_hand(&mut e, p0, forest());
    let own_instant = bury_from_hand(&mut e, p0, swords_to_plowshares());
    // Permanent cards in the *opponent's* graveyard, one of them the very
    // same printing as the card we will choose.
    let their_creature = bury_from_hand(&mut e, p1, index::GRIZZLY_BEARS);
    let their_artifact = bury_from_hand(&mut e, p1, index::MIND_STONE);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 4);
    e.refresh_offer();
    activate(&mut e, p0, shifting_woodland(), 1);
    let offered = aim_at(&mut e, p0, chosen);
    for card in [chosen, own_artifact, own_land] {
        assert!(
            offered.contains(&card),
            "a permanent card in our own graveyard is a legal choice"
        );
    }
    assert!(
        !offered.contains(&own_instant),
        "an instant card is not a permanent card"
    );
    for card in [their_creature, their_artifact] {
        assert!(
            !offered.contains(&card),
            "\"in your graveyard\": a permanent card in the opponent's graveyard is not a legal choice"
        );
    }
    let stack = e.state().zones.list(ZoneLocation::Stack).clone();
    let [ability] = stack[..] else {
        panic!("one ability on the stack, got {stack:?}")
    };
    assert_eq!(
        e.state().object(ability).unwrap().targets.as_slice(),
        &[chosen],
        "the copy ability targets exactly the card that was chosen"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, land),
        (2, 2),
        "it became a copy of the chosen Grizzly Bears"
    );
    assert_eq!(
        e.state().object(chosen).unwrap().zone,
        Zone::Graveyard,
        "the card is copied, not exiled or moved"
    );
}
