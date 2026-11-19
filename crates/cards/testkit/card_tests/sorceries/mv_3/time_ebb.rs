//! `cards/1997/src/por/time_ebb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index::{
    CONTROL_MAGIC, DARKSTEEL_MYR, GRIZZLY_BEARS, HILL_GIANT, MIST_LEOPARD, RAISE_THE_ALARM,
    SCRYB_RANGER, SLIPPERY_BOGLE, SNAKESKIN_VEIL,
};
use baylee_engine::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn time_ebb() -> CardIndex {
    card_index("30cc8f7b-3c28-40f5-8f8f-157e8212280b")
}

fn tap_mana_for(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) {
    for source in all_on_battlefield(engine, seat, card) {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("basic land taps for mana");
    }
}

fn soldier_tokens(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == seat
                    && o.card.is_none()
                    && o.characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::creature::SOLDIER)
            })
        })
        .collect()
}

/// Time Ebb ({2}{U}, Sorcery): "Put target creature on top of its owner's
/// library." Pays {2}{U} from lands, refuses noncreature targets, puts an
/// opponent's creature on top of their library, and makes it their next draw.
#[test]
fn time_ebb_puts_opponents_creature_on_top_and_refuses_noncreatures() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(
            1,
            &[
                GRIZZLY_BEARS,
                quiet_artifact(),
                their_enchantment(),
                forest(),
            ],
        )
        .hand(0, &[time_ebb()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let artifact = on_battlefield(&engine, P1, quiet_artifact()).unwrap();
    let enchantment = on_battlefield(&engine, P1, their_enchantment()).unwrap();
    let their_forest = on_battlefield(&engine, P1, forest()).unwrap();
    let p0_island = all_on_battlefield(&engine, P0, island())[0];
    let p0_lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);

    tap_mana_for(&mut engine, P0, island());
    let spell = in_hand(&engine, P0, time_ebb()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(legal.castable.contains(&spell), "{{2}}{{U}} pays Time Ebb");
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(options, vec![bears], "only creatures are legal targets");
    for illegal in [artifact, enchantment, their_forest, p0_island] {
        assert!(
            engine
                .apply(
                    P0,
                    PlayerAction::ChooseTargets {
                        objects: vec![illegal],
                        players: vec![],
                    },
                )
                .is_err(),
            "noncreature permanent {illegal:?} must be refused"
        );
    }

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P1, GRIZZLY_BEARS).is_none());
    assert_eq!(in_graveyard(&engine, P0, time_ebb()), Some(spell));
    assert_eq!(library_size(&engine, P0), p0_lib_before);
    assert_eq!(library_size(&engine, P1), p1_lib_before + 1);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(bears)
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(in_hand(&engine, P1, GRIZZLY_BEARS), Some(bears));
    assert_eq!(library_size(&engine, P1), p1_lib_before);
}

/// CR 108.3 / CR 110.2: a creature you control that an opponent owns goes to
/// the TOP of its owner's (the opponent's) library, not your library.
#[test]
fn time_ebb_puts_stolen_creature_on_its_owners_library_top() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 7])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[CONTROL_MAGIC, time_ebb()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, P0, CONTROL_MAGIC);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().controller, P0);
    assert_eq!(engine.state().object(giant).unwrap().owner, P1);

    let p0_lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);
    cast_from_hand(&mut engine, P0, time_ebb());
    let options = aim_at(&mut engine, P0, giant);
    assert_eq!(options, vec![giant]);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, P0, CONTROL_MAGIC).is_some());
    assert_eq!(library_size(&engine, P0), p0_lib_before);
    assert_eq!(library_size(&engine, P1), p1_lib_before + 1);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(giant),
        "stolen creature goes to owner's library, not controller's"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(in_hand(&engine, P1, HILL_GIANT), Some(giant));
}

/// CR 108.3 / CR 110.2: if an opponent stole your creature, Time Ebb still
/// puts it on top of YOUR library (its owner's), not theirs.
#[test]
fn time_ebb_puts_your_creature_stolen_by_opponent_on_your_library_top() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), GRIZZLY_BEARS])
        .battlefield(1, &[island(); 4])
        .hand(0, &[time_ebb()])
        .hand(1, &[CONTROL_MAGIC])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P1, CONTROL_MAGIC);
    aim_at(&mut engine, P1, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bears).unwrap().controller, P1);
    assert_eq!(engine.state().object(bears).unwrap().owner, P0);

    reach_their_main_phase(&mut engine, P0);
    let p0_lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);
    cast_from_hand(&mut engine, P0, time_ebb());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P1, CONTROL_MAGIC).is_some());
    assert_eq!(library_size(&engine, P1), p1_lib_before);
    assert_eq!(library_size(&engine, P0), p0_lib_before + 1);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .last()
            .copied(),
        Some(bears),
        "creature stolen by opponent returns to its owner's (P0's) library"
    );
}

