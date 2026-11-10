//! `cards/lands/utility/rogue_s_passage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rogue's Passage: "{4}, {T}: Target creature can't be blocked this turn."
///
/// `KeywordSet::UNBLOCKABLE` was read by `combat::can_block` and granted by
/// no card in the pool, so the rule had never been exercised from a card.
/// The control is inside the test rather than beside it: seat 0 attacks with
/// two creatures and only one of them was pointed at, so an empty offer —
/// which a blocker that simply could not block would also produce — is not
/// what this asserts.
#[test]
fn rogue_s_passage_takes_its_target_out_of_the_blockers_offer() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(17, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                rogue_s_passage(),
                llanowar_elves(),
                ondu_cleric(),
            ],
        )
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves deployed");
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("cleric deployed");

    // Both of seat 0's creatures are summoning sick on turn 1, so the attack
    // is on turn 3.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::UNBLOCKABLE),
        "nothing has been activated yet"
    );

    // Four Forests, and two things kept back by name: the Passage, which has
    // to pay its own {T} below, and the Elf, which has to be untapped to
    // attack. This used to read "the Passage is not a basic land, so the
    // CR 305.6 shortcut leaves it untapped" — the helper's defect written
    // down as a feature (#159).
    let passage = on_battlefield(&engine, p0, rogue_s_passage()).expect("the Passage is out");
    tap_mana_where(&mut engine, p0, |id| id != passage && id != elves);
    activate(&mut engine, p0, rogue_s_passage(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability targets a creature, got {:?}", engine.pending())
    };
    assert!(options.contains(&elves), "any creature is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        keywords(e, elves).contains(KeywordSet::UNBLOCKABLE)
    });
    assert!(
        !keywords(&engine, cleric).contains(KeywordSet::UNBLOCKABLE),
        "the grant names the target and nothing else"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (elves, Defender::Player(p1)),
                    (cleric, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    let blockers = loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => break blockers,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while reaching blockers: {other:?}"),
        }
    };
    assert_eq!(blockers.len(), 1, "seat 1 has exactly one creature");
    assert_eq!(
        blockers[0].attackers,
        vec![cleric],
        "the Passage's target is not among the attackers it may be paired with"
    );
}
