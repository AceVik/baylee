//! `cards/enchantments/auras/mv_3/bestial_fury.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bestial_fury() -> CardIndex {
    card_index("dfc08963-132f-4ebf-9384-15d17d1098f7")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Player 0 enchants its Hill Giant with Bestial Fury in the first main phase and attacks with the
/// Giant and its Gray Ogre. Player 1 owns a Grizzly Bears and a Gray Ogre. Stops at player 1's
/// blocker question.
struct Attack {
    engine: Engine<RegistryLookup>,
    giant: ObjectId,
    ogre: ObjectId,
    bears: ObjectId,
    their_ogre: ObjectId,
}

fn attack_with_fury() -> Attack {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                hill_giant(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), gray_ogre()])
        .hand(0, &[bestial_fury()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, bestial_fury());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let fury = on_battlefield(&engine, p0, bestial_fury()).expect("Bestial Fury enters");
    assert_eq!(
        engine.state().object(fury).unwrap().attached_to,
        Some(giant)
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1)), (ogre, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let their_ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    Attack {
        engine,
        giant,
        ogre,
        bears,
        their_ogre,
    }
}

/// `Bestial Fury` ({2}{R}, Enchantment — Aura, `Coverage::Partial`): "Whenever enchanted creature
/// becomes blocked, it gets +4/+0 and gains trample until end of turn." It triggers once however
/// many creatures block, and it ends with the turn.
#[test]
fn bestial_fury_pumps_the_blocked_enchanted_creature_once_until_end_of_turn() {
    let p1 = PlayerId::new(1);
    let Attack {
        mut engine,
        giant,
        ogre,
        bears,
        their_ogre,
    } = attack_with_fury();
    assert_eq!(pt(&engine, giant), (3, 3), "nothing before the block");

    // Two creatures block the Giant: one trigger, not one per blocker.
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, giant), (their_ogre, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (7, 3), "+4/+0 once");
    assert!(keywords(&engine, giant).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        pt(&engine, ogre),
        (2, 2),
        "the unenchanted attacker gets nothing"
    );
    assert!(!keywords(&engine, ogre).contains(KeywordSet::TRAMPLE));
}

/// "Enchanted creature" is the Giant alone: a block on the other attacker does nothing for it, and
/// an unblocked Giant is not "blocked".
#[test]
fn bestial_fury_ignores_a_block_on_another_creature_and_an_unblocked_giant() {
    let p1 = PlayerId::new(1);
    let Attack {
        mut engine,
        giant,
        ogre,
        bears,
        ..
    } = attack_with_fury();

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, ogre)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (3, 3), "the Giant was not blocked");
    assert!(!keywords(&engine, giant).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        pt(&engine, ogre),
        (2, 2),
        "the blocked Ogre is not enchanted"
    );
    assert!(!keywords(&engine, ogre).contains(KeywordSet::TRAMPLE));
}

/// "Until end of turn": a Giant that survived a single block is a 3/3 without trample again in
/// the next turn.
#[test]
fn bestial_fury_pump_ends_with_the_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let Attack {
        mut engine,
        giant,
        bears,
        ..
    } = attack_with_fury();
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (7, 3));

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    assert!(
        on_battlefield(&engine, p0, hill_giant()).is_some(),
        "two damage does not kill a 3/3"
    );
    assert_eq!(pt(&engine, giant), (3, 3));
    assert!(!keywords(&engine, giant).contains(KeywordSet::TRAMPLE));
}

/// "When this Aura enters, draw a card at the beginning of the next turn's upkeep": its
/// controller draws one card, at the next upkeep only, and the opponent draws nothing from it.
#[test]
fn bestial_fury_draws_a_card_at_the_next_upkeep_only() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let Attack { mut engine, .. } = attack_with_fury();
    let hand = |e: &Engine<RegistryLookup>, seat: PlayerId| {
        e.state().zones.list(ZoneLocation::Hand(seat)).len()
    };

    let mine_before = hand(&engine, p0);
    let theirs_before = hand(&engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    assert_eq!(
        hand(&engine, p0),
        mine_before,
        "not drawn before the upkeep trigger resolves"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand(&engine, p0), mine_before + 1, "the controller draws");
    assert_eq!(
        hand(&engine, p1),
        theirs_before,
        "the opponent draws nothing from it yet"
    );

    // Player 1's own draw step is the only other draw this turn; the following upkeep, player 0's,
    // draws nothing more from the Aura.
    let mine_after = hand(&engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand(&engine, p0),
        mine_after,
        "it is a one-shot, not every upkeep"
    );
}
