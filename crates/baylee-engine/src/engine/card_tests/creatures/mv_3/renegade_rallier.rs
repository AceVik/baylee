//! `cards/creatures/mv_3/renegade_rallier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Renegade Rallier — {1}{G}{W} 3/2 Human Warrior. Its printed enter trigger
/// is a revolt trigger: return a permanent card with mana value 2 or less from
/// your graveyard to the battlefield only "if a permanent left the battlefield
/// under your control this turn". That intervening-if (CR 603.4) is the
/// `Coverage::Partial` gap and the reason this board is built to leave nothing:
/// two Forests sit in the graveyard and a Dark Ritual has just resolved into
/// it, so the Rallier's own arrival is the turn's only event — and the trigger
/// asks for a target anyway.
///
/// The menu is the second half of the proof: the two Forests are offered and
/// the instant is not, so "permanent card" (CR 110.4a) is read and not merely
/// the mana-value cap.
#[test]
fn renegade_rallier_reanimates_a_cheap_permanent_card_with_nothing_having_left_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[forest(), forest(), plains(), swamp()])
        .hand(0, &[renegade_rallier(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Two cards the trigger may take, off the top of a deck of Forests.
    seed_graveyard(&mut engine, p0, 2);

    // And one it may not. Nothing has left the battlefield to pay for this
    // either: an instant resolving is a card changing zones, not a permanent.
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let ritual = in_graveyard(&engine, p0, dark_ritual())
        .expect("Dark Ritual resolved and went to its owner's graveyard");

    let forests_before = all_on_battlefield(&engine, p0, forest()).len();
    let board_before = engine.state().zones.list(ZoneLocation::Battlefield).clone();

    cast_from_hand(&mut engine, p0, renegade_rallier());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    // Revolt's condition is not merely unread on this board: every permanent
    // that was here before is still here, and the Rallier is the only addition.
    let now = engine.state().zones.list(ZoneLocation::Battlefield);
    assert!(
        board_before.iter().all(|id| now.contains(id)),
        "no permanent left the battlefield under p0's control this turn"
    );
    assert_eq!(
        now.len(),
        board_before.len() + 1,
        "the Rallier's own arrival is the only thing that has happened"
    );

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the Rallier's controller is the one asked");
    assert_eq!((min, max), (1, 1), "exactly one card comes back");
    assert!(
        player_options.is_empty(),
        "\"target permanent card\" reaches no player: {player_options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "the two Forests: lands, mana value 0, and nothing else: {options:?}"
    );
    assert!(
        !options.contains(&ritual),
        "Dark Ritual is an instant, so it is no permanent card (CR 110.4a): {options:?}"
    );

    let taken = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![taken],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let landed = engine
        .state()
        .object(taken)
        .expect("the returned card is still an object");
    assert_eq!(
        landed.zone,
        crate::zone::Zone::Battlefield,
        "\"return target permanent card ... to the battlefield\""
    );
    assert_eq!(
        landed.controller, p0,
        "under the control of the seat that cast it"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        forests_before + 1,
        "one Forest came back out of the graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_some(),
        "and the instant the menu refused stayed exactly where it was"
    );

    let rallier =
        on_battlefield(&engine, p0, renegade_rallier()).expect("the Rallier is on the table");
    assert_eq!(pt(&engine, rallier), (3, 2), "and it is the printed 3/2");
}
