//! `cards/1997/src/por/time_ebb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index::{
    CONTROL_MAGIC, DARKSTEEL_GARGOYLE, GRIZZLY_BEARS, HILL_GIANT, KALONIAN_BEHEMOTH,
    NACATL_OUTLANDER, SACRED_WOLF, SNAKESKIN_VEIL, SPROUT,
};
use baylee_engine::event::{GameEvent, LibraryPlace};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn time_ebb() -> CardIndex {
    card_index("30cc8f7b-3c28-40f5-8f8f-157e8212280b")
}

/// Time Ebb targets only creatures (refusing artifacts, enchantments, and lands),
/// puts an opponent's creature on top of its owner's library, and leaves that
/// creature as their next draw.
#[test]
fn time_ebb_puts_opponents_creature_on_top_of_owners_library_and_they_draw_it() {
    let te = time_ebb();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(
            1,
            &[GRIZZLY_BEARS, sol_ring(), their_enchantment(), forest()],
        )
        .hand(0, &[te])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let ring = on_battlefield(&engine, P1, sol_ring()).unwrap();
    let chant = on_battlefield(&engine, P1, their_enchantment()).unwrap();
    let p1_forest = on_battlefield(&engine, P1, forest()).unwrap();
    let p0_lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);

    cast_from_hand(&mut engine, P0, te);
    match engine.pending().clone() {
        Pending::ChooseTargets { options, .. } => {
            assert_eq!(options, vec![bears], "only creatures are legal targets");
            assert!(
                !options.contains(&ring)
                    && !options.contains(&chant)
                    && !options.contains(&p1_forest),
                "noncreature permanents must not be offered"
            );
        }
        other => panic!("expected ChooseTargets, got {other:?}"),
    }

    // Explicitly verify noncreature permanents are refused by the engine.
    for illegal in [ring, chant, p1_forest] {
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
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert!(in_graveyard(&engine, P0, te).is_some());
    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P1, GRIZZLY_BEARS).is_none());
    assert_eq!(library_size(&engine, P0), p0_lib_before);
    assert_eq!(library_size(&engine, P1), p1_lib_before + 1);
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(bears),
        "creature must sit on top of its owner's library"
    );

    // On P1's next turn, their draw step draws that exact creature.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(in_hand(&engine, P1, GRIZZLY_BEARS), Some(bears));
    assert_eq!(library_size(&engine, P1), p1_lib_before);
}

/// A creature you control that an opponent owns goes on top of its owner's
/// library, not its controller's (CR 108.3, CR 110.2), and an Aura attached to
/// it goes to its owner's graveyard as a state-based action (CR 704.5m).
#[test]
fn time_ebb_puts_stolen_creature_on_owners_library_not_controllers() {
    let te = time_ebb();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(); 7])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[CONTROL_MAGIC, te])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    // Gain control of P1's Hill Giant with Control Magic first.
    let islands = all_on_battlefield(&engine, P0, island());
    tap_mana_where(&mut engine, P0, |id| islands[..4].contains(&id));
    cast_with_floating(&mut engine, P0, CONTROL_MAGIC);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert_eq!(on_battlefield(&engine, P0, HILL_GIANT), Some(giant));
    let obj = engine.state().object(giant).unwrap();
    assert_eq!(obj.controller, P0, "P0 now controls the stolen creature");
    assert_eq!(
        obj.owner, P1,
        "P1 still owns the stolen creature (CR 108.3)"
    );

    let p0_lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);

    tap_mana_where(&mut engine, P0, |id| islands[4..7].contains(&id));
    cast_with_floating(&mut engine, P0, te);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert!(on_battlefield(&engine, P0, HILL_GIANT).is_none());
    assert!(on_battlefield(&engine, P1, HILL_GIANT).is_none());
    assert_eq!(
        library_size(&engine, P0),
        p0_lib_before,
        "controller's library must be unchanged"
    );
    assert_eq!(
        library_size(&engine, P1),
        p1_lib_before + 1,
        "owner's library gains the card"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(giant),
        "stolen creature goes on top of its owner's library"
    );
    assert!(
        in_graveyard(&engine, P0, CONTROL_MAGIC).is_some(),
        "unattached Control Magic goes to P0's graveyard (CR 704.5m)"
    );
    assert!(in_graveyard(&engine, P0, te).is_some());

    // P1 draws their Hill Giant on their next draw step.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(in_hand(&engine, P1, HILL_GIANT), Some(giant));
}

