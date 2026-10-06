//! `cards/creatures/mv_3/rib_cage_spider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rib Cage Spider is `{2}{G}` for a 1/4 whose whole text is reach — "This
/// creature can block creatures with flying". Reach is not readable off a card
/// file at all: it lives in the block declaration, so the scenario seats a
/// flier across the table and reads the pairing the engine is willing to
/// offer. The Llanowar Elves beside it is the control — a creature with
/// neither reach nor flying must not be offered against that same attacker,
/// or the offer says "any creature" instead of "reach". Declaring the block
/// and following the attacker into its graveyard is what makes the offer the
/// block that happened rather than a menu entry.
#[test]
fn rib_cage_spider_uses_its_reach_to_block_a_flier_that_ground_creatures_cannot() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), rib_cage_spider(), llanowar_elves()])
        // A 1/1 flier, seated rather than cast so its own enters-trigger never
        // fires: the question here is the block, not the draw.
        .battlefield(1, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spider = on_battlefield(&engine, p0, rib_cage_spider()).expect("the Spider is deployed");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are deployed");
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("the Strix is deployed");
    assert_eq!(
        pt(&engine, spider),
        (1, 4),
        "the printed body and nothing added"
    );
    assert!(
        keywords(&engine, spider).contains(KeywordSet::REACH),
        "reach is the whole of what the card prints"
    );

    // Across p0's turn and into p1's, where the flier can attack.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&strix),
        "an untapped 1/1 flier may attack: {attackers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(strix, Defender::Player(p0))],
            },
        )
        .unwrap();

    // CR 508.2: priority is handed round once attackers are declared, so
    // the block is the next question and not this one.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the defending seat declares blockers, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and it is the seat being attacked");
    let allowed = blockers
        .iter()
        .find(|option| option.blocker == spider)
        .expect("reach is why the Spider is offered against a flier at all");
    assert!(
        allowed.attackers.contains(&strix),
        "the Spider may be paired with the flier: {:?}",
        allowed.attackers
    );
    assert!(
        !blockers
            .iter()
            .any(|option| option.blocker == elves && option.attackers.contains(&strix)),
        "the Elves have neither reach nor flying, so the same attacker must \
         not be offered to them"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spider, strix)],
            },
        )
        .expect("the pairing the engine offered is legal to declare");
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, baleful_strix()).is_some()
    });

    assert!(
        in_graveyard(&engine, p1, baleful_strix()).is_some(),
        "the blocked flier took the Spider's damage and died: the offer was a \
         block and not a menu entry"
    );
}
