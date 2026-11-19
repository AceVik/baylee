//! `cards/instants/mv_3/archmage_s_charm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;
use baylee_engine::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn archmages_charm() -> CardIndex {
    card_index("85cbb035-25df-43bb-ae06-ab098f448b02")
}

#[track_caller]
fn choose_charm_mode(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    mode: usize,
) -> Vec<CastModeKind> {
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCastMode, got {:?}", engine.pending());
    };
    assert_eq!(player, seat);
    let offered: Vec<CastModeKind> = options.iter().map(|o| o.kind).collect();
    let slot = offered
        .iter()
        .position(|k| *k == CastModeKind::Mode(mode))
        .unwrap_or_else(|| panic!("mode {mode} not offered: {offered:?}"));
    engine
        .apply(seat, PlayerAction::ChooseMode(slot))
        .expect("choose offered mode");
    offered
}

#[track_caller]
fn choose_target_object(engine: &mut Engine<RegistryLookup>, seat: PlayerId, target: ObjectId) {
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(player, seat);
    assert!(
        options.contains(&target),
        "expected {target:?} in target options {options:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("chosen target accepted");
}

#[track_caller]
fn choose_target_player(engine: &mut Engine<RegistryLookup>, seat: PlayerId, target: PlayerId) {
    let Pending::ChoosePlayer {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChoosePlayer, got {:?}", engine.pending());
    };
    assert_eq!(player, seat);
    assert!(
        options.contains(&target),
        "expected {target:?} in player options {options:?}"
    );
    engine
        .apply(seat, PlayerAction::ChoosePlayer(target))
        .expect("chosen player accepted");
}

fn did_not_resolve(engine: &Engine<RegistryLookup>, object: ObjectId, from_seq: usize) -> bool {
    engine.journal().entries()[from_seq..].iter().any(
        |e| matches!(e.event, GameEvent::StackObjectDidNotResolve { object: o } if o == object),
    )
}

/// {U}{U}{U} is required, CR 700.2a omits modes with no legal targets, and
/// choosing one mode executes only that mode.
#[test]
fn archmages_charm_requires_three_blue_and_offers_only_legal_modes() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[archmages_charm()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    let charm = in_hand(&engine, P0, archmages_charm()).expect("charm in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.castable.contains(&charm),
        "{{U}}{{U}}{{G}} cannot pay {{U}}{{U}}{{U}}"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: charm })
            .is_err()
    );

    // With empty stack and only lands + MV 2 Bears on board, only mode 1 is legal
    // (auto-selected without asking ChooseCastMode).
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[archmages_charm()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    cast_from_hand(&mut engine, P0, archmages_charm());
    assert!(
        matches!(engine.pending(), Pending::ChoosePlayer { options, .. } if *options == vec![P0, P1]),
        "only mode 1 is legal, so engine proceeds straight to ChoosePlayer: {:?}",
        engine.pending()
    );
}

/// With all three modes legal, `ChooseCastMode` offers all three, rejects an
/// invalid mode index, and executes only the single chosen mode.
#[test]
fn archmages_charm_offers_all_modes_and_executes_only_chosen_mode() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[archmages_charm()])
        .battlefield(1, &[forest(), forest(), llanowar_elves()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P1));

    let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    let elves = on_battlefield(&engine, P1, llanowar_elves()).expect("elves");
    cast_from_hand(&mut engine, P1, grizzly_bears());
    let bears = on_stack(&engine, grizzly_bears()).expect("bears on stack");
    pass_until_priority(&mut engine, P0);

    cast_from_hand(&mut engine, P0, archmages_charm());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCastMode");
    };
    assert_eq!(options.len(), 3);
    assert!(engine.apply(P0, PlayerAction::ChooseMode(3)).is_err());
    let offered = choose_charm_mode(&mut engine, P0, 0);
    assert_eq!(
        offered,
        vec![
            CastModeKind::Mode(0),
            CastModeKind::Mode(1),
            CastModeKind::Mode(2)
        ]
    );
    choose_target_object(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P1, grizzly_bears()).is_some(),
        "mode 0 countered Bears"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand_before - 1,
        "mode 1 did not draw"
    );
    assert_eq!(
        engine.state().object(elves).map(|o| o.controller),
        Some(P1),
        "mode 2 did not steal"
    );
}

