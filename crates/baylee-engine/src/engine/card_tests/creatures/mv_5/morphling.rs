//! `cards/creatures/mv_5/morphling.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Morphling — {3}{U}{U}, a 3/3 Shapeshifter whose whole text is five
/// one-line abilities: {U} to untap it, {U} each for flying and shroud, and
/// {1} for each half of the +1/-1 / -1/+1 swap. Four of the five are played
/// in the turn it lands, off a single pool of nine Islands — five pay the
/// cast and four pay the lines — so the pool reading exactly zero afterwards
/// is the statement that every price was really paid. The Elf beside it is
/// the control for `Filter::This`: a creature under the same seat must stay a
/// printed 1/1 with neither keyword. The untap, which needs a tapped
/// creature, costs a turn — no longer summoning sick, Morphling attacks and
/// one blue stands it back up — and the turn in between is what reads the
/// printed "until end of turn" off the flying, the shroud and both pumps.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn morphling_plays_all_five_of_its_lines_off_one_pool_of_islands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Nine Islands is exactly the whole card: {3}{U}{U} for the body and the
    // four one-mana lines beside it.
    let mut board: Vec<CardIndex> = vec![island(); 9];
    board.push(llanowar_elves());
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &board)
        .hand(0, &[morphling()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elf is named as the printing kept back: it prints a `{T}: Add {G}`
    // of its own (#159), and the only creature this test pumps is Morphling.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "nine Islands tapped, and the Elf contributed nothing"
    );

    cast_with_floating(&mut engine, p0, morphling());
    pass_until(&mut engine, stack_is_empty);
    let shifter = on_battlefield(&engine, p0, morphling()).expect("Morphling resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still out");
    assert_eq!(pt(&engine, shifter), (3, 3), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "five of the nine went into {{3}}{{U}}{{U}}"
    );

    // Ability 1 — "{U}: This creature gains flying until end of turn."
    activate(&mut engine, p0, morphling(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, shifter).contains(KeywordSet::FLYING),
        "\"This creature gains flying until end of turn\""
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "one blue for the flying"
    );

    // Ability 2 — "{U}: This creature gains shroud until end of turn."
    activate(&mut engine, p0, morphling(), 2);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, shifter).contains(KeywordSet::SHROUD),
        "\"This creature gains shroud until end of turn\""
    );
    assert!(
        keywords(&engine, shifter).contains(KeywordSet::FLYING),
        "the two keyword lines stack on one creature"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "one blue for the shroud, and the flying is still up"
    );

    // Ability 3 — "{1}: This creature gets +1/-1 until end of turn."
    activate(&mut engine, p0, morphling(), 3);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, shifter), (4, 2), "+1/-1 on the printed 3/3");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one generic for the power half"
    );

    // Ability 4 — "{1}: This creature gets -1/+1 until end of turn."
    activate(&mut engine, p0, morphling(), 4);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, shifter),
        (3, 3),
        "the two halves cancel on the same creature"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nine Islands: five for the cast and four for the four lines"
    );

    // "This creature" and nobody else: the Elf beside it is a creature under
    // the same seat and no line has touched it.
    assert_eq!(pt(&engine, elf), (1, 1), "the Elf is a printed 1/1");
    let untouched = keywords(&engine, elf);
    assert!(
        !untouched.contains(KeywordSet::FLYING) && !untouched.contains(KeywordSet::SHROUD),
        "the keywords landed on Morphling and on no other creature: {untouched:?}"
    );

    // The printed durations, read where they end: the same permanent a turn
    // later cannot still be flying, shrouded, pumped, or all three.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, shifter),
        (3, 3),
        "both pumps expired with the turn"
    );
    let expired = keywords(&engine, shifter);
    assert!(
        !expired.contains(KeywordSet::FLYING) && !expired.contains(KeywordSet::SHROUD),
        "and so did the flying and the shroud: {expired:?}"
    );

    // The fifth line needs a tapped creature, which costs a turn: Morphling
    // was cast last turn, so it is no longer summoning sick (CR 302.6).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the active seat declares its own attackers");
    assert!(
        attackers.contains(&shifter),
        "a turn past its arrival the Shapeshifter may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shifter, Defender::Player(p1))],
            },
        )
        .expect("the creature came out of the list that offered it");

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        is_tapped(&engine, shifter),
        "declaring it as an attacker tapped it (CR 508.1f)"
    );

    // Mana first, then the claim: `legal.abilities` is filtered through
    // `can_afford`, which reads the pool and not the untapped Islands.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "the untap step stood every Island back up"
    );

    // Ability 0 — "{U}: Untap this creature."
    activate(&mut engine, p0, morphling(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, shifter),
        "\"{{U}}: Untap this creature\""
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "one blue paid for the untap"
    );
}
