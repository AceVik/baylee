//! `cards/instants/mv_5/second_thoughts.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "41bfef9f-6eb7-49c7-9b90-ff9f385ba670"

/// Second Thoughts — {4}{W} instant: "Exile target attacking creature. Draw
/// a card."
///
/// Both printed sentences are read off one combat phase, and each needs a
/// witness the other does not give it. The board carries an attacking Elf and
/// an Elf that stayed home, so the menu the target question publishes *is* the
/// first word of the card — a filter that had lost `ATTACKING` would offer the
/// bystander just as readily. The five Plains are tapped before anything is
/// claimed, because `can_afford` reads the pool and not the untapped lands,
/// and the creature is then read in the **exile** zone rather than in a
/// graveyard, which is the one word that separates this card from every
/// destroy spell. The draw is read as a move: the card that was on top of the
/// library is the card in hand afterwards, and the spell leaving the hand and
/// the drawn card replacing it cancel out of the hand count exactly.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn second_thoughts_exiles_an_attacking_creature_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Five Plains are exactly {4}{W}. One Elf is this seat's attacker, and the
    // one across the table is the creature that must stay off the menu.
    let mut engine = Duel::new(31, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[second_thoughts()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let bystander = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, attacker), (1, 1), "a body that may attack");
    assert_eq!(pt(&engine, bystander), (1, 1), "and one that stays home");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the active seat declares its own attackers");
    assert!(
        attackers.contains(&attacker),
        "an untapped 1/1 of this seat is offered as an attacker: {attackers:?}"
    );
    assert_eq!(
        attackers.len(),
        1,
        "and it is the only creature this seat has: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    // The combat phase keeps a priority round open with the Elf already
    // attacking and the damage step still ahead — the only place the card's
    // own filter can find a target. Answering the round by hand is also what
    // reads the price honestly: the mana is tapped for the question that
    // actually needs it and not a step earlier.
    let mut window = false;
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if player == p0 => {
                window = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing priority is always legal");
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .expect("declaring no blockers is always legal");
            }
            other => panic!("unexpected in the combat phase: {other:?}"),
        }
    }
    assert!(
        window,
        "the seat holding the instant is offered priority while its Elf is \
         still attacking"
    );

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Mana before the claim (the offer is read off the pool, not off the
    // board), and the attacking Elf named as the source kept back: it prints
    // its own `{T}: Add {G}`, so tapping it would put six mana in the pool for
    // a five-mana spell (#159).
    tap_mana_except(&mut engine, p0, attacker);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Plains, five white, and nothing off the Elf that is attacking"
    );
    cast_with_floating(&mut engine, p0, second_thoughts());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target attacking creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert_eq!(
        options,
        vec![attacker],
        "the attacking creature is the whole menu — the Elf on the other side \
         of the table is a creature and is not attacking, which is the word \
         this filter is read on"
    );
    // CR 601.2c names the target before CR 601.2h pays the cost, so the mana
    // is still floating and the creature still on the battlefield here.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the cost is the last step of the cast, so nothing is spent yet"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the exile has not happened either: it is the spell's resolution"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .expect("the attacking creature was the one option the question offered");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{W}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "the spell is on the stack, waiting to resolve"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&attacker),
        "\"Exile target attacking creature\": the Elf is in exile, under the \
         player who owned it"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_none(),
        "and not in a graveyard — a destroy spell would have put it there, and \
         nothing about the board above would look different"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the attacker left the battlefield"
    );
    assert_eq!(
        on_battlefield(&engine, p1, llanowar_elves()),
        Some(bystander),
        "and the creature the spell did not name never moved"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "still the printed 1/1 of a body that stayed home"
    );

    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the very card that was on top of the library, not merely \
         some card that appeared"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the spell left the hand and the draw replaced it, so the count is \
         where it was"
    );
}