/// Mode 0 counters target spell (CR 701.6a), leaves spent mana unrefunded
/// (CR 701.6b), ignores abilities on the stack (CR 113.9), and cannot target
/// itself (CR 115.5). Mode/target are chosen before mana payment (CR 601.2b/c/h).
#[test]
fn archmages_charm_mode_zero_counters_spell_and_ignores_abilities_and_self() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[forest(), forest(), index::PRODIGAL_SORCERER])
        .hand(0, &[grizzly_bears()])
        .battlefield(1, &[island(), island(), island()])
        .hand(1, &[archmages_charm()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, grizzly_bears());
    let bears_spell = on_stack(&engine, grizzly_bears()).expect("bears spell");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "P0 paid for Bears"
    );

    // Hold priority and put an activated ability on the stack above Bears.
    activate(&mut engine, P0, index::PRODIGAL_SORCERER, 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .expect("aim sorcerer at P1");
    let ability_id = top_of_stack(&engine);
    assert_ne!(ability_id, bears_spell);

    pass_until_priority(&mut engine, P1);
    let charm_id = in_hand(&engine, P1, archmages_charm()).expect("charm in hand");
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, archmages_charm());
    let offered = choose_charm_mode(&mut engine, P1, 0);
    assert_eq!(offered, vec![CastModeKind::Mode(0), CastModeKind::Mode(1)]);

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&bears_spell), "spell is a legal target");
    assert!(
        !options.contains(&ability_id),
        "CR 113.9: ability is not a spell"
    );
    assert!(
        !options.contains(&charm_id),
        "CR 115.5: cannot target itself"
    );
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        3,
        "CR 601.2c before CR 601.2h: mana stays floating while choosing targets"
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::ChooseObjects {
                    objects: vec![ability_id]
                }
            )
            .is_err()
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::ChooseObjects {
                    objects: vec![charm_id]
                }
            )
            .is_err()
    );

    choose_target_object(&mut engine, P1, bears_spell);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_none());
    assert!(
        in_graveyard(&engine, P0, grizzly_bears()).is_some(),
        "CR 701.6a"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "CR 701.6b: no refund"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "Sorcerer ability still resolved"
    );
    assert!(in_graveyard(&engine, P1, archmages_charm()).is_some());
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
}

/// Mode 0 legally targets an uncounterable spell, fails to counter it so it
/// resolves, and fizzles (CR 608.2b) if a legal target leaves the stack first.
#[test]
fn archmages_charm_mode_zero_uncounterable_and_fizzle_on_removed_target() {
    // Subcase A: uncounterable spell (Abrupt Decay) stays on stack and resolves.
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[swamp(), forest()])
        .hand(0, &[abrupt_decay()])
        .battlefield(1, &[island(), island(), island(), llanowar_elves()])
        .hand(1, &[archmages_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let elves = on_battlefield(&engine, P1, llanowar_elves()).expect("elves");
    cast_from_hand(&mut engine, P0, abrupt_decay());
    choose_target_object(&mut engine, P0, elves);
    let decay = on_stack(&engine, abrupt_decay()).expect("abrupt decay on stack");
    pass_until_priority(&mut engine, P1);

    cast_from_hand(&mut engine, P1, archmages_charm());
    choose_charm_mode(&mut engine, P1, 0);
    choose_target_object(&mut engine, P1, decay);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, archmages_charm()).is_some());
    assert!(
        in_graveyard(&engine, P0, abrupt_decay()).is_some(),
        "Abrupt Decay resolved"
    );
    assert!(
        in_graveyard(&engine, P1, llanowar_elves()).is_some(),
        "Elves destroyed"
    );

    // Subcase B: target countered before Charm resolves -> Charm fizzles (CR 608.2b).
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[grizzly_bears()])
        .battlefield(1, &[island(), island(), island(), island(), island()])
        .hand(1, &[archmages_charm(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, grizzly_bears());
    let bears = on_stack(&engine, grizzly_bears()).expect("bears");
    pass_until_priority(&mut engine, P1);

    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, archmages_charm());
    choose_charm_mode(&mut engine, P1, 0);
    choose_target_object(&mut engine, P1, bears);
    let charm = on_stack(&engine, archmages_charm()).expect("charm");
    let seq = engine.journal().len();

    // P1 retains priority (CR 601.2i) and counters Bears first with Counterspell.
    cast_with_floating(&mut engine, P1, counterspell());
    choose_target_object(&mut engine, P1, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, P1, archmages_charm()).is_some());
    assert!(
        did_not_resolve(&engine, charm, seq),
        "CR 608.2b: Charm fizzled"
    );
}

