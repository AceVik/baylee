//! `cards/instants/mv_1/anoint.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn anoint() -> CardIndex {
    card_index("fcb03c54-5b9a-428d-bbf6-ef8977770c8d")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// p0 has Grizzly Bears, a Hill Giant, a Plains and two Mountains, with Anoint
/// and two Lightning Bolts in hand. p0's first main phase, nothing tapped.
fn anoint_and_two_bolts() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                mountain(),
                mountain(),
                grizzly_bears(),
                hill_giant(),
            ],
        )
        .hand(0, &[anoint(), lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

#[track_caller]
fn bolt(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    cast_with_floating(engine, P0, lightning_bolt());
    aim_at(engine, P0, target);
    pass_until(engine, stack_is_empty);
}

/// Anoint (fcb03c54-5b9a-428d-bbf6-ef8977770c8d) — {W} Instant (Partial: buyback not modeled):
/// "Prevent the next 3 damage that would be dealt to target creature this turn."
#[test]
fn anoint_prevents_the_next_three_damage_to_the_target_creature() {
    let mut engine = anoint_and_two_bolts();
    tap_all_mana(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    cast_with_floating(&mut engine, P0, anoint());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    bolt(&mut engine, bears);

    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_some());
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
}

#[test]
fn anoint_is_used_up_by_the_first_three_damage() {
    let mut engine = anoint_and_two_bolts();
    tap_all_mana(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    cast_with_floating(&mut engine, P0, anoint());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    bolt(&mut engine, bears);
    bolt(&mut engine, bears);

    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_none(),
        "the second Bolt meets no shield"
    );
    assert!(in_graveyard(&engine, P0, grizzly_bears()).is_some());
}

#[test]
fn anoint_protects_only_its_target() {
    let mut engine = anoint_and_two_bolts();
    tap_all_mana(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, P0, hill_giant()).unwrap();

    cast_with_floating(&mut engine, P0, anoint());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    bolt(&mut engine, giant);

    assert!(
        in_graveyard(&engine, P0, hill_giant()).is_some(),
        "the Giant had no shield"
    );
    assert!(on_battlefield(&engine, P0, grizzly_bears()).is_some());
}

#[test]
fn anoint_goes_to_the_graveyard_without_its_unmodeled_buyback() {
    let mut engine = anoint_and_two_bolts();
    tap_all_mana(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    cast_with_floating(&mut engine, P0, anoint());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, anoint()).is_some());
    assert!(in_hand(&engine, P0, anoint()).is_none());
}

/// "This turn": a shield made on p0's turn does not stop a Bolt on p1's.
#[test]
fn anoint_shield_ends_with_the_turn() {
    let mut engine = anoint_and_two_bolts();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    // Only the Plains is spent now: the Mountains stay untapped for p1's turn.
    let plains_id = on_battlefield(&engine, P0, plains()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == plains_id);
    cast_with_floating(&mut engine, P0, anoint());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, P1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    tap_all_mana(&mut engine, P0);
    bolt(&mut engine, bears);

    assert!(
        in_graveyard(&engine, P0, grizzly_bears()).is_some(),
        "the next turn's Bolt is not prevented"
    );
}
