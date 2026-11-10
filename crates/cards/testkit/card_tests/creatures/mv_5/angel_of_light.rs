//! `cards/creatures/mv_5/angel_of_light.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Angel of Light — {4}{W}, a 3/3 with "Flying, vigilance" and no other text.
/// Neither keyword is visible by reading the card: one is a pairing the
/// declare-blockers question publishes and the other is the absence of a tap
/// the attack declaration would otherwise make. So the Angel attacks beside a
/// plain 1/1 Elf, which is the control for both — the Elf is a legal blocker
/// for the ground attacker and no blocker at all for the Angel, and the Elf is
/// turned by the same declaration that leaves the Angel standing. A creature
/// that entered this turn may not attack (CR 302.6), which is why the attack
/// waits a turn and the mana pool is read on the turn the Angel is cast.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn angel_of_light_attacks_without_tapping_and_no_ground_creature_can_block_it() {
    fn angel_of_light() -> CardIndex {
        card_index("d12cf640-ff60-4935-a2c7-28ab788de42c")
    }

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Five Plains are exactly {4}{W}; the Elf is this test's control in the
    // attack declaration below.
    let mut board = vec![plains(); 5];
    board.push(llanowar_elves());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[angel_of_light()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elf is named as the printing kept back: it prints its own
    // `{T}: Add {G}` and `tap_all_mana` would have spent it (#159), leaving a
    // creature that can no longer attack — the very thing this test reads.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Plains tapped, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, angel_of_light());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast's {{4}}{{W}} came out of the pool"
    );

    let angel = on_battlefield(&engine, p0, angel_of_light()).expect("the Angel resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, angel), (3, 3), "the body the card prints");
    let granted = keywords(&engine, angel);
    assert!(granted.contains(KeywordSet::FLYING), "Flying");
    assert!(granted.contains(KeywordSet::VIGILANCE), "vigilance");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::VIGILANCE),
        "the Elf beside it is the control: it prints no vigilance and keeps none"
    );

    // CR 302.6: a creature that entered this turn may not attack, so the offer
    // that carries both keywords is the next turn's.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, angel),
        "the untap step left it standing"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&angel) && attackers.contains(&mine),
        "both untapped creatures may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(p1)), (mine, Defender::Player(p1))],
            },
        )
        .expect("both came out of the list that offered them");

    // Evasion is a pairing (CR 509.1a), so the blocks the defending seat is
    // offered are where "flying" is read rather than restated.
    let mut offered = None;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => {
                offered = Some(blockers);
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to the declare-blockers question: {other:?}"),
        }
    }
    let offered = offered.expect("the declare-blockers question is reached");
    assert!(
        offered
            .iter()
            .any(|option| option.blocker == theirs && option.attackers.contains(&mine)),
        "the Elf across the table can block the ground attacker, so the offer \
         is not simply empty: {offered:?}"
    );
    assert!(
        offered
            .iter()
            .all(|option| !option.attackers.contains(&angel)),
        "and no pairing in the whole offer names the Angel — that is flying, \
         because a 1/1 with no flying and no reach is no blocker for it: {offered:?}"
    );

    let life_before = engine.state().players[1].life;
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declaring no blockers is always legal");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        !is_tapped(&engine, angel),
        "vigilance: combat is over and the Angel was never turned by it"
    );
    assert!(
        is_tapped(&engine, mine),
        "while the Elf beside it, which prints no such word, is still down from \
         the same declaration — the untap step is the only thing that will stand \
         it back up"
    );
    assert_eq!(
        engine.state().players[1].life,
        life_before - 4,
        "both unblocked attackers connected: three from the 3/3 and one from the \
         1/1, and the life belongs to the seat that was attacked"
    );
    assert_eq!(
        engine.state().players[0].life,
        engine.state().players[0].life,
        "the attacking seat loses nothing for attacking"
    );
    assert!(
        on_battlefield(&engine, p0, angel_of_light()).is_some(),
        "and the Angel survived its first attack, so nothing here measured a \
         creature that had already left"
    );
}