/// Time Ebb can also target your own creature, putting it on top of your own
/// library so you draw it on your next turn.
#[test]
fn time_ebb_puts_own_creature_on_own_library_top() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), GRIZZLY_BEARS])
        .hand(0, &[time_ebb()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    let p0_lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);

    cast_from_hand(&mut engine, P0, time_ebb());
    let options = aim_at(&mut engine, P0, bears);
    assert_eq!(options, vec![bears]);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, GRIZZLY_BEARS).is_none());
    assert_eq!(library_size(&engine, P1), p1_lib_before);
    assert_eq!(library_size(&engine, P0), p0_lib_before + 1);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .last()
            .copied(),
        Some(bears)
    );

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(in_hand(&engine, P0, GRIZZLY_BEARS), Some(bears));
    assert_eq!(library_size(&engine, P0), p0_lib_before);
}

/// CR 111.2 / CR 111.8 / CR 704.5d: a creature token leaves the battlefield to
/// its owner's library and ceases to exist, leaving library size and top card
/// unchanged.
#[test]
fn time_ebb_puts_creature_token_on_top_and_it_ceases_to_exist() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(); 7])
        .battlefield(1, &[plains(), plains()])
        .hand(0, &[time_ebb(), CONTROL_MAGIC, time_ebb()])
        .hand(1, &[RAISE_THE_ALARM])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    cast_from_hand(&mut engine, P1, RAISE_THE_ALARM);
    pass_until(&mut engine, stack_is_empty);
    let tokens = soldier_tokens(&engine, P1);
    assert_eq!(tokens.len(), 2, "Raise the Alarm created two tokens");
    let (tok_a, tok_b) = (tokens[0], tokens[1]);

    reach_their_main_phase(&mut engine, P0);
    let p0_lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);
    let p1_top_before = engine
        .state()
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .copied()
        .unwrap();

    // Target opponent's token directly.
    cast_from_hand(&mut engine, P0, time_ebb());
    aim_at(&mut engine, P0, tok_a);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine.state().object(tok_a).is_none(),
        "token ceased to exist off the battlefield (CR 704.5d)"
    );
    assert!(
        engine
            .state()
            .object(tok_b)
            .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == P1)
    );
    assert_eq!(library_size(&engine, P1), p1_lib_before);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(p1_top_before)
    );

    // Also steal the second token with Control Magic first, then Time Ebb it:
    // owner is still P1 (CR 111.2), token still ceases to exist, neither library grows.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let p1_lib_before_second = library_size(&engine, P1);
    let p1_top_before_second = engine
        .state()
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .copied()
        .unwrap();

    cast_from_hand(&mut engine, P0, CONTROL_MAGIC);
    aim_at(&mut engine, P0, tok_b);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(tok_b).unwrap().controller, P0);
    assert_eq!(engine.state().object(tok_b).unwrap().owner, P1);

    cast_from_hand(&mut engine, P0, time_ebb());
    aim_at(&mut engine, P0, tok_b);
    pass_until(&mut engine, stack_is_empty);

    assert!(engine.state().object(tok_b).is_none());
    assert!(in_graveyard(&engine, P0, CONTROL_MAGIC).is_some());
    assert_eq!(library_size(&engine, P0), p0_lib_before - 1); // P0 drew 1 for the new turn
    assert_eq!(library_size(&engine, P1), p1_lib_before_second);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(p1_top_before_second)
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .contains(&tok_b)
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(P1))
            .contains(&p1_top_before_second),
        "P1 draws the real card from top, never a token"
    );
}

