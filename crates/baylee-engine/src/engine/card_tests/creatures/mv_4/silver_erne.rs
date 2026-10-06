//! `cards/creatures/mv_4/silver_erne.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Silver Erne is a 2/2 Bird with flying, trample, and `Coverage::Implemented` costing {3}{U}.
/// Its printed keywords reach the permanent through the layer system.
/// In combat, an opponent's flier can be declared as a blocker while a ground creature cannot.
/// When unblocked, its 2 power deals damage directly to the defending player's life total.
#[test]
fn silver_erne_has_flying_and_trample_and_attacks_with_evasion() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[silver_erne()])
        .battlefield(1, &[air_elemental(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, silver_erne());
    pass_until(&mut engine, stack_is_empty);

    let erne =
        on_battlefield(&engine, p0, silver_erne()).expect("Silver Erne is on the battlefield");
    assert_eq!(
        pt(&engine, erne),
        (2, 2),
        "printed power and toughness is 2/2"
    );
    let kw = keywords(&engine, erne);
    assert!(kw.contains(KeywordSet::FLYING), "Silver Erne has flying");
    assert!(kw.contains(KeywordSet::TRAMPLE), "Silver Erne has trample");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let flier = on_battlefield(&engine, p1, air_elemental()).expect("their flier is out");
    let blocks = attack_and_collect_blocks(&mut engine, erne, p1);

    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == flier && o.attackers.contains(&erne)),
        "the flying creature may block the attacking flier: {blocks:?}"
    );
    assert!(
        !blocks
            .iter()
            .any(|o| o.blocker == elf && o.attackers.contains(&erne)),
        "the non-flying creature cannot block a flying attacker: {blocks:?}"
    );

    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!("expected block choice, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().players[1].life != 20);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "two combat damage dealt to the defending player"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "attacking player life remains unchanged"
    );
    assert!(
        on_battlefield(&engine, p0, silver_erne()).is_some(),
        "unblocked attacker survives combat"
    );
}