/// Time Ebb may target a creature you own and control, putting it on top of
/// your own library so you draw it on your next turn.
#[test]
fn time_ebb_puts_own_creature_on_own_library_top_and_you_draw_it() {
    let te = time_ebb();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island(), island(), GRIZZLY_BEARS])
        .hand(0, &[te])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    let p0_lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);

    cast_from_hand(&mut engine, P0, te);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert!(on_battlefield(&engine, P0, GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P0, GRIZZLY_BEARS).is_none());
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

    assert!(walk_to_own_main(&mut engine, P1));
    assert!(walk_to_own_main(&mut engine, P0));
    assert_eq!(in_hand(&engine, P0, GRIZZLY_BEARS), Some(bears));
}

/// A creature token targeted by Time Ebb is moved to the top of its owner's
/// library and then ceases to exist as a state-based action, leaving the
/// library size and its top card unchanged (CR 111.7, CR 111.8, CR 704.5d).
#[test]
fn time_ebb_puts_creature_token_on_top_and_token_ceases_to_exist() {
    let te = time_ebb();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[forest()])
        .hand(0, &[te])
        .hand(1, &[SPROUT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Pass priority to P1 so P1 casts Sprout to create a 1/1 Saproling token.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, SPROUT);
    pass_until(&mut engine, |e| at_rest(e, P0));

    let token_id = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_none() && o.controller == P1)
        })
        .expect("P1 created a Saproling token");

    let p0_lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);
    let p1_top_before = engine
        .state()
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .copied();
    let since = engine.state().journal.last_seq();

    cast_from_hand(&mut engine, P0, te);
    let options = aim_at(&mut engine, P0, token_id);
    assert_eq!(options, vec![token_id]);
    pass_until(&mut engine, |e| at_rest(e, P0));

    // The token did move from the battlefield to the top of the library...
    assert!(
        engine.state().journal.entries()[since as usize..]
            .iter()
            .any(|entry| matches!(
                entry.event,
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Battlefield,
                    to: Zone::Library,
                    place: Some(LibraryPlace::Top),
                    ..
                } if object == token_id
            )),
        "token should move to the top of its owner's library before ceasing to exist"
    );
    // ...and then ceased to exist as an SBA (CR 704.5d), leaving the library intact.
    assert!(
        engine.state().object(token_id).is_none(),
        "token outside the battlefield ceases to exist (CR 111.7, CR 704.5d)"
    );
    assert_eq!(library_size(&engine, P0), p0_lib_before);
    assert_eq!(
        library_size(&engine, P1),
        p1_lib_before,
        "owner's library size must be unchanged after token ceases to exist"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        p1_top_before,
        "underlying top card of owner's library must remain on top"
    );
    assert!(in_graveyard(&engine, P0, te).is_some());

    // P1's next draw is the original top Plains, not a ghost token.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(in_hand(&engine, P1, plains()), p1_top_before);
}

/// If the targeted creature leaves the battlefield before Time Ebb resolves,
/// its sole target is illegal, so Time Ebb does not resolve and goes to its
/// owner's graveyard with nothing moved to any library (CR 608.2b).
#[test]
fn time_ebb_fails_to_resolve_if_target_leaves_battlefield_in_response() {
    let te = time_ebb();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS, mountain()])
        .hand(0, &[te])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let p0_lib_before = library_size(&engine, P0);
    let p1_lib_before = library_size(&engine, P1);

    cast_from_hand(&mut engine, P0, te);
    aim_at(&mut engine, P0, bears);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    // P1 destroys the targeted creature in response with Lightning Bolt.
    cast_from_hand(&mut engine, P1, lightning_bolt());
    aim_at(&mut engine, P1, bears);
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none());
    assert_eq!(
        in_graveyard(&engine, P1, GRIZZLY_BEARS),
        Some(bears),
        "creature remains in graveyard rather than moving to library (CR 608.2b)"
    );
    assert_eq!(library_size(&engine, P0), p0_lib_before);
    assert_eq!(library_size(&engine, P1), p1_lib_before);
    assert!(in_graveyard(&engine, P0, te).is_some());
}

/// If the targeted creature gains hexproof in response, it is no longer a legal
/// target at resolution, so Time Ebb fails to resolve and the creature stays on
/// the battlefield (CR 608.2b, CR 702.11b).
#[test]
fn time_ebb_fails_to_resolve_if_target_gains_hexproof_in_response() {
    let te = time_ebb();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS, forest()])
        .hand(0, &[te])
        .hand(1, &[SNAKESKIN_VEIL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let p1_lib_before = library_size(&engine, P1);

    cast_from_hand(&mut engine, P0, te);
    aim_at(&mut engine, P0, bears);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, P1, SNAKESKIN_VEIL);
    aim_at(&mut engine, P1, bears);
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert_eq!(
        on_battlefield(&engine, P1, GRIZZLY_BEARS),
        Some(bears),
        "creature gained hexproof in response and must stay on the battlefield"
    );
    assert_eq!(library_size(&engine, P1), p1_lib_before);
    assert!(in_graveyard(&engine, P0, te).is_some());
}

