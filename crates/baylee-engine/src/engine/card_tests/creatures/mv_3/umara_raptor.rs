//! `cards/creatures/mv_3/umara_raptor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// CR 608.2h: an effect that needs information about an object no longer in
/// the zone it was expected to be in uses that object's last known
/// information.
///
/// Swords to Plowshares is two effects in one sentence — exile the creature,
/// *then* read its power — so the second half asks about an object the first
/// half moved. It read the printed card and paid one life for a 2/2.
#[test]
fn swords_reads_the_creature_it_exiled_as_it_last_stood() {
    let p0 = PlayerId::new(0);
    let mut engine = a_two_two_raptor(41, swords_to_plowshares(), &[]);
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor is out");
    let life_before = engine.state().players[0].life;

    let swords = in_hand(&engine, p0, swords_to_plowshares()).expect("the sword is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: swords })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bird],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, umara_raptor()).is_none(),
        "the Raptor was exiled",
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "the power it had on the battlefield, not the 1 its card prints",
    );
}

// oracle_id = "7744bae4-a8b7-44a5-9b4c-0048ad4cc448"

/// Air Elemental is `{3}{U}{U}` for a 4/4 Elemental whose entire printed text
/// is "Flying", so the body and the keyword are the whole card and both have to
/// be read off a permanent that got there by being cast. The keyword is worth
/// playing rather than reading, because a block declaration is exactly what a
/// text line like that decides: the opponent keeps an untapped ground creature
/// and an untapped flier, and only the flier may be offered against the
/// Elemental. The ground creature is the control an empty offering cannot
/// supply on its own — "no blockers" is also what a combat step that never came
/// reads as.
#[test]
fn air_elemental_lands_as_a_four_four_that_only_a_flier_may_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[llanowar_elves(), umara_raptor()])
        .hand(0, &[air_elemental()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Five Islands pay {3}{U}{U} exactly, so the Elemental stands on the table
    // because the card was cast and not because a board was seeded with it.
    cast_from_hand(&mut engine, p0, air_elemental());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, air_elemental()).is_some()
    });
    let elemental = on_battlefield(&engine, p0, air_elemental()).expect("the Elemental resolved");
    assert_eq!(pt(&engine, elemental), (4, 4), "the body the card prints");
    assert!(
        types(&engine, elemental).contains(TypeSet::CREATURE),
        "and what arrived is a creature"
    );
    assert!(
        keywords(&engine, elemental).contains(KeywordSet::FLYING),
        "the card's whole printed text reaches the permanent"
    );

    // CR 302.6: a creature that arrived this turn has not been under its
    // controller's control since their turn began, so the attack is a turn
    // later — across the opponent's turn and back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is still out");
    let raptor = on_battlefield(&engine, p1, umara_raptor()).expect("and so is their flier");
    let offered = attack_and_collect_blocks(&mut engine, elemental, p1);

    assert!(
        offered
            .iter()
            .any(|b| b.blocker == raptor && b.attackers.contains(&elemental)),
        "the flier across the table may block a flier: {offered:?}"
    );
    assert!(
        !offered.iter().any(|b| b.blocker == elves),
        "the untapped Elf is on the battlefield and is no legal blocker for a \
         creature with flying: {offered:?}"
    );
}

/// Report 01a16cdd: a Lieutenant returning from Ephemerate pumps the Raptor
/// once itself, and gives it another counter as another Ally entering.
#[test]
fn a_returned_lieutenant_raises_the_raptor_to_four_four() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                earth_king_s_lieutenant(),
                island(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[umara_raptor(), ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, umara_raptor());
    pass_until(&mut engine, stack_is_empty);
    let raptor = on_battlefield(&engine, p0, umara_raptor()).expect("the Raptor resolved");
    let lieutenant = on_battlefield(&engine, p0, earth_king_s_lieutenant())
        .expect("the Lieutenant was already there");
    assert_eq!(pt(&engine, raptor), (2, 2), "its own rally trigger");
    assert_eq!(
        pt(&engine, lieutenant),
        (2, 2),
        "the Lieutenant's own rally trigger"
    );

    let spell = in_hand(&engine, p0, ephemerate()).expect("Ephemerate is in hand");
    let pending = engine.pending().clone();
    assert!(
        matches!(pending, Pending::Priority { ref legal, .. } if legal.castable.contains(&spell)),
        "Ephemerate is castable before it is tapped for: {pending:?}, pool {:?}",
        engine.state().players[0].mana_pool
    );
    cast_from_hand(&mut engine, p0, ephemerate());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Ephemerate asks for a creature: {:?}", engine.pending())
    };
    assert!(
        options.contains(&lieutenant),
        "the Lieutenant is a legal target: {options:?} vs {lieutenant:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![lieutenant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let returned =
        on_battlefield(&engine, p0, earth_king_s_lieutenant()).expect("the Lieutenant returned");
    assert_eq!(
        pt(&engine, returned),
        (1, 1),
        "it returns without its old counters"
    );
    assert_eq!(
        pt(&engine, raptor),
        (4, 4),
        "one counter for the Lieutenant's arrival and one from its own ETB"
    );
}
