//! `cards/creatures/mv_2/brad_boimler_eager_ensign.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Brad Boimler, Eager Ensign ({1}{W}) is a 2/2 legendary Human Officer with
/// lifelink, and the card is `Coverage::Partial` because the replacement
/// effect that would add an extra counter whenever he becomes tapped has no
/// DSL form — so the half there is to play is the keyword. Lifelink exists
/// only while damage is being dealt, so he attacks and the other seat blocks:
/// his two damage land on an Elf, which is worth exactly two life to his
/// controller, while the defending player's life total does not move. A
/// lifegain trigger, or damage that simply connected with a player, would
/// each satisfy only one of those two readings.
#[test]
fn brad_boimler_deals_his_damage_to_a_blocker_and_his_controller_gains_that_much_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[brad_boimler_eager_ensign()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let brad = on_battlefield(&engine, p0, brad_boimler_eager_ensign()).expect("he is out");
    let blocker = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, brad), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, brad).contains(KeywordSet::LIFELINK),
        "the one line of this card that is implemented"
    );
    let (mine, theirs) = (
        engine.state().players[0].life,
        engine.state().players[1].life,
    );

    // Declare him as the only attacker, against the one defender a duel has.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers {
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the declaration it was asked for")
    };
    assert!(
        attackers.contains(&brad),
        "nothing keeps a settled 2/2 out of combat: {attackers:?}"
    );
    assert_eq!(
        defenders.len(),
        1,
        "one surviving opponent, and no planeswalkers"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(brad, defenders[0])],
            },
        )
        .unwrap();

    // The other seat blocks, so the damage lands on a creature rather than on
    // a player: lifelink is about damage, not about connecting.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the declaration it was asked for")
    };
    assert!(
        blockers
            .iter()
            .any(|option| option.blocker == blocker && option.attackers.contains(&brad)),
        "a 1/1 may block a 2/2 with no evasion: {blockers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, brad)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().players[0].life > mine);
    assert_eq!(
        engine.state().players[0].life,
        mine + 2,
        "lifelink: the two damage he dealt to the Elf are two life"
    );
    assert_eq!(
        engine.state().players[1].life,
        theirs,
        "and none of it reached the player, because the Elf blocked"
    );
}

use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aim(e: &mut Engine<RegistryLookup>, seat: PlayerId, objects: &[ObjectId], players: &[PlayerId]) {
    let Pending::ChooseTargets { player, .. } = e.pending() else {
        panic!("target question: {:?}", e.pending())
    };
    assert_eq!(*player, seat);
    e.apply(
        seat,
        PlayerAction::ChooseTargets {
            objects: objects.to_vec(),
            players: players.to_vec(),
        },
    )
    .unwrap();
}

fn cast_aim(
    e: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    cast_from_hand(e, seat, card);
    aim(e, seat, objects, players);
}

#[test]
fn batch57_brad_adds_to_own_counter_placements_from_either_player_persists_after_leaving_and_expires()
 {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::BRAD_BOIMLER_EAGER_ENSIGN,
                index::LLANOWAR_ELVES,
                index::FOREST,
                index::FOREST,
                index::FOREST,
                index::ISLAND,
                index::ISLAND,
            ],
        )
        .hand(
            0,
            &[
                index::TWIDDLE,
                index::UNSUMMON,
                index::BATTLEGROWTH,
                index::BATTLEGROWTH,
                index::BATTLEGROWTH,
                index::BATTLEGROWTH,
            ],
        )
        .battlefield(1, &[index::FOREST, index::GRIZZLY_BEARS])
        .hand(1, &[index::BATTLEGROWTH])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let brad = on_battlefield(&e, P0, index::BRAD_BOIMLER_EAGER_ENSIGN).unwrap();
    let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    cast_aim(&mut e, P0, index::TWIDDLE, &[brad], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, brad));
    cast_aim(&mut e, P0, index::BATTLEGROWTH, &[elf], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, elf, CounterKind::P1P1), 2);
    cast_aim(&mut e, P1, index::BATTLEGROWTH, &[elf], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        counters_on(&e, elf, CounterKind::P1P1),
        4,
        "recipient controller matters, not placer"
    );
    cast_aim(&mut e, P0, index::BATTLEGROWTH, &[bear], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, bear, CounterKind::P1P1), 1);
    cast_aim(&mut e, P0, index::UNSUMMON, &[brad], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, P0, index::BRAD_BOIMLER_EAGER_ENSIGN).is_some());
    cast_aim(&mut e, P0, index::BATTLEGROWTH, &[elf], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        counters_on(&e, elf, CounterKind::P1P1),
        6,
        "resolved replacement outlives Brad"
    );
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    cast_aim(&mut e, P0, index::BATTLEGROWTH, &[elf], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        counters_on(&e, elf, CounterKind::P1P1),
        7,
        "effect expired at end of turn"
    );
}

