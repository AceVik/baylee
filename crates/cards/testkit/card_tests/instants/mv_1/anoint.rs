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

/// p0 has Grizzly Bears, four Plains and a Mountain, with Anoint and a
/// Lightning Bolt in hand; p1 has two Islands and a Counterspell. p0's first
/// main phase, the Plains tapped and the Mountain kept for the Bolt.
fn anoint_with_four_floating() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[island(), island()])
        .hand(0, &[anoint(), lightning_bolt()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let white = all_on_battlefield(&engine, P0, plains());
    tap_mana_where(&mut engine, P0, |id| white.contains(&id));
    engine
}

#[track_caller]
fn bolt(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    cast_with_floating(engine, P0, lightning_bolt());
    aim_at(engine, P0, target);
    pass_until(engine, stack_is_empty);
}

/// Anoint (fcb03c54-5b9a-428d-bbf6-ef8977770c8d) — {W} Instant:
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

/// "This turn": a shield made on p0's turn does not stop a Bolt on p1's.
#[test]
fn anoint_shield_ends_with_the_turn() {
    let mut engine = anoint_and_two_bolts();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    // Only the Plains is spent now: the Mountains stay untapped for p1's turn.
    let plains_id = on_battlefield(&engine, P0, plains()).unwrap();
    tap_mana_where(&mut engine, P0, |id| id == plains_id);
    cast_with_floating(&mut engine, P0, anoint());
    answer_buyback(&mut engine, P0, false);
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

/// "Buyback {3}": the {3} is paid on top, the shield still stands, and the
/// card comes home.
#[test]
fn anoint_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = anoint_with_four_floating();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    cast_with_floating(&mut engine, P0, anoint());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{W}} and {{3}} are the four floating"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_bought_back(&engine, P0, anoint());

    tap_all_mana(&mut engine, P0);
    bolt(&mut engine, bears);
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
}

/// Declined, it costs three less and is an Anoint that is spent.
#[test]
fn anoint_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = anoint_with_four_floating();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    cast_with_floating(&mut engine, P0, anoint());
    answer_buyback(&mut engine, P0, false);
    aim_at(&mut engine, P0, bears);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    pass_until(&mut engine, stack_is_empty);
    assert_not_bought_back(&engine, P0, anoint());

    tap_all_mana(&mut engine, P0);
    bolt(&mut engine, bears);
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
}

/// Countered, it does not resolve: buyback paid, card gone, no shield.
#[test]
fn anoint_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = anoint_with_four_floating();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    cast_with_floating(&mut engine, P0, anoint());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, anoint());
    tap_all_mana(&mut engine, P0);
    bolt(&mut engine, bears);
    assert!(
        in_graveyard(&engine, P0, grizzly_bears()).is_some(),
        "no shield was made"
    );
}

/// With its one target gone it does not resolve either.
#[test]
fn anoint_that_loses_its_target_goes_to_the_graveyard() {
    let mut engine = anoint_with_four_floating();
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    cast_with_floating(&mut engine, P0, anoint());
    answer_buyback(&mut engine, P0, true);
    aim_at(&mut engine, P0, bears);
    bury(&mut engine, &[bears]);
    pass_until(&mut engine, stack_is_empty);

    assert_not_bought_back(&engine, P0, anoint());
}
