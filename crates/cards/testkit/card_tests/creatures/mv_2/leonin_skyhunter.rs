//! `cards/creatures/mv_2/leonin_skyhunter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Leonin Skyhunter is `{W}{W}` for a 2/2 Cat Knight and one word of text:
/// flying. That word is only worth playing when somebody tries to block, and
/// the block offer here is a *pairing* — `BlockOption` lists which blocker may
/// be assigned to which attacker — so the same declared attacker is looked up
/// twice on one board: a ground Elf may not be paired with it and a Baleful
/// Strix may. The Skyhunter is cast with the two Plains and then waits a full
/// turn cycle, because CR 302.6 keeps a creature cast this turn out of the
/// attack declaration entirely, and an unoffered attacker proves nothing about
/// flying.
#[test]
fn leonin_skyhunter_flies_over_the_ground_and_is_blockable_only_by_a_flier() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(401, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[leonin_skyhunter()])
        .battlefield(1, &[llanowar_elves(), baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, leonin_skyhunter());
    pass_until(&mut engine, stack_is_empty);
    let hunter = on_battlefield(&engine, p0, leonin_skyhunter()).expect("the Skyhunter resolved");
    assert_eq!(pt(&engine, hunter), (2, 2), "the body the card prints");
    assert!(
        types(&engine, hunter).contains(TypeSet::CREATURE),
        "and it arrived as the creature it is"
    );
    assert!(
        keywords(&engine, hunter).contains(KeywordSet::FLYING),
        "the whole of its printed text is the keyword"
    );

    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("a ground blocker");
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("a flying blocker");
    assert!(
        !is_tapped(&engine, elves) && !is_tapped(&engine, strix),
        "both are untapped and so both are available to block"
    );

    // A creature cast this turn may not attack (CR 302.6), so the scenario
    // walks the opponent's turn and back rather than reading the keyword off
    // a permanent the combat step would never offer.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, hunter),
        "the untap step stood the Skyhunter back up"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&hunter),
        "the flier is offered as an attacker in its controller's turn: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(hunter, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Walk to the defending seat's declaration without answering it: `pass_until`
    // declares no blockers, which would resolve the combat before the pairing
    // could be read.
    for _ in 0..30 {
        if matches!(engine.pending(), Pending::ChooseBlockers { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "expected priority on the way to the block declaration, got {:?}",
                engine.pending()
            );
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the defending seat is asked how it blocks: {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p1, "and it is the seat being attacked");

    assert!(
        blockers
            .iter()
            .find(|o| o.blocker == elves)
            .is_none_or(|o| !o.attackers.contains(&hunter)),
        "CR 702.9b: a creature without flying cannot be paired with a flier: {blockers:?}"
    );
    let winged = blockers
        .iter()
        .find(|o| o.blocker == strix)
        .expect("a creature with flying is offered as a blocker at all");
    assert!(
        winged.attackers.contains(&hunter),
        "and flying is symmetric — the Strix is paired with the Skyhunter: {blockers:?}"
    );
}
