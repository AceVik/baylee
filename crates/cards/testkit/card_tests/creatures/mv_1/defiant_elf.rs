//! `cards/creatures/mv_1/defiant_elf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Defiant Elf is a {G} 1/1 Elf whose entire printed text is trample, so the
/// test plays the keyword instead of reading it: the Elf is cast, pumped to a
/// 4/4 and sent into a 1/1 blocker on its next turn, and the three damage that
/// land on the defending player are exactly the excess only trample assigns
/// (CR 702.19b). Two controls keep that number honest — the Elf across the
/// table carries no trample, and a block that assigned all four damage to the
/// blocker would leave the life total at twenty, which is the same reading an
/// unblocked attacker gives. The second turn is not padding: a creature that
/// entered this turn is summoning sick (CR 302.6) and could not attack at all.
#[test]
fn defiant_elf_tramples_its_excess_damage_over_the_creature_that_blocks_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[defiant_elf(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, defiant_elf());
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, p0, defiant_elf()).expect("the Elf resolved");
    assert_eq!(pt(&engine, elf), (1, 1), "the body the card prints");
    let kinds = types(&engine, elf);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "a creature and nothing else: {kinds:?}"
    );
    assert!(
        keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "the printed keyword reaches the permanent through the layers"
    );
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("the blocker stands");
    assert!(
        !keywords(&engine, blocker).contains(KeywordSet::TRAMPLE),
        "the keyword belongs to this card and not to the battlefield"
    );

    // The Elf just arrived, so the combat is on its controller's next turn —
    // and the pump has to happen there too, since +3/+3 lasts only until the
    // end of the turn it is cast in.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    cast_from_hand(&mut engine, p0, giant_growth());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "giant growth targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf),
        "the future attacker is a creature and so a legal target: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elf), (4, 4), "+3/+3 until end of turn");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .expect("an untapped 4/4 past summoning sickness may attack");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, elf)],
            },
        )
        .expect("a 1/1 blocks a 4/4 with no evasion");

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "one damage is lethal for the 1/1 and the other three trample over \
         it (CR 702.19b); twenty here would be a creature that assigned its \
         whole body to the blocker"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the blocker took lethal damage and died"
    );
}
