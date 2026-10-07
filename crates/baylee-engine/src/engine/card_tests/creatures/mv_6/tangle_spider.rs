//! `cards/creatures/mv_6/tangle_spider.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tangle Spider prints flash and reach on a {4}{G}{G} 3/4 Spider, and neither
/// keyword is readable in the card file. Flash is read as a comparison that
/// holds the mana still: in the opponent's main phase, with six green already
/// floating, the Spider is castable while a Llanowar Elves in the same hand is
/// not — a creature is a sorcery-speed spell otherwise, and a timing refusal
/// and a price refusal look identical from outside the offer. Reach is read off
/// the declare-blockers offer, where the Spider is named as a legal blocker of
/// a flying attacker and the untapped Elf beside it is not.
#[test]
fn tangle_spiders_flash_lands_it_on_the_opponents_turn_where_its_reach_blocks_a_flier() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Six Forests pay {4}{G}{G}, and the Elf is the printing kept back: it is
    // this test's ground control in the block declaration below, and a
    // creature tapped for mana may not block.
    let mut board: Vec<CardIndex> = vec![forest(); 6];
    board.push(llanowar_elves());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[tangle_spider(), llanowar_elves()])
        .battlefield(1, &[aven_brigadier()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().turn.active,
        p1,
        "the Spider is cast in the opponent's turn or this proves nothing"
    );
    // The active seat holds priority first (CR 117.3a), so it has to be passed
    // before the other seat may act at all.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    // Mana before the claim: `legal.castable` is filtered through `can_afford`,
    // which reads the pool and not the six untapped Forests.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six tapped Forests of green, and the Elf left standing"
    );
    let spider_card = in_hand(&engine, p0, tangle_spider()).expect("the Spider is in hand");
    let elf_card = in_hand(&engine, p0, llanowar_elves()).expect("the control Elf is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spider_card),
        "flash: {{4}}{{G}}{{G}} is affordable here, and the card is castable in \
         nobody's main phase but the active player's: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&elf_card),
        "and a creature without flash is a sorcery-speed spell: the Elf in the \
         same hand, with the same six green floating, is not castable at all: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, tangle_spider());
    pass_until(&mut engine, stack_is_empty);
    let spider = on_battlefield(&engine, p0, tangle_spider())
        .expect("the Spider resolved on the opponent's turn");
    assert_eq!(
        engine.state().turn.active,
        p1,
        "and it is on the battlefield before that turn has ended"
    );
    assert_eq!(pt(&engine, spider), (3, 4), "the body the card prints");

    // The flyer attacks, and the pairing the engine publishes is the only place
    // "can this creature block that one" is answered (CR 509.1).
    let flyer = on_battlefield(&engine, p1, aven_brigadier()).expect("the flier is out");
    assert!(
        keywords(&engine, flyer).contains(KeywordSet::FLYING),
        "the premise: the attacker this board is built around flies"
    );
    let blocks = attack_and_collect_blocks(&mut engine, flyer, p0);
    assert!(
        blocks
            .iter()
            .any(|option| option.blocker == spider && option.attackers.contains(&flyer)),
        "reach: the Spider is offered as a legal blocker of a flying attacker: {blocks:?}"
    );
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the ground control is out");
    assert!(
        !blocks
            .iter()
            .any(|option| option.blocker == elf && option.attackers.contains(&flyer)),
        "and the untapped Elf beside it is not, so the offer is read per \
         creature and is no grant handed to the whole board: {blocks:?}"
    );

    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!(
            "expected the declare-blockers question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the defending seat is the one that answers it");
    engine
        .apply(
            player,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spider, flyer)],
            },
        )
        .expect("the pairing the offer enumerated is the one the engine accepts");
}