/// Time Ebb cannot target an opponent's hexproof creature (CR 702.11b), any
/// creature with shroud (CR 702.18a), or a creature with protection from blue
/// (CR 702.16b), but it CAN target an indestructible creature (CR 702.12b) and
/// your own hexproof creature (CR 702.11b).
#[test]
fn time_ebb_respects_hexproof_shroud_protection_and_tucks_indestructible() {
    let te = time_ebb();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                SACRED_WOLF,
            ],
        )
        .battlefield(
            1,
            &[
                SACRED_WOLF,
                KALONIAN_BEHEMOTH,
                NACATL_OUTLANDER,
                DARKSTEEL_GARGOYLE,
            ],
        )
        .hand(0, &[te, te])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_wolf = on_battlefield(&engine, P0, SACRED_WOLF).unwrap();
    let their_wolf = on_battlefield(&engine, P1, SACRED_WOLF).unwrap();
    let behemoth = on_battlefield(&engine, P1, KALONIAN_BEHEMOTH).unwrap();
    let outlander = on_battlefield(&engine, P1, NACATL_OUTLANDER).unwrap();
    let gargoyle = on_battlefield(&engine, P1, DARKSTEEL_GARGOYLE).unwrap();

    let islands = all_on_battlefield(&engine, P0, island());
    tap_mana_where(&mut engine, P0, |id| islands[..3].contains(&id));
    cast_with_floating(&mut engine, P0, te);

    match engine.pending().clone() {
        Pending::ChooseTargets { options, .. } => {
            assert!(options.contains(&my_wolf), "own hexproof creature is legal");
            assert!(
                options.contains(&gargoyle),
                "indestructible creature is legal"
            );
            assert!(
                !options.contains(&their_wolf),
                "opponent's hexproof creature is illegal"
            );
            assert!(!options.contains(&behemoth), "shroud creature is illegal");
            assert!(
                !options.contains(&outlander),
                "protection from blue creature is illegal"
            );
        }
        other => panic!("expected ChooseTargets, got {other:?}"),
    }

    for illegal in [their_wolf, behemoth, outlander] {
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
            "illegal target {illegal:?} must be refused"
        );
    }

    // Tuck the indestructible creature (moving to library is not destruction).
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![gargoyle],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert!(on_battlefield(&engine, P1, DARKSTEEL_GARGOYLE).is_none());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .last()
            .copied(),
        Some(gargoyle)
    );

    // Tuck your own hexproof creature.
    tap_mana_where(&mut engine, P0, |id| islands[3..6].contains(&id));
    cast_with_floating(&mut engine, P0, te);
    aim_at(&mut engine, P0, my_wolf);
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert!(on_battlefield(&engine, P0, SACRED_WOLF).is_none());
    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .last()
            .copied(),
        Some(my_wolf)
    );
}

/// Countering Time Ebb prevents its effect and puts it into its owner's
/// graveyard while leaving the target creature on the battlefield (CR 701.6a).
#[test]
fn time_ebb_countered_leaves_creature_on_battlefield() {
    let te = time_ebb();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS, island(), island()])
        .hand(0, &[te])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    let p1_lib_before = library_size(&engine, P1);

    cast_from_hand(&mut engine, P0, te);
    aim_at(&mut engine, P0, bears);
    let spell_on_stack = engine.state().zones.list(ZoneLocation::Stack)[0];
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, P1, counterspell());
    aim_at(&mut engine, P1, spell_on_stack);
    pass_until(&mut engine, |e| at_rest(e, P0));

    assert_eq!(on_battlefield(&engine, P1, GRIZZLY_BEARS), Some(bears));
    assert_eq!(library_size(&engine, P1), p1_lib_before);
    assert!(in_graveyard(&engine, P0, te).is_some());
    assert!(in_graveyard(&engine, P1, counterspell()).is_some());
}

