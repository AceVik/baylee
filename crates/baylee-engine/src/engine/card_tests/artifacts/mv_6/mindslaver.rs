//! Mindslaver controls decisions and sight only during the target's next actual turn.

use super::*;

fn mindslaver() -> CardIndex {
    card_index("a806f4ee-f48a-46c3-8772-5aaabebe4c7e")
}

#[test]
fn mindslaver_waits_for_the_targets_next_turn_even_when_cast_on_their_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[mindslaver(), island(), island(), island(), island()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .hand(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, mindslaver(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p0, mindslaver()).is_some());
    assert!(!e.may_inspect_private(p0, p1), "not the current turn");

    reach_their_main_phase(&mut e, p0);
    assert!(
        !e.may_inspect_private(p0, p1),
        "not the controller's intervening turn"
    );
    reach_their_main_phase(&mut e, p1);
    assert!(
        e.may_inspect_private(p0, p1),
        "the target's next turn is controlled"
    );

    let spell = in_hand(&e, p1, gray_ogre()).unwrap();
    for source in all_on_battlefield(&e, p1, mountain()) {
        e.apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    e.apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the controller makes the target's casting decision");
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, p1, gray_ogre()).is_some(),
        "resources still belong to the target"
    );

    reach_their_main_phase(&mut e, p0);
    assert!(
        !e.may_inspect_private(p0, p1),
        "control ends with that turn"
    );
}

// --- A controlled turn, decided against its player (CR 723) ---------------

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// P0's side: Mindslaver, the four Islands that pay for it, and `rest`,
/// which stays untapped.
fn slaver(rest: &[CardIndex]) -> Vec<CardIndex> {
    let mut cards = vec![mindslaver(), island(), island(), island(), island()];
    cards.extend_from_slice(rest);
    cards
}

/// P0 Mindslavers P1 from its own main phase, paying with its Islands
/// alone, and the game runs on to P1's next main phase, which P0 controls
/// (CR 723.1).
fn controlled_main(duel: Duel) -> Engine<RegistryLookup> {
    let mut e = duel.start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let islands = all_on_battlefield(&e, P0, island());
    tap_mana_where(&mut e, P0, |id| islands.contains(&id));
    activate(&mut e, P0, mindslaver(), 0);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    reach_their_main_phase(&mut e, P1);
    assert_eq!(e.pending().asked(), Some(P1), "P1's own priority");
    assert_eq!(e.decision_actor(), Some(P0), "made by P0 (CR 723.5)");
    e
}

/// Every copy of `card` in `seat`'s hand.
fn all_in_hand(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Hand(seat))
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// The controller casts the controlled player's Lightning Bolts with that
/// player's Mountains, one at that player's own creature and one at that
/// player (CR 723.5), and the controlled player cannot answer for itself.
#[test]
fn the_controller_aims_the_subjects_bolts_at_its_own_creature_and_at_it() {
    let mut e = controlled_main(
        Duel::new(SEED, forest())
            .battlefield(0, &slaver(&[]))
            .battlefield(1, &[mountain(), mountain(), gray_ogre()])
            .hand(1, &[lightning_bolt(), lightning_bolt()]),
    );
    assert!(
        e.apply(P1, PlayerAction::PassPriority).is_err(),
        "the controlled player does not decide its own turn"
    );
    let ogre = on_battlefield(&e, P1, gray_ogre()).unwrap();
    let mountains = all_on_battlefield(&e, P1, mountain());
    let bolts = all_in_hand(&e, P1, lightning_bolt());
    assert_eq!(bolts.len(), 2);

    for (land, bolt, at) in [
        (mountains[0], bolts[0], (vec![ogre], vec![])),
        (mountains[1], bolts[1], (vec![], vec![P1])),
    ] {
        e.apply(P0, PlayerAction::ActivateManaAbility { source: land })
            .expect("P1's land, tapped by P0");
        e.apply(P0, PlayerAction::CastSpell { card: bolt })
            .expect("P1's spell, cast by P0");
        let Pending::ChooseTargets { player, .. } = e.pending().clone() else {
            panic!("expected the Bolt's target, got {:?}", e.pending())
        };
        assert_eq!(player, P1, "the target is P1's spell's");
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: at.0,
                players: at.1,
            },
        )
        .expect("P1's own creature and P1 are legal targets");
        pass_until(&mut e, stack_is_empty);
    }
    assert!(in_graveyard(&e, P1, gray_ogre()).is_some(), "its own Ogre");
    assert_eq!(e.state().players[1].life, 17, "its own face");
    assert_eq!(e.state().players[0].life, 20);
}

