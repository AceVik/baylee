//! `cards/creatures/mv_4/ancient_spider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ancient Spider is a `{2}{G}{W}` 2/5 whose entire printed text is two combat
/// keywords — reach and first strike — so nothing about it can be read off the
/// card file and everything has to be played. It is cast for real and then
/// blocks the only attacker that is worth blocking: a Baleful Strix, which is a
/// flier, while a reach-less Llanowar Elves standing beside it is offered for
/// that same attacker by nobody — the word "reach" read out of the engine's own
/// pairing list. Resolving the block is the second half: the Spider's two
/// damage land in the first-strike combat damage step (CR 702.7), so the 1/1
/// dies while the 2/5 is still standing and never answers, which is what tells
/// a creature with first strike from one that merely has two power.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn ancient_spider_blocks_a_flier_with_reach_and_kills_it_before_it_can_answer() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), forest(), forest(), llanowar_elves()],
        )
        .battlefield(1, &[baleful_strix()])
        .hand(0, &[ancient_spider()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Two Plains and two Forests pay `{2}{G}{W}`, and the Elf is named as the
    // printing kept back: it is this test's control below, and a creature
    // tapped for mana is a creature that could not have blocked either.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Plains and two Forests, with the Elf's own {{G}} not among them"
    );
    cast_with_floating(&mut engine, p0, ancient_spider());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, ancient_spider()).is_some() && stack_is_empty(e)
    });

    let spider = on_battlefield(&engine, p0, ancient_spider()).expect("the Spider resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still standing");
    let strix =
        on_battlefield(&engine, p1, baleful_strix()).expect("the flier is across the table");
    assert_eq!(pt(&engine, spider), (2, 5), "the body the card prints");
    let printed = keywords(&engine, spider);
    assert!(
        printed.contains(KeywordSet::REACH),
        "\"Reach (This creature can block creatures with flying.)\" reaches \
         the permanent that is actually in play"
    );
    assert!(
        printed.contains(KeywordSet::FIRST_STRIKE),
        "and so does the other printed keyword"
    );

    // p1's turn, and the flier is the only attacker this scenario is about:
    // nothing is reach and nothing is first strike until something is blocked.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&strix),
        "an untapped 1/1 flier past summoning sickness may attack: {attackers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(strix, Defender::Player(p0))],
            },
        )
        .expect("the attacker the engine offered is a legal declaration");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the blocker declaration")
    };
    assert_eq!(player, p0, "the seat being attacked declares the blocks");
    let may_block_the_flier = |id: ObjectId| {
        blockers
            .iter()
            .any(|o| o.blocker == id && o.attackers.contains(&strix))
    };
    assert!(
        may_block_the_flier(spider),
        "reach is a pairing, and this is the half the engine has to hand back: \
         {blockers:?}"
    );
    assert!(
        !may_block_the_flier(elf),
        "a creature with neither reach nor flying cannot block a flier, which \
         is what says the line above is the Spider's and not the board's: \
         {blockers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spider, strix)],
            },
        )
        .expect("the pair the blocker question offered is a legal declaration");

    // The end step is past the combat damage step (CR 510.2), so the whole
    // combat is settled behind this predicate.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, baleful_strix()).is_some(),
        "two damage to a 1/1 is lethal"
    );
    assert!(
        on_battlefield(&engine, p0, ancient_spider()).is_some(),
        "\"First strike\": the Spider's damage is dealt in the first-strike \
         combat damage step, so the Strix is already gone when its own damage \
         would be dealt, and the 2/5 blocking it is still standing"
    );
    assert!(
        !is_tapped(&engine, spider),
        "blocking is not an activated ability and taps nothing"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the flier never reached the player it attacked"
    );
}