/// Mode 1 makes the targeted player — either you or the opponent — draw
/// exactly the top two cards of their library, leaving the other untouched.
#[test]
fn archmages_charm_mode_one_draws_two_for_target_player_you_or_opponent() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[archmages_charm(), archmages_charm()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p0_lib: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    let p1_lib: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(P1)).clone();
    let p0_top2 = &p0_lib[p0_lib.len() - 2..];
    let p1_top2 = &p1_lib[p1_lib.len() - 2..];
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();

    // Target self (P0).
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, archmages_charm());
    choose_charm_mode(&mut engine, P0, 1);
    choose_target_player(&mut engine, P0, P0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, P0), p0_lib.len() - 2);
    assert_eq!(library_size(&engine, P1), p1_lib.len());
    let p0_hand = engine.state().zones.list(ZoneLocation::Hand(P0));
    assert!(
        p0_top2.iter().all(|c| p0_hand.contains(c)),
        "P0 drew exact top 2"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand_before
    );

    // Target opponent (P1).
    let p0_hand_mid = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    cast_with_floating(&mut engine, P0, archmages_charm());
    choose_charm_mode(&mut engine, P0, 1);
    choose_target_player(&mut engine, P0, P1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, P0), p0_lib.len() - 2);
    assert_eq!(library_size(&engine, P1), p1_lib.len() - 2);
    let p1_hand = engine.state().zones.list(ZoneLocation::Hand(P1));
    assert!(
        p1_top2.iter().all(|c| p1_hand.contains(c)),
        "P1 drew exact top 2"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand_mid - 1,
        "P0 only spent the Charm"
    );
}

/// Mode 1 respects player hexproof (CR 702.11c) both at resolution (CR 608.2b)
/// and when choosing targets on cast.
#[test]
fn archmages_charm_mode_one_respects_player_hexproof() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[archmages_charm(), archmages_charm()])
        .battlefield(1, &[plains(), forest(), llanowar_elves()])
        .hand(1, &[everybody_lives()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p1_lib_before = library_size(&engine, P1);
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, archmages_charm());
    choose_charm_mode(&mut engine, P0, 1);
    choose_target_player(&mut engine, P0, P1);
    let charm = on_stack(&engine, archmages_charm()).expect("charm on stack");
    let seq = engine.journal().len();

    pass_until_priority(&mut engine, P1);
    cast_from_hand(&mut engine, P1, everybody_lives());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        did_not_resolve(&engine, charm, seq),
        "CR 608.2b: P1 gained hexproof"
    );
    assert_eq!(library_size(&engine, P1), p1_lib_before, "P1 drew nothing");

    // Second cast while hexproof is active: P1 cannot be targeted, P0 still can.
    // Everybody Lives also gave Llanowar Elves hexproof, so only Mode 1 is legal
    // and auto-selected.
    reach_main_phase(&mut engine, P0);
    let p0_lib_before = library_size(&engine, P0);
    cast_with_floating(&mut engine, P0, archmages_charm());
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        panic!("expected ChoosePlayer, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![P0], "opponent has hexproof; self is legal");
    assert!(engine.apply(P0, PlayerAction::ChoosePlayer(P1)).is_err());
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(library_size(&engine, P0), p0_lib_before - 2);
}

