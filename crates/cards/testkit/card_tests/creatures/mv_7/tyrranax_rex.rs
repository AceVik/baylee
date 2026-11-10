//! `cards/creatures/mv_7/tyrranax_rex.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tyrranax Rex: ward {4} charges an opponent for the privilege.
///
/// `AbilityDef::Ward` is a triggered ability that counters the spell unless
/// its controller pays, so the played proof is the question: the opponent is
/// asked for `{4}` they cannot pay off one Plains, and the Rex is still
/// standing when the dust settles.
#[test]
fn tyrranax_rex_wards_an_opponents_removal_for_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(395, forest())
        .battlefield(0, &[tyrranax_rex()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rex = on_battlefield(&engine, p0, tyrranax_rex()).expect("the Rex is seated");
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&rex),
        "ward does not stop the targeting, only charges for it"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![rex],
                players: vec![],
            },
        )
        .expect("the Rex is a legal target");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the ward trigger is answered on the way"
    );

    assert!(
        on_battlefield(&engine, p0, tyrranax_rex()).is_some(),
        "one Plains cannot pay {{4}}, so the spell was countered by ward and \
         the Rex is still there"
    );
}

/// Tyrranax Rex: "Toxic 4 (Players dealt combat damage by this creature also
/// get four poison counters.)" Unblocked, the Rex deals eight: the defending
/// player loses eight life **and** gets four poison counters, and the
/// attacking player gets none.
#[test]
fn tyrranax_rex_gives_four_poison_counters_with_its_combat_damage() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(397, forest())
        .battlefield(0, &[tyrranax_rex()])
        .start();
    keep_mulligans(&mut engine);
    let rex = on_battlefield(&engine, p0, tyrranax_rex()).expect("the Rex is seated");
    let life = engine.state().players[1].life;

    rex_attacks(&mut engine, rex, vec![]);

    assert_eq!(
        engine.state().players[1].life,
        life - 8,
        "the damage's other result still happens"
    );
    assert_eq!(
        engine.state().players[1].poison,
        4,
        "and the player dealt combat damage gets four poison counters"
    );
    assert_eq!(
        engine.state().players[0].poison,
        0,
        "its controller gets none"
    );
}

/// Toxic is about damage dealt **to a player** (CR 702.164c). Blocked by
/// the opponent's own Rex, the two deal their damage to each other and
/// nobody gets a poison counter, though both creatures have toxic.
#[test]
fn tyrranax_rex_blocked_gives_nobody_poison() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(398, forest())
        .battlefield(0, &[tyrranax_rex()])
        .battlefield(1, &[tyrranax_rex()])
        .start();
    keep_mulligans(&mut engine);
    let rex = on_battlefield(&engine, p0, tyrranax_rex()).expect("p0's Rex");
    let theirs = on_battlefield(&engine, p1, tyrranax_rex()).expect("p1's Rex");
    let life = engine.state().players[1].life;

    rex_attacks(&mut engine, rex, vec![(theirs, rex)]);

    assert_eq!(
        engine.state().players[1].life,
        life,
        "eight to eight: nothing tramples over"
    );
    assert_eq!(
        engine.state().players[1].poison,
        0,
        "no damage to p1, no poison"
    );
    assert_eq!(engine.state().players[0].poison, 0, "nor to p0");
}
