//! `cards/creatures/mv_3/rishadan_cutpurse.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "147e0e74-f76b-4f0a-951f-d39afdd72c02"
fn rishadan_cutpurse() -> CardIndex {
    card_index("147e0e74-f76b-4f0a-951f-d39afdd72c02")
}

/// `Rishadan Cutpurse` (`Coverage::Generated`):
/// "When this creature enters, each opponent sacrifices a permanent unless they pay {1}."
///
/// Verifies both branches of `Rishadan Cutpurse`'s enters-the-battlefield trigger:
/// when the opponent declines to pay `{1}`, they must sacrifice a permanent;
/// when the opponent pays `{1}`, they keep all their permanents.
#[test]
fn rishadan_cutpurse_makes_opponent_sacrifice_a_permanent_unless_they_pay_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Case 1: Opponent declines to pay {1} -> sacrifices their permanent.
    let mut engine = Duel::new(9510, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[rishadan_cutpurse()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Grizzly Bears");
    cast_from_hand(&mut engine, p0, rishadan_cutpurse());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 1 },
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
        "P1 sacrificed Grizzly Bears when declining to pay {{1}}"
    );

    // Case 2: Opponent pays {1} -> keeps their permanent.
    let mut engine = Duel::new(9511, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[island(), grizzly_bears()])
        .hand(0, &[rishadan_cutpurse()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, rishadan_cutpurse());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 1 },
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
        "P1 kept Grizzly Bears after paying {{1}}"
    );
}
