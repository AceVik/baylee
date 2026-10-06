//! `cards/creatures/mv_7/archangel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Archangel is {5}{W}{W} for a 5/5 Angel whose entire printed text is
/// "Flying, vigilance". A keyword is the one characteristic that keeps reading
/// correctly while the rest of a card is missing, so the test plays the card
/// into a real turn: the Angel is cast off lands that really paid for it and
/// resolves onto the battlefield, an unequipped Elf beside it is the control
/// that shows the two keywords belong to this creature rather than to the
/// board, and the attack declaration on the next turn is what reads vigilance —
/// the same attack that leaves the Elf tapped leaves the Angel standing.
#[test]
fn archangel_lands_as_a_five_five_angel_that_attacks_without_tapping() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                quiet_creature(),
            ],
        )
        .hand(0, &[archangel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `castable` is filtered through the mana pool, so an empty pool pays no
    // {5}{W}{W} and the Angel is not offered until the lands are tapped.
    let card = in_hand(&engine, p0, archangel()).expect("the Archangel is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{5}}{{W}}{{W}}, so the Angel is not offered: {:?}",
        legal.castable
    );

    // The Elf is named as the printing kept back: it is the creature that
    // carries the control in the combat step below, and a body tapped for mana
    // may not attack.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Plains tapped and the Elf held back: seven white for {{5}}{{W}}{{W}}"
    );
    cast_with_floating(&mut engine, p0, archangel());
    pass_until(&mut engine, stack_is_empty);

    let angel = on_battlefield(&engine, p0, archangel()).expect("the Archangel resolved");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is on the table");
    assert!(
        types(&engine, angel).contains(TypeSet::CREATURE),
        "what arrived is the creature the card prints"
    );
    assert_eq!(pt(&engine, angel), (5, 5), "the body the card prints");
    let granted = keywords(&engine, angel);
    assert!(granted.contains(KeywordSet::FLYING), "\"Flying\"");
    assert!(granted.contains(KeywordSet::VIGILANCE), "\"vigilance\"");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING)
            && !keywords(&engine, elf).contains(KeywordSet::VIGILANCE),
        "the keywords belong to the Angel and not to the board"
    );

    // A creature that entered this turn is summoning sick (CR 302.6), so the
    // attack is made on its controller's next turn; the walk answers the empty
    // combat of the turn in between on its own.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, angel) && !is_tapped(&engine, elf),
        "the untap step stood both of them back up"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&angel) && attackers.contains(&elf),
        "both untapped creatures are offered, the Angel included: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(p1)), (elf, Defender::Player(p1))],
            },
        )
        .expect("both attackers came out of the list that offered them");

    // The end step is past combat damage (CR 510.2): the stack is already empty
    // the moment attackers are declared, so a predicate on the stack would stop
    // before a single point had been dealt.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        14,
        "a 5/5 and a 1/1 connected: six damage, so the Angel's power is real"
    );
    assert!(
        is_tapped(&engine, elf),
        "the Elf attacked and is tapped, which is what attacking does"
    );
    assert!(
        !is_tapped(&engine, angel),
        "\"vigilance\": the Angel attacked and is still standing (CR 702.20)"
    );
}