/// CR 608.2b: if the target creature leaves the battlefield in response, Time
/// Ebb's only target is illegal and it does not resolve (nothing happens).
#[test]
fn time_ebb_target_leaving_battlefield_in_response_does_nothing() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[island(), GRIZZLY_BEARS])
        .hand(0, &[time_ebb()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let p1_lib_before = library_size(&engine, P1);
    let spell = in_hand(&engine, P0, time_ebb()).unwrap();

    cast_from_hand(&mut engine, P0, time_ebb());
    aim_at(&mut engine, P0, bears);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );

    tap_mana_for(&mut engine, P1, island());
    cast_with_floating(&mut engine, P1, unsummon());
    aim_at(&mut engine, P1, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(in_hand(&engine, P1, GRIZZLY_BEARS), Some(bears));
    assert_eq!(library_size(&engine, P1), p1_lib_before);
    assert_eq!(in_graveyard(&engine, P0, time_ebb()), Some(spell));
    assert!(engine.state().journal.entries().iter().any(|e| matches!(
        e.event,
        GameEvent::StackObjectDidNotResolve { object } if object == spell
    )));
}

/// CR 608.2b: if the target creature gains hexproof in response, it becomes an
/// illegal target and Time Ebb fails to resolve.
#[test]
fn time_ebb_target_gaining_hexproof_in_response_fizzles() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[forest(), GRIZZLY_BEARS])
        .hand(0, &[time_ebb()])
        .hand(1, &[SNAKESKIN_VEIL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let p1_lib_before = library_size(&engine, P1);
    let spell = in_hand(&engine, P0, time_ebb()).unwrap();

    cast_from_hand(&mut engine, P0, time_ebb());
    aim_at(&mut engine, P0, bears);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );

    tap_mana_for(&mut engine, P1, forest());
    cast_with_floating(&mut engine, P1, SNAKESKIN_VEIL);
    aim_at(&mut engine, P1, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(on_battlefield(&engine, P1, GRIZZLY_BEARS), Some(bears));
    assert_eq!(pt(&engine, bears), (3, 3));
    assert_eq!(library_size(&engine, P1), p1_lib_before);
    assert_eq!(in_graveyard(&engine, P0, time_ebb()), Some(spell));
    assert!(engine.state().journal.entries().iter().any(|e| matches!(
        e.event,
        GameEvent::StackObjectDidNotResolve { object } if object == spell
    )));
}

/// Targeting boundaries: opponent's hexproof (CR 702.11b), any shroud
/// (CR 702.18a), and any protection from blue (CR 702.16b) are illegal targets;
/// own hexproof and indestructible (CR 702.12b) creatures are legal targets.
#[test]
fn time_ebb_respects_hexproof_shroud_protection_and_moves_indestructible() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                SLIPPERY_BOGLE,
                MIST_LEOPARD,
                SCRYB_RANGER,
            ],
        )
        .battlefield(
            1,
            &[SLIPPERY_BOGLE, MIST_LEOPARD, SCRYB_RANGER, DARKSTEEL_MYR],
        )
        .hand(0, &[time_ebb()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let own_bogle = on_battlefield(&engine, P0, SLIPPERY_BOGLE).unwrap();
    let own_leopard = on_battlefield(&engine, P0, MIST_LEOPARD).unwrap();
    let own_ranger = on_battlefield(&engine, P0, SCRYB_RANGER).unwrap();
    let opp_bogle = on_battlefield(&engine, P1, SLIPPERY_BOGLE).unwrap();
    let opp_leopard = on_battlefield(&engine, P1, MIST_LEOPARD).unwrap();
    let opp_ranger = on_battlefield(&engine, P1, SCRYB_RANGER).unwrap();
    let myr = on_battlefield(&engine, P1, DARKSTEEL_MYR).unwrap();

    cast_from_hand(&mut engine, P0, time_ebb());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(options.contains(&own_bogle) && options.contains(&myr));
    assert_eq!(options.len(), 2);

    for illegal in [own_leopard, own_ranger, opp_bogle, opp_leopard, opp_ranger] {
        assert!(
            engine
                .apply(
                    P0,
                    PlayerAction::ChooseTargets {
                        objects: vec![illegal],
                        players: vec![],
                    },
                )
                .is_err(),
            "protected/shroud/opponent-hexproof target {illegal:?} must be refused"
        );
    }

    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![myr],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, DARKSTEEL_MYR).is_none());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(myr)
    );
}

/// Casting constraints and countering: uncastable with no creatures (CR 601.2c),
/// sorcery timing only (CR 307.1), and countering leaves the creature on board
/// with no mana refund (CR 701.6a / CR 701.6b).
#[test]
fn time_ebb_requires_target_sorcery_timing_and_spends_mana_if_countered() {
    let mut no_creature = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[quiet_artifact(), their_enchantment()])
        .hand(0, &[time_ebb()])
        .start();
    keep_mulligans(&mut no_creature);
    reach_main_phase(&mut no_creature, P0);
    tap_mana_for(&mut no_creature, P0, island());
    let spell_empty = in_hand(&no_creature, P0, time_ebb()).unwrap();
    let Pending::Priority { legal, .. } = no_creature.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.castable.contains(&spell_empty),
        "uncastable without a creature target (CR 601.2c)"
    );
    assert!(
        no_creature
            .apply(P0, PlayerAction::CastSpell { card: spell_empty })
            .is_err()
    );

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[island(), island(), GRIZZLY_BEARS])
        .hand(0, &[time_ebb()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);
    let spell = in_hand(&engine, P0, time_ebb()).unwrap();
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "sorcery cannot be cast on opponent's turn (CR 307.1)"
    );

    reach_their_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let p1_lib_before = library_size(&engine, P1);

    tap_mana_for(&mut engine, P0, island());
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    aim_at(&mut engine, P0, bears);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );

    tap_mana_for(&mut engine, P1, island());
    cast_with_floating(&mut engine, P1, counterspell());
    aim_at(&mut engine, P1, spell);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(on_battlefield(&engine, P1, GRIZZLY_BEARS), Some(bears));
    assert_eq!(library_size(&engine, P1), p1_lib_before);
    assert_eq!(in_graveyard(&engine, P0, time_ebb()), Some(spell));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(
        all_on_battlefield(&engine, P0, island())
            .iter()
            .all(|&id| engine
                .state()
                .object(id)
                .unwrap()
                .status
                .contains(Status::TAPPED)),
        "paid mana is not refunded when countered (CR 701.6b)"
    );
}