/// Time Ebb costs {2}{U} and requires a legal creature target (CR 601.2c):
/// floating {G}{G}{G}, floating {U}{U}, or having only noncreature permanents
/// on the battlefield with {U}{U}{U} floating all prevent casting.
#[test]
fn time_ebb_requires_two_and_blue_and_a_legal_creature_target() {
    let te = time_ebb();

    // 1. Wrong colour ({G}{G}{G} floating): not castable even with a creature out.
    let mut no_blue = Duel::new(SEED, plains())
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[te])
        .start();
    keep_mulligans(&mut no_blue);
    reach_main_phase(&mut no_blue, P0);
    tap_all_mana(&mut no_blue, P0);
    let te_id = in_hand(&no_blue, P0, te).unwrap();
    assert!(
        matches!(no_blue.pending(), Pending::Priority { legal, .. } if !legal.castable.contains(&te_id)),
        "{{G}}{{G}}{{G}} cannot pay {{2}}{{U}}"
    );
    assert!(
        no_blue
            .apply(P0, PlayerAction::CastSpell { card: te_id })
            .is_err()
    );

    // 2. Short mana ({U}{U} floating): not castable even with a creature out.
    let mut short_mana = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[te])
        .start();
    keep_mulligans(&mut short_mana);
    reach_main_phase(&mut short_mana, P0);
    tap_all_mana(&mut short_mana, P0);
    let te_id = in_hand(&short_mana, P0, te).unwrap();
    assert!(
        matches!(short_mana.pending(), Pending::Priority { legal, .. } if !legal.castable.contains(&te_id)),
        "{{U}}{{U}} cannot pay {{2}}{{U}}"
    );
    assert!(
        short_mana
            .apply(P0, PlayerAction::CastSpell { card: te_id })
            .is_err()
    );

    // 3. No creature on the battlefield (with {U}{U}{U} floating): not castable (CR 601.2c).
    let mut no_creature = Duel::new(SEED, plains())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[sol_ring(), their_enchantment()])
        .hand(0, &[te])
        .start();
    keep_mulligans(&mut no_creature);
    reach_main_phase(&mut no_creature, P0);
    tap_all_mana(&mut no_creature, P0);
    let te_id = in_hand(&no_creature, P0, te).unwrap();
    assert!(
        matches!(no_creature.pending(), Pending::Priority { legal, .. } if !legal.castable.contains(&te_id)),
        "Time Ebb cannot be cast with no legal creature target"
    );
    assert!(
        no_creature
            .apply(P0, PlayerAction::CastSpell { card: te_id })
            .is_err()
    );
}

/// Time Ebb is a sorcery (CR 307.1): with {U}{U}{U} floating, it is castable
/// in P0's main phase on an empty stack, refused while a spell is on the stack,
/// castable again once the stack clears, and refused on the opponent's turn.
#[test]
fn time_ebb_requires_sorcery_timing() {
    let te = time_ebb();
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                forest(),
            ],
        )
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[te, SPROUT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let te_id = in_hand(&engine, P0, te).unwrap();
    let islands = all_on_battlefield(&engine, P0, island());
    tap_mana_where(&mut engine, P0, |id| islands[..3].contains(&id));
    assert!(
        matches!(engine.pending(), Pending::Priority { legal, .. } if legal.castable.contains(&te_id)),
        "control: Time Ebb is castable in P0's main phase with {{U}}{{U}}{{U}} and empty stack"
    );

    // Put Sprout on the stack while {U}{U}{U} stays floating.
    let p0_forest = on_battlefield(&engine, P0, forest()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == p0_forest);
    cast_with_floating(&mut engine, P0, SPROUT);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, legal, .. } if *player == P0 && !legal.castable.contains(&te_id)),
        "sorcery cannot be cast while the stack is non-empty (CR 307.1)"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: te_id })
            .is_err()
    );
    pass_until(&mut engine, |e| at_rest(e, P0));
    assert!(
        matches!(engine.pending(), Pending::Priority { legal, .. } if legal.castable.contains(&te_id)),
        "control: once Sprout resolves, Time Ebb is castable again with the still-floating {{U}}{{U}}{{U}}"
    );

    // Advance to P1's main phase, float {U}{U}{U} from the three untapped Islands:
    // still not castable on the opponent's turn.
    reach_their_main_phase(&mut engine, P1);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_mana_where(&mut engine, P0, |id| islands[3..6].contains(&id));
    assert!(
        matches!(engine.pending(), Pending::Priority { player, legal, .. } if *player == P0 && !legal.castable.contains(&te_id)),
        "sorcery cannot be cast during the opponent's turn even with {{U}}{{U}}{{U}} floating (CR 307.1)"
    );
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: te_id })
            .is_err()
    );
}
