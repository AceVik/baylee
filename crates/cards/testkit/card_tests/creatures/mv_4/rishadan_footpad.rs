//! `cards/creatures/mv_4/rishadan_footpad.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a59f147b-3e03-4e82-aa34-48c220ea32e6"
fn rishadan_footpad() -> CardIndex {
    card_index("a59f147b-3e03-4e82-aa34-48c220ea32e6")
}

/// `Rishadan Footpad` (`Coverage::Generated`):
/// "When this creature enters, each opponent sacrifices a permanent unless they pay {2}."
///
/// Verifies both branches of `Rishadan Footpad`'s enters-the-battlefield trigger:
/// when the opponent declines to pay `{2}`, they sacrifice a permanent; when
/// they pay `{2}`, they keep all their permanents.
#[test]
fn rishadan_footpad_makes_opponent_sacrifice_a_permanent_unless_they_pay_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Case 1: Opponent declines to pay {2} -> sacrifices their permanent.
    let mut engine = Duel::new(9514, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[rishadan_footpad()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Grizzly Bears");
    cast_from_hand(&mut engine, p0, rishadan_footpad());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 2 },
                ..
            } if *player == p1
        )
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "P1 sacrificed Grizzly Bears when declining to pay {{2}}"
    );

    // Case 2: Opponent pays {2} -> keeps their permanent.
    let mut engine = Duel::new(9515, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[island(), island(), grizzly_bears()])
        .hand(0, &[rishadan_footpad()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, rishadan_footpad());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 2 },
                ..
            } if *player == p1
        )
    });
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_some(),
        "P1 kept Grizzly Bears after paying {{2}}"
    );
}
