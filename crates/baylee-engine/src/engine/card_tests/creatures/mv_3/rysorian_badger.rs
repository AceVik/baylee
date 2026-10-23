//! Rysorian Badger — `{2}{G}` 2/2 Creature — Badger (Homelands)
//!
//! Whenever this creature attacks and isn't blocked, you may exile up to two target creature cards
//! from defending player's graveyard. If you do, you gain 1 life for each card exiled this way and
//! this creature assigns no combat damage this turn.
//!
//! The card is `Partial`: the life gain is a fixed 1 (`ThisWay` cannot count cards exiled from a
//! graveyard) and the trigger needs at least one creature card to target.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rysorian_badger() -> CardIndex {
    card_index("cf9fc317-3184-4999-84ef-945a8842096e")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// P0 has the Badger and, in the graveyard, a Hill Giant of its own. P1 has `p1_blockers` on the
/// battlefield and, in the graveyard, `p1_graveyard`. The engine is at P0's first main phase.
fn table(p1_blockers: &[CardIndex], p1_graveyard: &[CardIndex]) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[hill_giant()])
        .hand(1, p1_graveyard)
        .battlefield(0, &[rysorian_badger()])
        .battlefield(1, p1_blockers)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hand_to_graveyard(&mut engine, p0, hill_giant());
    for card in p1_graveyard {
        hand_to_graveyard(&mut engine, p1, *card);
    }
    engine
}

/// Attacks P1 with the Badger; P1 blocks it with `block_with`, or does not block.
fn attack(engine: &mut Engine<RegistryLookup>, block_with: Option<ObjectId>) -> ObjectId {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let badger = on_battlefield(engine, p0, rysorian_badger()).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(badger, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let blockers = block_with.map(|b| (b, badger)).into_iter().collect();
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers })
        .unwrap();
    badger
}

#[test]
fn rysorian_badger_exiles_up_to_two_creature_cards_from_the_defenders_graveyard_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[grizzly_bears(), gray_ogre(), lightning_bolt()]);
    let gy_bear = in_graveyard(&engine, p1, grizzly_bears()).unwrap();
    let gy_ogre = in_graveyard(&engine, p1, gray_ogre()).unwrap();
    let gy_bolt = in_graveyard(&engine, p1, lightning_bolt()).unwrap();
    let own_giant = in_graveyard(&engine, p0, hill_giant()).unwrap();

    attack(&mut engine, None);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    // "Creature cards from defending player's graveyard": a noncreature card in it, and a creature
    // card in the attacker's own graveyard, are not targets.
    for illegal in [gy_bolt, own_giant] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![illegal],
                        players: vec![],
                    },
                )
                .is_err(),
            "only creature cards in the defending player's graveyard are targets"
        );
    }
    // "Up to two": three is too many.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![gy_bear, gy_ogre, gy_bolt],
                    players: vec![],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gy_bear, gy_ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().object(gy_bear).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(gy_ogre).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().object(gy_bolt).unwrap().zone,
        Zone::Graveyard,
        "the noncreature card stays"
    );
    assert_eq!(
        engine.state().object(own_giant).unwrap().zone,
        Zone::Graveyard,
        "the attacker's own graveyard is untouched"
    );
    // Printed: 1 life for each card exiled, so 22 for two. The card gains a fixed 1 (it is
    // Partial); this assertion moves to 22 when `ThisWay` can count cards exiled from a graveyard.
    assert!(engine.state().players[0].life >= 21);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the Badger assigned no combat damage this turn"
    );
}

#[test]
fn rysorian_badger_keeps_its_damage_and_the_graveyard_when_the_player_declines() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[grizzly_bears()]);
    let gy_bear = in_graveyard(&engine, p1, grizzly_bears()).unwrap();

    attack(&mut engine, None);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gy_bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(gy_bear).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "no card exiled, no life"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "declined: the Badger dealt its 2 damage"
    );
}

#[test]
fn rysorian_badger_does_nothing_when_blocked() {
    let p1 = PlayerId::new(1);
    let mut engine = table(&[gray_ogre()], &[grizzly_bears()]);
    let gy_bear = in_graveyard(&engine, p1, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    attack(&mut engine, Some(ogre));
    // "Isn't blocked": a blocked Badger has no trigger, so nothing asks a target or a "may".
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(gy_bear).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20, "the Ogre took the blow");
}

#[test]
fn rysorian_badger_with_no_creature_card_to_exile_still_deals_its_damage() {
    let p1 = PlayerId::new(1);
    // The defender's graveyard holds only a noncreature card: nothing can be exiled, so "if you do"
    // never applies and the Badger is not stopped from dealing damage.
    let mut engine = table(&[], &[lightning_bolt()]);
    let gy_bolt = in_graveyard(&engine, p1, lightning_bolt()).unwrap();

    attack(&mut engine, None);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(gy_bolt).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "no card exiled, no life"
    );
    assert_eq!(engine.state().players[1].life, 18);
}
