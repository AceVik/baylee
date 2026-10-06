//! `cards/creatures/mv_2/sea_eagle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sea Eagle is a {1}{U} 1/1 Bird whose whole printed text is flying, so the
/// test has to play it twice: once as a spell, where the cost, the body and
/// the keyword can be read off the permanent it becomes, and once in combat,
/// because flying is an evasion rule and a keyword assertion on its own would
/// pass on a board where nothing ever tried to block. The two Islands pay
/// {1}{U} while the Llanowar Elves beside them is kept back, so the Elf can
/// attack a turn later and the defending Elf's block menu carries the ground
/// attacker and not the Bird — which is the difference between a projected
/// keyword and a keyword the game was asked to use.
#[test]
fn sea_eagle_arrives_as_a_one_one_flier_a_ground_elf_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), llanowar_elves()])
        .hand(0, &[sea_eagle()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elf beside the Bird is not mana for it: it is the ground attacker
    // whose block menu makes the flying mean something below, so it is the one
    // source kept standing while the two Islands fill the pool.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands, and the {{1}}{{U}} the Bird prints is payable"
    );
    cast_with_floating(&mut engine, p0, sea_eagle());
    pass_until(&mut engine, stack_is_empty);

    let eagle = on_battlefield(&engine, p0, sea_eagle()).expect("the Eagle resolved");
    assert_eq!(pt(&engine, eagle), (1, 1), "the body the card prints");
    assert!(
        types(&engine, eagle).contains(TypeSet::CREATURE),
        "it is the creature it prints: {:?}",
        types(&engine, eagle)
    );
    assert!(
        keywords(&engine, eagle).contains(KeywordSet::FLYING),
        "the one line of text the card has"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{U}} came out of the pool the Islands filled"
    );

    // A turn each way: a creature cast on a turn may not attack until its
    // controller's next one (CR 302.6), and the block question below belongs
    // to the turn after that.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(eagle, Defender::Player(p1)), (elf, Defender::Player(p1))],
            },
        )
        .expect("an untapped flier and an untapped Elf may both attack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p1, "the defending seat declares blockers");
    let may_block = |attacker: ObjectId| {
        blockers
            .iter()
            .filter(|option| option.blocker == blocker)
            .any(|option| option.attackers.contains(&attacker))
    };
    assert!(
        may_block(elf),
        "the ground Elf is offered against the ground attacker, so the empty \
         half below is flying and not a blocker that never had a chance: {blockers:?}"
    );
    assert!(
        !may_block(eagle),
        "and it is never offered against the Bird, which is `CR 702.9b` and \
         the whole of what the card prints: {blockers:?}"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().players[1].life < 20);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "an unblocked 1/1 flier and an unblocked 1/1 Elf: two damage"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing came back, so the damage read above belongs to the \
         attackers this turn and not to an earlier one"
    );
}
