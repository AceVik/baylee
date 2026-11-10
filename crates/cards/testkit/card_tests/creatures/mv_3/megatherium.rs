//! `cards/creatures/mv_3/megatherium.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "b98123b2-c913-41b9-bb24-10a287d24bca"
fn megatherium() -> CardIndex {
    card_index("b98123b2-c913-41b9-bb24-10a287d24bca")
}

/// `Megatherium` (`Coverage::Generated`):
/// "Trample. When this creature enters, sacrifice it unless you pay {1} for each card in your hand."
///
/// Verifies both paying `{1}` per card in hand to keep `Megatherium` on the
/// battlefield and declining to pay so it is sacrificed.
#[test]
fn megatherium_requires_paying_one_per_card_in_hand_or_sacrifices_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9408, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[megatherium(), megatherium(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    // Cast first Megatherium with 2 cards remaining in hand; pay {2} to keep it.
    cast_from_hand(&mut engine, p0, megatherium());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 2 },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let beast = on_battlefield(&engine, p0, megatherium()).expect("Megatherium stayed");
    assert_eq!(pt(&engine, beast), (4, 4));
    assert!(keywords(&engine, beast).contains(KeywordSet::TRAMPLE));

    // Next turn: cast second Megatherium and decline payment -> sacrificed.
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));
    cast_from_hand(&mut engine, p0, megatherium());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p0, megatherium()).len(),
        1,
        "declining payment sacrifices the entering Megatherium"
    );
    assert!(in_graveyard(&engine, p0, megatherium()).is_some());
}