#[test]
fn batch57_brad_each_new_tap_grants_one_more_additive_replacement() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::BRAD_BOIMLER_EAGER_ENSIGN,
                index::LLANOWAR_ELVES,
                index::FOREST,
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
            ],
        )
        .hand(
            0,
            &[
                index::TWIDDLE,
                index::TWIDDLE,
                index::TWIDDLE,
                index::BATTLEGROWTH,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let brad = on_battlefield(&e, P0, index::BRAD_BOIMLER_EAGER_ENSIGN).unwrap();
    let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    for tapped in [true, false, true] {
        cast_aim(&mut e, P0, index::TWIDDLE, &[brad], &[]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(is_tapped(&e, brad), tapped);
    }
    cast_aim(&mut e, P0, index::BATTLEGROWTH, &[elf], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        counters_on(&e, elf, CounterKind::P1P1),
        3,
        "two taps, not three state toggles"
    );
    assert_eq!(pt(&e, elf), (4, 4));
}

#[test]
fn batch57_brad_increments_negative_counters_as_well_as_positive_counters() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::BRAD_BOIMLER_EAGER_ENSIGN,
                index::YAWGMOTH_THRAN_PHYSICIAN,
                index::ROOTBREAKER_WURM,
                index::LLANOWAR_ELVES,
                index::FOREST,
                index::ISLAND,
            ],
        )
        .hand(0, &[index::TWIDDLE, index::BATTLEGROWTH])
        .battlefield(1, &[index::ROOTBREAKER_WURM])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let brad = on_battlefield(&e, P0, index::BRAD_BOIMLER_EAGER_ENSIGN).unwrap();
    let root = on_battlefield(&e, P0, index::ROOTBREAKER_WURM).unwrap();
    let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    let yawg = on_battlefield(&e, P0, index::YAWGMOTH_THRAN_PHYSICIAN).unwrap();
    let other = on_battlefield(&e, P1, index::ROOTBREAKER_WURM).unwrap();
    assert_eq!(pt(&e, root), (6, 6));
    cast_aim(&mut e, P0, index::TWIDDLE, &[brad], &[]);
    pass_until(&mut e, stack_is_empty);
    cast_aim(&mut e, P0, index::BATTLEGROWTH, &[brad], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, brad, CounterKind::P1P1), 2);
    kit::activate_object(&mut e, P0, yawg, 1);
    aim(&mut e, P0, &[root], &[]);
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending()
    else {
        panic!("sacrifice cost")
    };
    assert_eq!(*player, P0);
    assert!(options.contains(&elf));
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, index::LLANOWAR_ELVES).is_some());
    assert_eq!(counters_on(&e, root, CounterKind::M1M1), 2);
    assert_eq!(pt(&e, root), (4, 4));
    assert_eq!(counters_on(&e, other, CounterKind::M1M1), 0);
    assert_eq!(e.state().players[0].life, 19);
}

#[test]
fn batch57_brad_empower_resumes_after_token_entry_before_zero_loyalty_state_based_actions() {
    for (add_first, loyalty) in [(true, 8), (false, 7)] {
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(
                0,
                &[
                    index::BRAD_BOIMLER_EAGER_ENSIGN,
                    index::DOUBLING_SEASON,
                    index::ISLAND,
                    index::ISLAND,
                    index::ISLAND,
                ],
            )
            .hand(0, &[index::TWIDDLE, index::THEORIST_S_PROXY])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        let brad = on_battlefield(&e, P0, index::BRAD_BOIMLER_EAGER_ENSIGN).unwrap();
        cast_aim(&mut e, P0, index::TWIDDLE, &[brad], &[]);
        pass_until(&mut e, stack_is_empty);
        cast_with_floating(&mut e, P0, index::THEORIST_S_PROXY);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            player,
            options,
            min,
            max,
            ..
        } = e.pending().clone()
        else {
            unreachable!()
        };
        assert_eq!(player, P0);
        assert_eq!((min, max), (1, 1));
        assert_eq!(options.len(), 2);
        for token in &options {
            assert!(types(&e, *token).contains(TypeSet::PLANESWALKER));
            assert_eq!(
                counters_on(&e, *token, CounterKind::Loyalty),
                0,
                "zero entry counters do not gain an extra counter"
            );
        }
        let chosen = options[1];
        let unchosen = options[0];
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
        let Pending::YesNo {
            player,
            prompt:
                crate::choice::YesNoPrompt::ApplyAdditionalCountersNext {
                    permanent,
                    additional,
                },
            ..
        } = e.pending()
        else {
            panic!("Brad and Doubling Season order: {:?}", e.pending());
        };
        assert_eq!((*player, *permanent, *additional), (P0, chosen, 1));
        assert_eq!(
            counters_on(&e, chosen, CounterKind::Loyalty),
            0,
            "counter placement has not committed before ordering"
        );
        e.apply(P0, PlayerAction::YesNo(add_first)).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            tokens_of(&e, P0),
            vec![chosen],
            "unchosen zero-loyalty Jace dies only after empower finishes"
        );
        assert!(e.state().object(unchosen).is_none());
        assert_eq!(counters_on(&e, chosen, CounterKind::Loyalty), loyalty);
        let hand = e.state().zones.list(ZoneLocation::Hand(P0)).len();
        kit::activate_object(&mut e, P0, chosen, 1);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(counters_on(&e, chosen, CounterKind::Loyalty), loyalty - 3);
        assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), hand + 1);
    }
}
