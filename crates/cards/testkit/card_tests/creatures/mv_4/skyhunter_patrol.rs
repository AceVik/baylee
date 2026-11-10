//! `cards/creatures/mv_4/skyhunter_patrol.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "aadcff6f-9207-4d90-a12d-4913c96867e2"

/// Skyhunter Patrol is a vanilla 2/3 Cat Knight whose whole rules text is two
/// keywords, so both are read off a live board rather than off the card file.
/// Four Plains pay its {{2}}{{W}}{{W}} down to an empty pool, the permanent
/// that arrives projects the printed body with Flying and first strike, and a
/// turn later the block offer names the opponent's flier against it while the
/// ground Elf beside that flier cannot block it at all — the flier is the
/// positive half that says the pairing list was really published and not
/// merely empty, and the unblocked attack then puts the Patrol's two power on
/// the defending seat.
#[test]
fn skyhunter_patrol_is_a_flying_first_striker_only_a_flier_may_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[skyhunter_patrol()])
        // A flier and a ground creature across the table, so the block offer
        // below has both a positive half and a negative one.
        .battlefield(1, &[air_elemental(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{W}{W} off the four Plains and nothing else, so the emptied pool is
    // what says the cost was really paid rather than the creature arriving on
    // its own.
    cast_from_hand(&mut engine, p0, skyhunter_patrol());
    pass_until(&mut engine, stack_is_empty);
    let patrol = on_battlefield(&engine, p0, skyhunter_patrol()).expect("the Patrol resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Plains paid {{2}}{{W}}{{W}} to the last mana"
    );
    assert!(
        types(&engine, patrol).contains(TypeSet::CREATURE),
        "what arrived is the creature it prints"
    );
    assert_eq!(pt(&engine, patrol), (2, 3), "and the body it prints");
    let printed = keywords(&engine, patrol);
    assert!(
        printed.contains(KeywordSet::FLYING),
        "Flying reaches the permanent: {printed:?}"
    );
    assert!(
        printed.contains(KeywordSet::FIRST_STRIKE),
        "and so does first strike: {printed:?}"
    );

    // A creature that entered this turn may not attack (CR 302.6), so the
    // combat that reads the evasion is its controller's next turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let flier = on_battlefield(&engine, p1, air_elemental()).expect("their flier is out");
    let blocks = attack_and_collect_blocks(&mut engine, patrol, p1);
    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == flier && o.attackers.contains(&patrol)),
        "the flier across the table is offered against it, which is what says \
         the pairing list was published at all: {blocks:?}"
    );
    assert!(
        !blocks
            .iter()
            .any(|o| o.blocker == elf && o.attackers.contains(&patrol)),
        "\"can't be blocked except by creatures with flying or reach\" — the \
         ground Elf has neither: {blocks:?}"
    );

    // Nobody blocks, so the flier connects: the offer alone cannot show that
    // the attacker is a real body that gets through to the seat.
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!("expected the block declaration, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().players[1].life != 20);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "two damage from the 2/3 flier nobody blocked"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing came back across the table"
    );
    assert!(
        on_battlefield(&engine, p0, skyhunter_patrol()).is_some(),
        "an unblocked attacker survives its own combat"
    );
}