/// Mode 2 gains control of target nonland permanent with mana value 1 or less,
/// persists after the spell leaves the stack and across turns (CR 611.2a,
/// CR 613.1b), and resets summoning sickness until the new controller's next turn
/// (CR 302.6).
#[test]
fn archmages_charm_mode_two_gains_indefinite_control_across_turns() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[archmages_charm()])
        .battlefield(1, &[llanowar_elves(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let elves = on_battlefield(&engine, P1, llanowar_elves()).expect("elves");
    let ring = on_battlefield(&engine, P1, sol_ring()).expect("sol ring");
    let cast_turn = engine.state().turn.number;

    cast_from_hand(&mut engine, P0, archmages_charm());
    choose_charm_mode(&mut engine, P0, 2);
    choose_target_object(&mut engine, P0, elves);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_stack(&engine, archmages_charm()).is_none());
    assert!(in_graveyard(&engine, P0, archmages_charm()).is_some());
    assert_eq!(engine.state().object(elves).map(|o| o.controller), Some(P0));
    assert_eq!(engine.state().object(ring).map(|o| o.controller), Some(P1));

    // CR 302.6: stolen creature is summoning sick on the turn it changed control.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(
        !attackers.contains(&elves),
        "CR 302.6: summoning sick for P0 this turn"
    );

    // Advance past P1's turn to P0's next turn: control remains indefinitely (CR 611.2a).
    pass_until(&mut engine, |e| {
        e.state().turn.number >= cast_turn + 2
            && e.state().turn.active == P0
            && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert_eq!(
        engine.state().object(elves).map(|o| o.controller),
        Some(P0),
        "CR 611.2a: control lasts until end of game"
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(
        attackers.contains(&elves),
        "no longer summoning sick on P0's next turn"
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elves, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == baylee_engine::turn::Step::CombatEnd
    });
    assert_eq!(engine.state().players[1].life, 19);
}

/// Mode 2 target legality (CR 202.3, 202.3a, 202.3e, 707.2): legal for MV 0–1
/// nonland permanents (including tokens, X permanents on battlefield, and your
/// own permanents); illegal for lands (including artifact lands and Dryad Arbor),
/// MV 2 permanents, token copies of MV 2 permanents, and hexproof permanents.
#[test]
#[allow(clippy::too_many_lines)]
fn archmages_charm_mode_two_target_legality_mv_nonland_tokens_and_x() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), sol_ring()])
        .hand(0, &[archmages_charm()])
        .battlefield(
            1,
            &[
                plains(),
                plains(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                index::ORNITHOPTER,
                llanowar_elves(),
                sol_ring(),
                grizzly_bears(),
                index::SEAT_OF_THE_SYNOD,
                dryad_arbor(),
                index::SLIPPERY_BOGLE,
            ],
        )
        .hand(
            1,
            &[
                raise_the_alarm(),
                index::ENDLESS_ONE,
                index::RITE_OF_REPLICATION,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P1));

    let bears = on_battlefield(&engine, P1, grizzly_bears()).expect("bears");

    // P1 creates two 1/1 Soldier tokens (no mana cost -> MV 0, CR 202.3a),
    cast_from_hand(&mut engine, P1, raise_the_alarm());
    pass_until(&mut engine, stack_is_empty);

    // casts Endless One for X = 3 (on stack MV 3, on battlefield MV 0, CR 202.3e),
    reach_main_phase(&mut engine, P1);
    engine.dev_state_mut(P1).unwrap().players[1]
        .mana_pool
        .add(ManaColor::Blue, 10);
    engine.refresh_offer();
    cast_with_floating(&mut engine, P1, index::ENDLESS_ONE);
    engine.apply(P1, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let endless = on_battlefield(&engine, P1, index::ENDLESS_ONE).expect("endless one");
    assert_eq!(counters_on(&engine, endless, CounterKind::P1P1), 3);

    // and creates a token copy of Grizzly Bears (copies {1}{G} cost -> MV 2, CR 707.2).
    cast_with_floating(&mut engine, P1, index::RITE_OF_REPLICATION);
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    choose_target_object(&mut engine, P1, bears);
    pass_until(&mut engine, stack_is_empty);

    let soldier = engine
        .state()
        .battlefield_view()
        .iter()
        .copied()
        .find(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == P1 && !o.is_card() && o.characteristics().mana_value() == 0
            })
        })
        .expect("Soldier token");
    let bears_token = engine
        .state()
        .battlefield_view()
        .iter()
        .copied()
        .find(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == P1 && !o.is_card() && o.characteristics().mana_value() == 2
            })
        })
        .expect("Grizzly Bears token copy");

    assert!(walk_to_own_main(&mut engine, P0));
    let thopter = on_battlefield(&engine, P1, index::ORNITHOPTER).expect("ornithopter");
    let elves = on_battlefield(&engine, P1, llanowar_elves()).expect("elves");
    let p1_ring = on_battlefield(&engine, P1, sol_ring()).expect("p1 ring");
    let p0_ring = on_battlefield(&engine, P0, sol_ring()).expect("p0 ring");
    let seat = on_battlefield(&engine, P1, index::SEAT_OF_THE_SYNOD).expect("seat");
    let arbor = on_battlefield(&engine, P1, dryad_arbor()).expect("dryad arbor");
    let bogle = on_battlefield(&engine, P1, index::SLIPPERY_BOGLE).expect("bogle");
    let p1_island = on_battlefield(&engine, P1, island()).expect("island");

    cast_from_hand(&mut engine, P0, archmages_charm());
    choose_charm_mode(&mut engine, P0, 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };

    for legal in [thopter, elves, p1_ring, p0_ring, soldier, endless] {
        assert!(
            options.contains(&legal),
            "expected legal target {legal:?} in {options:?}"
        );
    }
    for illegal in [bears, bears_token, p1_island, seat, arbor, bogle] {
        assert!(
            !options.contains(&illegal),
            "illegal target {illegal:?} offered in {options:?}"
        );
        assert!(
            engine
                .apply(
                    P0,
                    PlayerAction::ChooseObjects {
                        objects: vec![illegal]
                    }
                )
                .is_err()
        );
    }

    choose_target_object(&mut engine, P0, soldier);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(soldier).map(|o| o.controller),
        Some(P0)
    );
}

