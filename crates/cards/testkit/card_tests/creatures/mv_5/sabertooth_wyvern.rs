//! `cards/creatures/mv_5/sabertooth_wyvern.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sabertooth Wyvern is a 3/2 Drake with flying and first strike under `Coverage::Implemented` costing {4}{R}.
/// Its printed keywords project onto the battlefield permanent through the layer system.
/// In combat, only creatures with flying or reach can be declared as blockers against it.
/// An unblocked attack deals its 3 power directly to the defending player's life total.
#[test]
fn sabertooth_wyvern_has_flying_and_first_strike_and_attacks_with_evasion() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[sabertooth_wyvern()])
        .battlefield(1, &[air_elemental(), llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, sabertooth_wyvern());
    pass_until(&mut engine, stack_is_empty);

    let wyvern =
        on_battlefield(&engine, p0, sabertooth_wyvern()).expect("Sabertooth Wyvern resolved");
    assert_eq!(pt(&engine, wyvern), (3, 2), "printed body is 3/2");
    let kw = keywords(&engine, wyvern);
    assert!(
        kw.contains(KeywordSet::FLYING),
        "Sabertooth Wyvern has flying"
    );
    assert!(
        kw.contains(KeywordSet::FIRST_STRIKE),
        "Sabertooth Wyvern has first strike"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let flier = on_battlefield(&engine, p1, air_elemental()).expect("their flier is out");
    let blocks = attack_and_collect_blocks(&mut engine, wyvern, p1);

    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == flier && o.attackers.contains(&wyvern)),
        "the flier across the table is offered to block: {blocks:?}"
    );
    assert!(
        !blocks
            .iter()
            .any(|o| o.blocker == elf && o.attackers.contains(&wyvern)),
        "the non-flying creature cannot block a flier: {blocks:?}"
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
        17,
        "three combat damage dealt to defending player"
    );
    assert!(
        on_battlefield(&engine, p0, sabertooth_wyvern()).is_some(),
        "Sabertooth Wyvern survives combat"
    );
}
