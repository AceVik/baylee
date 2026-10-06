//! `cards/creatures/mv_2/bog_imp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bog Imp is `{1}{B}` for a 1/1 with flying and no other line, so the whole
/// card is the keyword: the body is a `(1, 1)`, the projected keyword set
/// holds `FLYING`, and — the half a card file cannot prove — a ground
/// creature across the table is offered as a blocker for a plain 1/1
/// attacking beside the Imp and *not* for the Imp itself.
///
/// The Elf is the control and not scenery. Without it, an empty offer for
/// the Imp would be satisfied just as well by a combat step that never
/// asked, or by a board on which nothing could block anything. The Imp is
/// cast on turn one and attacks on turn two, because CR 302.6 makes the
/// attack the one thing a creature cannot do the turn it arrives.
#[test]
fn bog_imp_is_a_one_one_flier_a_ground_creature_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), quiet_creature()])
        .hand(0, &[bog_imp()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1}{B} off the two Swamps, with the Elf named as the thing kept back:
    // it is this test's ground attacker below, and a creature tapped for
    // mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    cast_with_floating(&mut engine, p0, bog_imp());
    pass_until(&mut engine, stack_is_empty);

    let imp = on_battlefield(&engine, p0, bog_imp()).expect("the Imp resolved");
    let my_elf = on_battlefield(&engine, p0, quiet_creature()).expect("my Elf is out");
    let their_elf = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    assert_eq!(pt(&engine, imp), (1, 1), "the body the card prints");
    assert!(
        keywords(&engine, imp).contains(KeywordSet::FLYING),
        "the printed keyword reaches the permanent"
    );
    assert!(
        !keywords(&engine, their_elf).contains(KeywordSet::FLYING),
        "and the creature it has to walk past has no flying of its own"
    );

    // Through the opponent's turn and back: CR 302.6 keeps the Imp out of
    // this turn's combat, and nothing else in the scenario needs a second
    // turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        attackers.contains(&imp) && attackers.contains(&my_elf),
        "turn two, both untapped and neither sick: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(imp, Defender::Player(p1)), (my_elf, Defender::Player(p1))],
            },
        )
        .expect("both creatures the offer named may be declared");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    let offered = blockers
        .iter()
        .find(|option| option.blocker == their_elf)
        .expect("a ground 1/1 beside an attacking 1/1 is offered as a blocker");
    assert!(
        offered.attackers.contains(&my_elf),
        "the Elf may block the Elf: {offered:?}"
    );
    assert!(
        !offered.attackers.contains(&imp),
        "\"can't be blocked except by creatures with flying or reach\" — \
         neither of which stands across the table: {offered:?}"
    );
}
