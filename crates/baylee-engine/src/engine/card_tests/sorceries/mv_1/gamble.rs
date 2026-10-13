//! `cards/sorceries/mv_1/gamble.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gamble — {R} sorcery: "Search your library for a card, put that card into your
/// hand, discard a card at random, then shuffle."
///
/// With no other hand card, the tutored card is necessarily discarded.
#[test]
fn gamble_discards_its_only_hand_card_after_searching() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[gamble()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let lib_size_before = library_size(&engine, p0);
    let spell = in_hand(&engine, p0, gamble()).expect("gamble is in hand");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("one Mountain pays {R} for Gamble");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until only stops on ChooseCards")
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "search prompt indicates library tutor"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "Gamble searches for exactly one card (mandatory search)"
    );
    assert!(
        !options.is_empty(),
        "library has basic Forests to search from"
    );

    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("choosing the card from library is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(chosen).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the searched card was the only possible random discard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        0,
        "Gamble discarded the tutored card"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_size_before - 1,
        "library is one card smaller after tutoring"
    );
    assert_eq!(
        in_graveyard(&engine, p0, gamble()),
        Some(spell),
        "Gamble resolved into the graveyard"
    );
}

/// "discard a card at random": with a Forest and an Island already in hand the
/// card is picked by the engine, never asked of the player. After the search
/// answer there is no discard prompt, the hand is back to two cards (the tutored
/// one in, one discarded out), exactly one of the three cards left it, and
/// across seeds the discarded card is not always the same one.
#[test]
fn gamble_discards_at_random_and_never_asks_which_card() {
    let p0 = PlayerId::new(0);
    // 0: the Forest that was already in hand, 1: the Island, 2: the tutored card.
    let mut discarded_roles = std::collections::BTreeSet::new();
    for seed in 0..24 {
        let mut engine = Duel::new(seed, forest())
            .battlefield(0, &[mountain()])
            .hand(0, &[gamble(), forest(), island()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
        let kept_forest = in_hand(&engine, p0, forest()).expect("the Forest is in hand");
        let kept_island = in_hand(&engine, p0, island()).expect("the Island is in hand");
        cast_from_hand(&mut engine, p0, gamble());
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            unreachable!("pass_until only stops on ChooseCards")
        };
        let tutored = options[0];
        let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
        assert_eq!(
            hand_before, 2,
            "control: Forest and Island, Gamble is on the stack"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![tutored],
                },
            )
            .unwrap();
        assert!(
            !matches!(engine.pending(), Pending::ChooseCards { .. }),
            "seed {seed}: \"at random\" asks the player nothing, got {:?}",
            engine.pending()
        );
        pass_until(&mut engine, stack_is_empty);
        let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
        assert_eq!(
            hand.len(),
            2,
            "seed {seed}: one card tutored in, one discarded out"
        );
        let gone: Vec<usize> = [kept_forest, kept_island, tutored]
            .iter()
            .enumerate()
            .filter(|(_, id)| !hand.contains(id))
            .map(|(role, _)| role)
            .collect();
        assert_eq!(
            gone.len(),
            1,
            "seed {seed}: exactly one of the three cards was discarded"
        );
        assert_eq!(
            engine
                .state()
                .object([kept_forest, kept_island, tutored][gone[0]])
                .map(|o| o.zone),
            Some(Zone::Graveyard)
        );
        discarded_roles.insert(gone[0]);
    }
    assert!(
        discarded_roles.len() >= 2,
        "across 24 seeds the random discard is not one fixed card, got {discarded_roles:?}"
    );
}
