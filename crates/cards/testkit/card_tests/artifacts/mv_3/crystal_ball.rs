//! Oracle checklist: pay {1} and tap; inspect exactly the top two, place one on top and one on bottom in the chosen piles, without drawing or changing hand.
#[allow(clippy::wildcard_imports)]
use super::*;

fn crystal_ball() -> CardIndex {
    card_index("bd85fe4d-1d62-416f-ac2d-e287911c84e3")
}

#[test]
fn ball_scry_two_keeps_one_known_card_on_top_and_sends_the_other_to_bottom() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[crystal_ball()])
        .hand(0, &[mountain(), island()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let bottom = hand_to_library_top(&mut e, p0, mountain());
    let top = hand_to_library_top(&mut e, p0, island());
    let library_before = library_size(&e, p0);
    let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.refresh_offer();

    activate(&mut e, p0, crystal_ball(), 0);
    assert!(is_tapped(
        &e,
        on_battlefield(&e, p0, crystal_ball()).unwrap()
    ));
    pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange {
        player,
        cards,
        prompt,
        ..
    } = e.pending().clone()
    else {
        panic!(
            "scry asks how to arrange the looked-at cards: {:?}",
            e.pending()
        );
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ArrangePrompt::Scry);
    assert_eq!(cards.len(), 2);
    assert!(cards.contains(&top) && cards.contains(&bottom));
    e.apply(
        p0,
        PlayerAction::Arrange {
            piles: vec![vec![top], vec![bottom]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(library_size(&e, p0), library_before);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(p0)).last(),
        Some(&top)
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(p0)).first(),
        Some(&bottom)
    );
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}

#[test]
fn ball_scry_two_allows_any_top_order_or_both_cards_on_bottom() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[crystal_ball()])
        .hand(0, &[mountain(), island()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let first = hand_to_library_top(&mut e, p0, mountain());
    let second = hand_to_library_top(&mut e, p0, island());
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.refresh_offer();
    activate(&mut e, p0, crystal_ball(), 0);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
    e.apply(
        p0,
        PlayerAction::Arrange {
            piles: vec![vec![first, second], vec![]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let library = e.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(
        library[library.len() - 2..],
        [second, first],
        "both stay on top in a chosen order"
    );

    let mut bottom = Duel::new(SEED + 1, forest())
        .battlefield(0, &[crystal_ball()])
        .hand(0, &[mountain(), island()])
        .start();
    keep_mulligans(&mut bottom);
    assert!(walk_to_own_main(&mut bottom, p0));
    let top = hand_to_library_top(&mut bottom, p0, mountain());
    let next = hand_to_library_top(&mut bottom, p0, island());
    let before = bottom.state().zones.list(ZoneLocation::Library(p0)).clone();
    bottom.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    bottom.refresh_offer();
    activate(&mut bottom, p0, crystal_ball(), 0);
    pass_until(&mut bottom, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    bottom
        .apply(
            p0,
            PlayerAction::Arrange {
                piles: vec![vec![], vec![top, next]],
            },
        )
        .unwrap();
    pass_until(&mut bottom, stack_is_empty);
    let cards = bottom.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(
        &cards[..2],
        &[next, top],
        "both bottom cards retain their chosen order"
    );
    let mut expected = vec![next, top];
    expected.extend(
        before
            .into_iter()
            .filter(|card| *card != top && *card != next),
    );
    assert_eq!(
        cards,
        expected.as_slice(),
        "the rest of the library remains intact"
    );
}
