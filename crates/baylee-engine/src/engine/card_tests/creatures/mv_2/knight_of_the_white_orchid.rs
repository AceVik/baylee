//! `cards/creatures/mv_2/knight_of_the_white_orchid.rs`, played.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("7d0efc64-151f-45b4-8c0b-c32609f9d862")
}

#[test]
fn knight_conditionally_offers_a_plains_onto_the_battlefield_untapped() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[subject()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, |g| matches!(g.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, |g| {
        matches!(
            g.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!()
    };
    let fetched_plains = options[0];
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![fetched_plains],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let knight = on_battlefield(&e, p0, subject()).unwrap();
    assert!(keywords(&e, knight).contains(KeywordSet::FIRST_STRIKE));
    let lands = all_on_battlefield(&e, p0, plains());
    assert_eq!(lands.len(), 3);
    assert!(!is_tapped(&e, fetched_plains));
}

fn savannah() -> CardIndex {
    card_index("703243f0-8cb3-420f-958f-5fd4bde30293")
}

/// "A Plains card" is any card with the Plains type, Savannah too, and no
/// Forest; and "if an opponent controls more lands than you" is false at two
/// lands each, so the Knight then triggers nothing at all (CR 603.4). The
/// first test's library was all Plains and its opponent always ahead, so a
/// search for any card or an unconditional trigger passed it.
#[test]
fn knight_finds_only_plains_cards_and_needs_an_opponent_ahead_on_lands() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[subject(), subject(), plains(), savannah()])
        .battlefield(1, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let basic = in_hand(&e, p0, plains()).expect("a basic Plains");
    let dual = in_hand(&e, p0, savannah()).expect("Savannah");
    for card in [basic, dual] {
        e.state
            .move_object(
                card,
                ZoneLocation::Library(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, |g| matches!(g.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, |g| {
        matches!(
            g.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { mut options, .. } = e.pending().clone() else {
        unreachable!()
    };
    options.sort();
    let mut plains_cards = vec![basic, dual];
    plains_cards.sort();
    assert_eq!(options, plains_cards, "both Plains cards, no Forest");
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![dual],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(on_battlefield(&e, p0, savannah()), Some(dual));

    // Three lands each now: a second Knight finds nobody ahead.
    {
        let state = e.dev_state_mut(p0).expect("dev board");
        state.players[0].mana_pool.add(ManaColor::White, 2);
    }
    e.refresh_offer();
    cast_with_floating(&mut e, p0, subject());
    pass_until(&mut e, |g| {
        stack_is_empty(g) || matches!(g.pending(), Pending::YesNo { .. })
    });
    assert!(
        stack_is_empty(&e) && !matches!(e.pending(), Pending::YesNo { .. }),
        "nothing triggered: {:?}",
        e.pending()
    );
    assert_eq!(all_on_battlefield(&e, p0, subject()).len(), 2);
}
