#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch45_north_star_is_optional_once_and_cannot_reduce_colored_mana_to_zero() {
    let mut e = prepared(
        &[index::GRIZZLY_BEARS, index::COUNSEL_OF_THE_SORATAMI],
        &[index::NORTH_STAR],
        &[],
    );
    activate(&mut e, P0, index::NORTH_STAR, 0);
    pass_until(&mut e, stack_is_empty);
    pool(&mut e, ManaColor::Red, 2);
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    assert!(castable(&e, bear));
    cast(&mut e, bear);
    assert!(matches!(
        e.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::ManaAnyType,
            ..
        }
    ));
    e.apply(P0, PlayerAction::CancelCast).unwrap();
    assert!(crate::casting::north_star_effect(&e.state, P0).is_some());
    cast(&mut e, bear);
    yes(&mut e, true);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    assert!(crate::casting::north_star_effect(&e.state, P0).is_none());
    pass_until(&mut e, stack_is_empty);
    pool(&mut e, ManaColor::Red, 3);
    assert!(!castable(
        &e,
        in_hand(&e, P0, index::COUNSEL_OF_THE_SORATAMI).unwrap()
    ));
}

#[test]
fn batch45_north_star_keeps_kicker_colored_and_cancellation_preserves_the_grant() {
    let mut e = prepared(
        &[index::TEAR_ASUNDER],
        &[index::NORTH_STAR],
        &[index::SOL_RING],
    );
    activate(&mut e, P0, index::NORTH_STAR, 0);
    pass_until(&mut e, stack_is_empty);
    pool(&mut e, ManaColor::Red, 4);
    let spell = in_hand(&e, P0, index::TEAR_ASUNDER).unwrap();
    let target_card = on_battlefield(&e, P1, index::SOL_RING).unwrap();
    cast(&mut e, spell);
    if let Pending::ChooseCastMode { options, .. } = e.pending() {
        assert!(
            options
                .iter()
                .all(|option| option.kind != CastModeKind::Kicked)
        );
        mode(&mut e, CastModeKind::Normal);
    }
    e.apply(P0, PlayerAction::CancelCast).unwrap();
    assert!(crate::casting::north_star_effect(&e.state, P0).is_some());
    e.state.players[0].mana_pool.add(ManaColor::Black, 1);
    cast(&mut e, spell);
    mode(&mut e, CastModeKind::Kicked);
    yes(&mut e, true);
    target(&mut e, vec![target_card], vec![]);
    assert_eq!(e.state.players[0].mana_pool.available(ManaColor::Black), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(target_card).unwrap().zone, Zone::Exile);
}

#[test]
fn batch45_throne_mana_is_offered_for_a_kicked_spell_before_it_is_on_the_stack() {
    let mut e = prepared(
        &[index::TEAR_ASUNDER, index::GIANT_GROWTH],
        &[index::THRONE_OF_MAKINDI, index::GRIZZLY_BEARS],
        &[],
    );
    activate(&mut e, P0, index::THRONE_OF_MAKINDI, 1);
    pass_until(&mut e, stack_is_empty);
    let throne = on_battlefield(&e, P0, index::THRONE_OF_MAKINDI).unwrap();
    e.state.set_tapped(throne, false);
    pool(&mut e, ManaColor::Black, 2);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: throne,
            ability_index: 2,
        },
    )
    .unwrap();
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(counters_on(&e, throne, CounterKind::Charge), 0);
    let vines = in_hand(&e, P0, index::TEAR_ASUNDER).unwrap();
    assert!(castable(&e, vines));
    assert!(!castable(&e, in_hand(&e, P0, index::GIANT_GROWTH).unwrap()));
    cast(&mut e, vines);
    mode(&mut e, CastModeKind::Kicked);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    target(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
}

#[test]
fn batch45_bloodsoaked_counts_losses_without_netting_gains_and_limits_the_spending_grant() {
    let mut e = prepared(
        &[index::BLOODSOAKED_INSIGHT, index::COUNSEL_OF_THE_SORATAMI],
        &[],
        &[],
    );
    e.state.change_life(P1, -4, Cause::Effect);
    e.state.change_life(P1, 9, Cause::Effect);
    let top = library(&e, P1).last().copied().unwrap();
    let original = e.state.object(top).unwrap().card.unwrap();
    // Put a known expensive sorcery into the opponent's library for the real exile effect.
    let borrowed = in_hand(&e, P0, index::COUNSEL_OF_THE_SORATAMI).unwrap();
    e.state.object_mut(borrowed).unwrap().owner = P1;
    e.state
        .move_object(
            borrowed,
            ZoneLocation::Library(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pool(&mut e, ManaColor::Red, 3);
    cast_with_floating(&mut e, P0, index::BLOODSOAKED_INSIGHT);
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(borrowed).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.object(top).unwrap().card, Some(original));
    pool(&mut e, ManaColor::Colorless, 3);
    cast(&mut e, borrowed);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(borrowed).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state.object(borrowed).unwrap().zone_owner, Some(P1));
}

/// "Target opponent exiles the top three cards of their library": exactly the
/// top three of p1's library leave it, for p1's exile zone; the fourth card
/// and p0's own library are untouched.
#[test]
fn batch45_bloodsoaked_exiles_exactly_the_top_three_cards_of_the_opponents_library() {
    let mut e = prepared(&[index::BLOODSOAKED_INSIGHT], &[], &[]);
    let theirs = library(&e, P1);
    let mine = library(&e, P0);
    let (len, my_len) = (theirs.len(), mine.len());
    assert!(
        len > 4,
        "control: a library deep enough to tell three from four"
    );
    let top_three: Vec<ObjectId> = theirs[len - 3..].to_vec();
    let fourth = theirs[len - 4];
    cast_with_floating(&mut e, P0, index::BLOODSOAKED_INSIGHT);
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        library(&e, P1).len(),
        len - 3,
        "three cards left the library, not two or four"
    );
    let mut exiled = e.state.zones.list(ZoneLocation::Exile(P1)).clone();
    exiled.sort();
    let mut expected = top_three;
    expected.sort();
    assert_eq!(
        exiled, expected,
        "exactly the top three, in p1's exile zone"
    );
    assert_eq!(
        library(&e, P1).last().copied(),
        Some(fourth),
        "the fourth card is the new top of the library"
    );
    assert_eq!(
        library(&e, P0).len(),
        my_len,
        "\"target opponent\" exiles theirs, not mine"
    );
    assert!(e.state.zones.list(ZoneLocation::Exile(P0)).is_empty());
}