/// Only the controlled player's resources pay for it (CR 723.5a): P0's
/// untapped Mountain is not on offer, cannot be tapped for P1, and P1's
/// Bolt, with nothing of P1's to pay for it, is not castable.
#[test]
fn only_the_subjects_own_mana_pays_for_the_subject() {
    let mut e = controlled_main(
        Duel::new(SEED, forest())
            .battlefield(0, &slaver(&[mountain()]))
            .battlefield(1, &[gray_ogre()])
            .hand(1, &[lightning_bolt()]),
    );
    let theirs = on_battlefield(&e, P0, mountain()).unwrap();
    let bolt = in_hand(&e, P1, lightning_bolt()).unwrap();
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    assert!(!legal.mana_abilities.contains(&theirs), "{legal:?}");
    assert!(!legal.castable.contains(&bolt) && !legal.payable.contains(&bolt));
    assert!(
        e.apply(P0, PlayerAction::ActivateManaAbility { source: theirs })
            .is_err(),
        "the controller's own land does not pay for P1"
    );
    assert!(e.apply(P0, PlayerAction::CastSpell { card: bolt }).is_err());
    assert!(!is_tapped(&e, theirs));
    assert!(in_hand(&e, P1, lightning_bolt()).is_some());
}

/// The controller attacks with the controlled player's creature into its
/// own blocker (CR 723.5), and blocks with its own creature as itself
/// (CR 723.8): the Ogre dies, and P0 takes nothing.
#[test]
fn the_controller_attacks_with_the_subjects_creature_into_its_own_blocker() {
    let mut e = controlled_main(
        Duel::new(SEED, forest())
            .battlefield(0, &slaver(&[serra_angel()]))
            .battlefield(1, &[gray_ogre()]),
    );
    let ogre = on_battlefield(&e, P1, gray_ogre()).unwrap();
    let angel = on_battlefield(&e, P0, serra_angel()).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = e.pending().clone()
    else {
        unreachable!("pass_until stopped on it")
    };
    assert_eq!((player, e.decision_actor()), (P1, Some(P0)));
    assert!(attackers.contains(&ogre));
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(ogre, Defender::Player(P0))],
        },
    )
    .expect("P1's attack, declared by P0");
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { player, .. } = e.pending().clone() else {
        unreachable!("pass_until stopped on it")
    };
    assert_eq!(
        (player, e.decision_actor()),
        (P0, Some(P0)),
        "P0 blocks as itself"
    );
    e.apply(
        P0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(angel, ogre)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert!(in_graveyard(&e, P1, gray_ogre()).is_some(), "the Ogre died");
    assert!(on_battlefield(&e, P0, serra_angel()).is_some());
    assert_eq!(e.state().players[0].life, 20, "P0 took nothing");
}

/// A choice the controlled player's spell asks as it resolves is the
/// controller's (CR 723.5): P0 puts P1's two best cards back on top of
/// P1's library with P1's Brainstorm.
#[test]
fn the_controller_makes_the_subjects_resolution_choice() {
    let mut e = controlled_main(
        Duel::new(SEED, forest())
            .battlefield(0, &slaver(&[]))
            .battlefield(1, &[island()])
            .hand(1, &[brainstorm(), lightning_bolt(), gray_ogre()]),
    );
    let land = on_battlefield(&e, P1, island()).unwrap();
    let spell = in_hand(&e, P1, brainstorm()).unwrap();
    let (bolt, ogre) = (
        in_hand(&e, P1, lightning_bolt()).unwrap(),
        in_hand(&e, P1, gray_ogre()).unwrap(),
    );
    e.apply(P0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!("pass_until stopped on it")
    };
    assert_eq!((player, e.decision_actor()), (P1, Some(P0)));
    assert!(options.contains(&bolt) && options.contains(&ogre));
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![bolt, ogre],
        },
    )
    .expect("P0 chooses what P1 puts back");
    pass_until(&mut e, stack_is_empty);
    let library = e.state().zones.list(ZoneLocation::Library(P1));
    let top: Vec<ObjectId> = library.iter().rev().take(2).copied().collect();
    let cards = |id: &ObjectId| e.state().object(*id).and_then(|o| o.card).map(|c| c.index);
    let mut on_top: Vec<_> = top.iter().filter_map(cards).collect();
    on_top.sort_unstable();
    let mut put_back = vec![lightning_bolt(), gray_ogre()];
    put_back.sort_unstable();
    assert_eq!(on_top, put_back, "the two cards P0 chose");
    assert!(in_hand(&e, P1, lightning_bolt()).is_none());
}

