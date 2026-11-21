//! `cards/instants/mv_2/liliana_s_triumph.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c2e68ac5-cfff-4b76-8062-25a82fdf9a5c"
fn liliana_s_triumph() -> CardIndex {
    card_index("c2e68ac5-cfff-4b76-8062-25a82fdf9a5c")
}

/// Without a Liliana planeswalker (even with a non-Liliana planeswalker and a
/// creature named Liliana), Liliana's Triumph ({1}{B}) makes each opponent
/// sacrifice a creature of their choice without discarding a card.
#[test]
fn lilianas_triumph_without_liliana_only_sacrifices_creature() {
    let mut engine = lk::game(
        SEED,
        &[
            index::SAVANNAH_LIONS,
            index::JACE_THE_MIND_SCULPTOR,
            index::LILIANA_THE_REPENTANT,
        ],
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[liliana_s_triumph()],
        &[mountain()],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, liliana_s_triumph());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    assert!(matches!(
        engine.pending(),
        Pending::ChooseCards { player, .. } if *player == P1
    ));
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, P0, index::SAVANNAH_LIONS).is_some());
    assert!(
        in_hand(&engine, P1, mountain()).is_some(),
        "without a Liliana planeswalker, opponent does not discard"
    );
}

/// With a Liliana planeswalker under your control, each opponent sacrifices a
/// creature AND discards a card.
#[test]
fn lilianas_triumph_with_liliana_also_makes_opponent_discard() {
    let mut engine = lk::game(
        SEED,
        &[index::JACE_THE_MIND_SCULPTOR],
        &[index::GRIZZLY_BEARS],
        &[liliana_s_triumph()],
        &[mountain(), plains()],
    );
    let pw = lk::obj(&engine, P0, index::JACE_THE_MIND_SCULPTOR);
    {
        let st = engine.dev_state_mut(P0).unwrap();
        std::sync::Arc::make_mut(&mut st.object_mut(pw).unwrap().base)
            .subtypes
            .insert(baylee_core::generated::subtypes::planeswalker::LILIANA);
        st.refresh_characteristics();
    }
    let mtn = in_hand(&engine, P1, mountain()).unwrap();

    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, liliana_s_triumph());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, P1);
    if options.contains(&mtn) {
        engine
            .apply(P1, PlayerAction::ChooseObjects { objects: vec![mtn] })
            .unwrap();
    } else {
        engine
            .apply(
                P1,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        engine
            .apply(P1, PlayerAction::ChooseObjects { objects: vec![mtn] })
            .unwrap();
    }
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(
        in_graveyard(&engine, P1, mountain()).is_some(),
        "controlling a Liliana planeswalker makes opponent discard a card too"
    );
    assert!(in_hand(&engine, P1, plains()).is_some());
}
