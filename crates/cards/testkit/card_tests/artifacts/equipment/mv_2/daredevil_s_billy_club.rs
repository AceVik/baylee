//! `cards/artifacts/equipment/mv_2/daredevil_s_billy_club.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn daredevil_s_billy_club() -> CardIndex {
    card_index("654db13d-c7e4-4ee0-a87a-3635467f2f19")
}

/// Daredevil's Billy Club: "Equipped creature gets +1/+0 and has menace.
/// Equip {2}".
#[test]
fn billy_club_gives_plus_one_power_and_menace() {
    equipped_gets(
        daredevil_s_billy_club(),
        2,
        llanowar_elves(),
        (1, 0),
        KeywordSet::MENACE,
    );
}

/// "When this Equipment enters, it deals 2 damage to any target and 1 damage
/// to you." Aimed at the opponent: they lose two, we lose one.
#[test]
fn billy_club_entering_deals_two_to_a_player_and_one_to_you() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[daredevil_s_billy_club()])
        .battlefield(0, &[mountain(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, daredevil_s_billy_club());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the question")
    };
    assert!(
        player_options.contains(&p1),
        "\"any target\" offers the opponent: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "two damage to the target"
    );
    assert_eq!(engine.state().players[0].life, 19, "one damage to you");
    assert!(
        on_battlefield(&engine, p0, daredevil_s_billy_club()).is_some(),
        "the Equipment stays"
    );
}

/// The same trigger aimed at a creature kills a 1/1 and still costs us one.
#[test]
fn billy_club_entering_can_kill_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[daredevil_s_billy_club()])
        .battlefield(0, &[mountain(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");
    cast_from_hand(&mut engine, p0, daredevil_s_billy_club());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the question")
    };
    assert!(
        options.contains(&elf),
        "a creature is a legal target: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "two damage kills the Elf"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the player was not the target"
    );
    assert_eq!(engine.state().players[0].life, 19, "one damage to you");
}