/// At the controlled player's cleanup the controller chooses what that
/// player discards to its maximum hand size (CR 723.5, CR 514.1).
#[test]
fn the_controller_chooses_the_subjects_discard_to_hand_size() {
    let mut hand = vec![lightning_bolt(), lightning_bolt()];
    hand.extend([forest(); 6]);
    let mut e = controlled_main(
        Duel::new(SEED, forest())
            .battlefield(0, &slaver(&[]))
            .hand(1, &hand),
    );
    let bolts = all_in_hand(&e, P1, lightning_bolt());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::DiscardChoice { .. })
    });
    let Pending::DiscardChoice { player, count } = e.pending().clone() else {
        unreachable!("pass_until stopped on it")
    };
    assert_eq!((player, e.decision_actor()), (P1, Some(P0)));
    assert_eq!(count, 2, "eight cards and the draw, against seven");
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: bolts.clone(),
        },
    )
    .expect("P0 chooses P1's discards");
    for bolt in bolts {
        assert!(
            e.state()
                .zones
                .list(ZoneLocation::Graveyard(P1))
                .contains(&bolt),
            "P1's Bolts are the ones discarded"
        );
    }
}

/// The controller goes on making its own decisions in that turn
/// (CR 723.8): passing for P1 with P1's spell on the stack hands P0 its
/// own priority, and P0 answers with its own Bolt from its own Mountain.
#[test]
fn the_controller_still_makes_its_own_decisions() {
    let mut e = controlled_main(
        Duel::new(SEED, forest())
            .battlefield(0, &slaver(&[mountain()]))
            .battlefield(1, &[mountain(), gray_ogre()])
            .hand(0, &[lightning_bolt()])
            .hand(1, &[lightning_bolt()]),
    );
    let ogre = on_battlefield(&e, P1, gray_ogre()).unwrap();
    let theirs = in_hand(&e, P1, lightning_bolt()).unwrap();
    let land = on_battlefield(&e, P1, mountain()).unwrap();
    e.apply(P0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: theirs })
        .unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![P0],
        },
    )
    .expect("P1's Bolt at P0: P0 may decide it badly for itself too");
    e.apply(P0, PlayerAction::PassPriority)
        .expect("P0 passes for P1");
    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    assert_eq!((player, e.decision_actor()), (P0, Some(P0)), "P0's own");
    let mine = in_hand(&e, P0, lightning_bolt()).unwrap();
    let my_land = on_battlefield(&e, P0, mountain()).unwrap();
    e.apply(P0, PlayerAction::ActivateManaAbility { source: my_land })
        .expect("P0's own land, for P0's own spell");
    e.apply(P0, PlayerAction::CastSpell { card: mine }).unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![ogre],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P1, gray_ogre()).is_some());
    assert_eq!(e.state().players[0].life, 17);
}

/// The controller cannot make the controlled player concede, and the
/// controlled player may concede at any time (CR 723.6, CR 104.3a).
#[test]
fn only_the_subject_concedes_for_the_subject() {
    let duel = || {
        Duel::new(SEED, forest())
            .battlefield(0, &slaver(&[]))
            .battlefield(1, &[gray_ogre()])
    };
    let mut e = controlled_main(duel());
    e.apply(P0, PlayerAction::Concede).unwrap();
    assert!(e.state().has_left(P0), "P0's concession is P0's own");
    assert!(!e.state().has_left(P1));

    let mut e = controlled_main(duel());
    e.apply(P1, PlayerAction::Concede)
        .expect("a controlled player may still concede");
    assert!(e.state().has_left(P1));
    assert!(!e.state().has_left(P0));
}

/// Two effects controlling the same player's next turn overwrite each
/// other, and the last one created is the one that works (CR 723.1a): P0
/// Mindslavers P1, then P2 does in the same turn, and P2 decides P1's turn.
#[test]
fn the_last_player_control_effect_is_the_one_that_works() {
    let p2 = PlayerId::new(2);
    let mut e = Duel::table(SEED, forest(), 3)
        .battlefield(0, &slaver(&[]))
        .battlefield(1, &[gray_ogre()])
        .battlefield(2, &slaver(&[]))
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for (seat, first) in [(P0, true), (p2, false)] {
        if !first {
            pass_until(
                &mut e,
                |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p2),
            );
        }
        let islands = all_on_battlefield(&e, seat, island());
        tap_mana_where(&mut e, seat, |id| islands.contains(&id));
        activate(&mut e, seat, mindslaver(), 0);
        e.apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
    }
    assert_eq!(e.state().turn.active, P0, "both in P0's turn");
    reach_their_main_phase(&mut e, P1);
    assert_eq!(e.pending().asked(), Some(P1));
    assert_eq!(e.decision_actor(), Some(p2), "the later effect works");
    assert!(e.may_inspect_private(p2, P1));
    assert!(
        !e.may_inspect_private(P0, P1),
        "the earlier one is overwritten"
    );
}