/// Mode 2 steals an Endless One cast for X > 0 (since X=0 on battlefield -> MV 0)
/// and rejects nontoken creatures turned into lands by Ashaya while still
/// allowing token creatures.
#[test]
fn archmages_charm_mode_two_steals_x_permanent_and_excludes_ashaya_lands() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                swamp(),
                mountain(),
            ],
        )
        .hand(0, &[archmages_charm(), archmages_charm(), terminate()])
        .battlefield(
            1,
            &[
                plains(),
                plains(),
                index::ASHAYA_SOUL_OF_THE_WILD,
                llanowar_elves(),
            ],
        )
        .hand(1, &[raise_the_alarm(), index::ENDLESS_ONE])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, P1));

    cast_from_hand(&mut engine, P1, raise_the_alarm());
    pass_until(&mut engine, stack_is_empty);
    reach_main_phase(&mut engine, P1);
    engine.dev_state_mut(P1).unwrap().players[1]
        .mana_pool
        .add(ManaColor::Blue, 4);
    engine.refresh_offer();
    cast_with_floating(&mut engine, P1, index::ENDLESS_ONE);
    engine.apply(P1, PlayerAction::ChooseNumber(4)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let endless = on_battlefield(&engine, P1, index::ENDLESS_ONE).expect("endless one");
    let elves = on_battlefield(&engine, P1, llanowar_elves()).expect("elves");
    let soldier = engine
        .state()
        .battlefield_view()
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.controller == P1 && !o.is_card())
        })
        .expect("soldier token");

    assert!(walk_to_own_main(&mut engine, P0));
    let islands = all_on_battlefield(&engine, P0, island());
    tap_mana_where(&mut engine, P0, |id| islands.contains(&id));
    cast_with_floating(&mut engine, P0, archmages_charm());
    choose_charm_mode(&mut engine, P0, 2);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        !options.contains(&elves),
        "Ashaya makes nontoken Elves a Land"
    );
    assert!(
        !options.contains(&endless),
        "Ashaya makes nontoken Endless One a Land too"
    );
    assert!(options.contains(&soldier), "Ashaya leaves tokens nonland");
    choose_target_object(&mut engine, P0, soldier);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(soldier).map(|o| o.controller),
        Some(P0)
    );

    // Once Ashaya is gone, Endless One (MV 0 on board despite 4 counters) is stealable.
    let ashaya = on_battlefield(&engine, P1, index::ASHAYA_SOUL_OF_THE_WILD).expect("ashaya");
    cast_from_hand(&mut engine, P0, terminate());
    choose_target_object(&mut engine, P0, ashaya);
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, P0, archmages_charm());
    choose_charm_mode(&mut engine, P0, 2);
    choose_target_object(&mut engine, P0, endless);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(endless).map(|o| o.controller),
        Some(P0)
    );
    assert_eq!(counters_on(&engine, endless, CounterKind::P1P1), 4);
}

