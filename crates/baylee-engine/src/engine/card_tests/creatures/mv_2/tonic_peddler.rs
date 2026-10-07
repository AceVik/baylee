//! `cards/creatures/mv_2/tonic_peddler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tonic Peddler is a `{1}{W}` 1/1 whose whole text is "`{W}`, `{T}`, Discard
/// a card: Target player gains 3 life."
///
/// Two halves of that line are the engine's answers rather than the card's.
/// The target is a *player*, so the offer has to name both seats and the life
/// has to land on the one that was picked — p1 here, with p0's own total as
/// the control. And "Discard a card" is a cost (CR 601.2h) that arrives as a
/// `CostDiscard` menu of hand cards, so the proof it was paid is the very card
/// the question offered lying in its owner's graveyard plus one card fewer in
/// the hand: costs are the *last* step of the activation, so while the target
/// question stood the card was still in hand and the Peddler still untapped.
/// It is cast on turn one and its ability is played a turn later, because a
/// creature has summoning sickness in the turn it arrives (CR 302.6) and a
/// `{T}` price cannot be paid there.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn tonic_peddler_discards_a_card_to_give_targeted_player_three_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Three Plains pay the {1}{W} and leave a white for the {W}; the Forest is
    // the card the ability will have to throw away, and without it in hand the
    // discard — and so the whole activation — would be unpayable.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[tonic_peddler(), forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, tonic_peddler());
    pass_until(&mut engine, stack_is_empty);
    let peddler = on_battlefield(&engine, p0, tonic_peddler()).expect("the Peddler resolved");
    assert_eq!(pt(&engine, peddler), (1, 1), "the body the card prints");

    // A turn later: the creature arrived this turn and is summoning sick, so
    // the {T} in the price is not payable until its controller's next turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    // The offer is filtered by `can_afford`, which reads the pool and not the
    // untapped lands, so the mana is tapped before anything is claimed.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains, and the Peddler is no mana source of its own"
    );
    assert!(!is_tapped(&engine, peddler), "nothing has tapped it yet");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(peddler, 0)),
        "{{W}}, {{T}}, Discard a card is payable with a card in hand: {:?}",
        legal.abilities
    );

    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    activate(&mut engine, p0, tonic_peddler(), 0);

    // CR 601.2c names the target and CR 601.2h pays afterwards, so the two
    // questions are answered in whatever order they arrive rather than in the
    // order they are expected.
    let mut aimed_at_the_opponent = false;
    let mut discarded = None;
    for _ in 0..12 {
        if aimed_at_the_opponent && discarded.is_some() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChoosePlayer { player, options } => {
                assert!(
                    options.contains(&p0) && options.contains(&p1),
                    "\"target player\" is either seat: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .unwrap();
                aimed_at_the_opponent = true;
            }
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert!(
                    player_options.contains(&p0) && player_options.contains(&p1),
                    "\"target player\" is either seat: {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .unwrap();
                aimed_at_the_opponent = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostDiscard,
                    "a cost and not a search, which is all a client has to tell \
                     the two apart"
                );
                assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
                assert!(
                    !options.is_empty(),
                    "the hand the question is about is not empty"
                );
                let doomed = options[0];
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![doomed],
                        },
                    )
                    .unwrap();
                discarded = Some(doomed);
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Peddler's ability resolves: {other:?}"),
        }
    }
    assert!(
        aimed_at_the_opponent,
        "the ability asked whom to give the life to"
    );
    let discarded = discarded.expect("the discard cost is part of the price");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        23,
        "\"Target player gains 3 life\" — three, on the seat that was named"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing for the seat that paid for it"
    );
    assert!(
        is_tapped(&engine, peddler),
        "{{T}} was half the price, paid by the Peddler itself"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before - 1,
        "\"Discard a card\" leaves the hand one card shorter"
    );
    assert_eq!(
        engine.state().object(discarded).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard),
        "and it is the very card the question offered, in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, tonic_peddler()).is_some(),
        "the Peddler outlives the card it threw away"
    );
}
