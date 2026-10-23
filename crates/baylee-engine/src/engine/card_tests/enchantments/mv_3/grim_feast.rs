//! `cards/enchantments/mv_3/grim_feast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, LIGHTNING_BOLT, WALL_OF_WOOD};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grim_feast() -> CardIndex {
    card_index("331233ba-0ddc-4cb2-b3b7-1b782e03529f")
}

/// Grim Feast (331233ba-0ddc-4cb2-b3b7-1b782e03529f) — {1}{B}{G} Enchantment:
/// "At the beginning of your upkeep, this enchantment deals 1 damage to you.
/// Whenever a creature is put into an opponent's graveyard from the
/// battlefield, you gain life equal to its toughness."
#[test]
fn grim_feast_gains_life_for_opponent_creature_dying() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), forest(), mountain(), mountain()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[grim_feast(), LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, grim_feast());
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Hill Giant (3/3) dies -> P0 gains 3 life (20 -> 23).
    assert_eq!(engine.state().players[0].life, 23);
}

/// P0 casts Grim Feast, then Lightning Bolt at `target` (owned by `owner`),
/// with P1's board `theirs`. Returns the engine and P0's life afterwards.
fn feast_then_bolt(
    mine: &[CardIndex],
    theirs: &[CardIndex],
    owner: PlayerId,
    target: CardIndex,
) -> Engine<RegistryLookup> {
    let mut field = vec![swamp(), forest(), mountain(), mountain()];
    field.extend_from_slice(mine);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &field)
        .battlefield(1, theirs)
        .hand(0, &[grim_feast(), LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let victim = on_battlefield(&engine, owner, target).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, grim_feast());
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![victim],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    engine
}

/// "gain life equal to its toughness": a Wall of Wood (0/3) gives 3, which
/// its power (0) could not.
#[test]
fn grim_feast_gains_toughness_not_power() {
    let engine = feast_then_bolt(&[], &[WALL_OF_WOOD], P1, WALL_OF_WOOD);

    assert!(on_battlefield(&engine, P1, WALL_OF_WOOD).is_none());
    assert_eq!(engine.state().players[0].life, 23);
}

/// "an opponent's graveyard": P0's own Grizzly Bears dying gives P0 nothing.
#[test]
fn grim_feast_gains_nothing_when_your_own_creature_dies() {
    let engine = feast_then_bolt(&[GRIZZLY_BEARS], &[], P0, GRIZZLY_BEARS);

    assert!(on_battlefield(&engine, P0, GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().players[0].life, 20);
}

/// "At the beginning of your upkeep, ... 1 damage to you": nothing happens at
/// the opponent's upkeep (turn 2), and P0 takes 1 at its own (turn 3); the
/// opponent never takes any.
#[test]
fn grim_feast_hurts_only_its_controller_at_their_own_upkeep() {
    let mut engine = feast_then_bolt(&[], &[HILL_GIANT], P1, HILL_GIANT);
    assert_eq!(engine.state().players[0].life, 23);

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(engine.state().players[0].life, 23);
    assert_eq!(engine.state().players[1].life, 20);

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(engine.state().players[0].life, 22);
    assert_eq!(engine.state().players[1].life, 20);
}