/// Mode 2 respects CR 608.2b illegality before resolution, Guardian Beast's
/// control-change guard without leaving a dormant effect, and CR 700.2g modal
/// spell copying.
#[test]
#[allow(clippy::too_many_lines)]
fn archmages_charm_mode_two_resolution_guards_and_modal_copy() {
    // Subcase A: target gains hexproof in response -> CR 608.2b fizzle.
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[archmages_charm()])
        .battlefield(1, &[forest(), forest(), llanowar_elves()])
        .hand(1, &[heroic_intervention()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let elves = on_battlefield(&engine, P1, llanowar_elves()).expect("elves");
    cast_from_hand(&mut engine, P0, archmages_charm());
    choose_charm_mode(&mut engine, P0, 2);
    choose_target_object(&mut engine, P0, elves);
    let charm = on_stack(&engine, archmages_charm()).expect("charm");
    let seq = engine.journal().len();

    pass_until_priority(&mut engine, P1);
    cast_from_hand(&mut engine, P1, heroic_intervention());
    pass_until(&mut engine, stack_is_empty);

    assert!(did_not_resolve(&engine, charm, seq));
    assert_eq!(engine.state().object(elves).map(|o| o.controller), Some(P1));

    // Subcase B: Guardian Beast blocks gaining control of Sol Ring and leaves no dormant effect.
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[archmages_charm()])
        .battlefield(1, &[index::GUARDIAN_BEAST, sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ring = on_battlefield(&engine, P1, sol_ring()).expect("sol ring");
    let beast = on_battlefield(&engine, P1, index::GUARDIAN_BEAST).expect("guardian beast");
    cast_from_hand(&mut engine, P0, archmages_charm());
    choose_charm_mode(&mut engine, P0, 2);
    choose_target_object(&mut engine, P0, ring);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(ring).map(|o| o.controller), Some(P1));
    kill(&mut engine, beast);
    assert_eq!(engine.state().object(ring).map(|o| o.controller), Some(P1));

    // Subcase C: CR 700.2g — Storm of Saruman copies Mode 1 and keeps the chosen mode.
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), storm_of_saruman()])
        .hand(0, &[index::ORNITHOPTER, archmages_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);
    cast_from_hand(&mut engine, P0, index::ORNITHOPTER);
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, P0, archmages_charm());
    choose_charm_mode(&mut engine, P0, 1);
    choose_target_player(&mut engine, P0, P0);
    while !stack_is_empty(&engine) {
        match engine.pending() {
            Pending::ChooseCastMode { .. } => {
                panic!("CR 700.2g: copy of modal spell must not choose a different mode");
            }
            Pending::ChooseTargets {
                player,
                reason: baylee_engine::choice::TargetPrompt::Retarget { .. },
                ..
            } => {
                // Keep the original target P0.
                let p = *player;
                engine
                    .apply(
                        p,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            _ => {
                let (p, a) = answer_one(&engine).expect("answer copy prompts");
                engine.apply(p, a).unwrap();
            }
        }
    }
    assert_eq!(
        library_size(&engine, P0),
        lib_before - 4,
        "mode 1 copied -> drew 2 twice"
    );
    assert_eq!(library_size(&engine, P1), p1_lib_before);
}
