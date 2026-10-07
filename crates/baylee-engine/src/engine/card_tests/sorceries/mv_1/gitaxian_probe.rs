//! `cards/sorceries/mv_1/gitaxian_probe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gitaxian Probe — `{U/P}` sorcery: "({U/P} can be paid with either {U} or
/// 2 life.) Look at target player's hand. Draw a card."
///
/// The card is `Coverage::Partial` with the *look* missing and the target
/// requirement and the draw written, so the three claims here are exactly
/// that half. First the question: "target player" enumerates both seats —
/// and it arrives as `ChoosePlayer` rather than as the object-and-player
/// `ChooseTargets`, because a requirement that is only a player never
/// reaches the object half of targeting — and the probe is aimed at the
/// opponent, the seat whose hand it is printed to look at. Second the cantrip: the spell
/// leaves the hand, lands in its owner's graveyard, and puts exactly one
/// card off the top of the caster's library into that same hand, so the
/// hand it was cast from is the hand it finishes with. Third the gap: the
/// target's hand and library are the size they were, because no effect
/// reads a hand and the look moves nothing.
#[test]
fn gitaxian_probe_asks_for_any_player_and_replaces_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(17, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[gitaxian_probe()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    let library_before = library_size(&engine, p0);
    let their_hand = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p1))
        .len();
    let their_library = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, gitaxian_probe());

    // The phyrexian symbol may be asked about before the target is; two
    // Islands are sitting there to pay either way, so the first option the
    // engine lists is taken.
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseCastMode { player, .. } => {
                engine.apply(player, PlayerAction::ChooseMode(0)).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            Pending::ChoosePlayer { .. } => break,
            other => panic!("expected the spell's target question, got {other:?}"),
        }
    }

    // A spell whose whole target requirement is a player is asked as
    // `ChoosePlayer` and not as `ChooseTargets`: the wizard branches on
    // `TargetSpec::AnyPlayer` before the object half of targeting runs, so
    // there is no empty object list to inspect — the enumeration of seats
    // *is* the question.
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("the probe never asked what to look at")
    };
    assert_eq!(player, p0, "the caster chooses the target");
    assert_eq!(
        options,
        vec![p0, p1],
        "\"target player\" is every seat at the table, the caster included"
    );

    engine
        .apply(p0, PlayerAction::ChoosePlayer(p1))
        .expect("the opponent is a player the probe may target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, gitaxian_probe()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before,
        "one card spent and one drawn: the Probe is a cantrip"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the card drawn came off the top of the caster's library"
    );

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p1))
            .len(),
        their_hand,
        "the look at the target's hand is the `Coverage::Partial` gap — \
         nothing was taken, shown or moved"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library,
        "and nothing left the target's library either, so the draw went to \
         the caster and not to the seat that was pointed at"
    );
}
