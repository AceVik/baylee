//! `cards/creatures/artifacts/mv_2/red_herring.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn red_herring() -> CardIndex {
    card_index("f0ee45b5-697b-48f8-b1a6-cb3ae75e4a29")
}

/// Red Herring: a 2/2 with "Haste" and "This creature attacks each combat if
/// able". Cast this turn, it may attack at once, and an empty attack is
/// refused where a declaration naming it is taken.
#[test]
fn red_herring_has_haste_and_must_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[red_herring()], &[mountain(), forest()], &[]);
    let herring = cast_and_land(&mut engine, red_herring());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, herring), (2, 2));
    assert!(keywords(&engine, herring).contains(KeywordSet::HASTE));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(attackers.contains(&herring), "haste: it can attack at once");
    assert!(
        engine
            .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "it attacks each combat if able"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(herring, Defender::Player(p1))],
            },
        )
        .expect("the declaration naming it is legal");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 1), 18);
}

/// "{2}, Sacrifice this creature: Draw a card."
#[test]
fn red_herring_sacrifices_itself_to_draw_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[red_herring(), forest(), forest()], &[]);
    let herring = on_battlefield(&engine, p0, red_herring()).expect("seated");
    assert!(
        !priority_offer(&engine).abilities.contains(&(herring, 1)),
        "no mana floating"
    );
    let hand = hand_count(&engine, 0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, red_herring(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, red_herring()).is_some());
    assert_eq!(hand_count(&engine, 0), hand + 1);
}
