//! `cards/creatures/mv_2/chevill_bane_of_monsters.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_cards_dsl::counters;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chevill_bane_of_monsters() -> CardIndex {
    card_index("50863075-a30b-4238-8e31-5af32a39d886")
}

fn chevill_game_before_upkeep(removal: CardIndex) -> Engine<RegistryLookup> {
    Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                chevill_bane_of_monsters(),
                grizzly_bears(),
                mountain(),
                plains(),
            ],
        )
        .battlefield(
            1,
            &[
                grizzly_bears(),
                llanowar_elves(),
                karn_the_great_creator(),
                forest(),
            ],
        )
        .hand(0, &[removal])
        .start()
}

fn chevill_game(removal: CardIndex) -> Engine<RegistryLookup> {
    let mut engine = chevill_game_before_upkeep(removal);
    keep_mulligans(&mut engine);
    engine
}

/// A counter seeded while the mulligans are still open is present when the
/// first upkeep trigger is collected. Seeding after the last keep would be
/// too late: the target question is already open by then.
fn seed_bounty(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> ObjectId {
    let object = on_battlefield(engine, seat, card).expect("the counter's permanent is present");
    let state = engine
        .dev_state_mut(P0)
        .expect("the harness may seed a counter");
    state
        .object_mut(object)
        .unwrap()
        .counters
        .add(counters::BOUNTY, 1);
    state.invalidate_projections();
    object
}

#[track_caller]
fn mark(e: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    let marked = on_battlefield(e, P1, card).expect("the opposing target is present");
    let options = pass_until_targets(e, P0);
    let own = on_battlefield(e, P0, chevill_bane_of_monsters()).expect("Chevill");
    let land = on_battlefield(e, P1, forest()).expect("the opposing Forest");
    assert!(options.contains(&marked), "{options:?}");
    assert!(
        !options.contains(&own) && !options.contains(&land),
        "{options:?}"
    );
    aim_at(e, P0, marked);
    pass_until(e, stack_is_empty);
    assert_eq!(counters_on(e, marked, counters::BOUNTY), 1);
    marked
}

/// The first upkeep targets only an opposing creature or planeswalker. Its
/// bounty counter is the one whose permanent dying later gains three and draws.
#[test]
fn chevill_marks_an_opposing_permanent_and_rewards_its_death() {
    let mut e = chevill_game(lightning_bolt());
    let bear = on_battlefield(&e, P1, grizzly_bears()).unwrap();
    let elf = on_battlefield(&e, P1, llanowar_elves()).unwrap();
    let karn = on_battlefield(&e, P1, karn_the_great_creator()).unwrap();
    let options = pass_until_targets(&mut e, P0);
    assert!(options.contains(&bear) && options.contains(&elf) && options.contains(&karn));
    aim_at(&mut e, P0, bear);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, bear, counters::BOUNTY), 1);
    assert_eq!(counters_on(&e, elf, counters::BOUNTY), 0);
    let chevill = on_battlefield(&e, P0, chevill_bane_of_monsters()).unwrap();
    assert!(keywords(&e, chevill).contains(KeywordSet::DEATHTOUCH));

    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    cast_from_hand(&mut e, P0, lightning_bolt());
    aim_at(&mut e, P0, bear);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P1, grizzly_bears()).is_some());
    assert_eq!(e.state().players[0].life, 23, "exactly one death trigger");
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library - 1
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), 1);
}

/// An opposing bounty counter suppresses the intervening-if on the next
/// upkeep too; the first counter does not disappear when its trigger resolves.
#[test]
fn chevill_does_not_mark_while_an_opponent_already_has_a_bounty_counter() {
    let mut e = chevill_game_before_upkeep(index::LIGHTNING_BOLT);
    let bear = seed_bounty(&mut e, P1, grizzly_bears());
    let elf = on_battlefield(&e, P1, llanowar_elves()).unwrap();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    assert_eq!(counters_on(&e, bear, counters::BOUNTY), 1);
    assert_eq!(counters_on(&e, elf, counters::BOUNTY), 0);
}

/// The sentence does not say "no permanents": your own bounty counter is
/// irrelevant to the condition, and does not receive another from Chevill.
#[test]
fn chevill_ignores_a_bounty_counter_on_your_own_creature() {
    let mut e = chevill_game_before_upkeep(index::LIGHTNING_BOLT);
    let own = seed_bounty(&mut e, P0, grizzly_bears());
    keep_mulligans(&mut e);
    let elf = mark(&mut e, llanowar_elves());
    assert_eq!(counters_on(&e, own, counters::BOUNTY), 1);
    assert_eq!(counters_on(&e, elf, counters::BOUNTY), 1);
}

/// Marking one creature does not reward the death of a different, unmarked
/// opposing creature.
#[test]
fn chevill_does_not_reward_an_unmarked_death() {
    let mut e = chevill_game(lightning_bolt());
    let elf = mark(&mut e, llanowar_elves());
    let bear = on_battlefield(&e, P1, grizzly_bears()).unwrap();
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    cast_from_hand(&mut e, P0, lightning_bolt());
    aim_at(&mut e, P0, bear);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P1, grizzly_bears()).is_some());
    assert_eq!(e.state().players[0].life, 20);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), 0);
    assert!(e.state().object(elf).is_some());
}

/// Exile is not death even when the departing permanent had a bounty counter.
#[test]
fn chevill_does_not_reward_exiling_a_marked_creature() {
    let mut e = chevill_game(swords_to_plowshares());
    let bear = mark(&mut e, grizzly_bears());
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    cast_from_hand(&mut e, P0, swords_to_plowshares());
    aim_at(&mut e, P0, bear);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(bear).unwrap().zone, Zone::Exile);
    assert_eq!(e.state().players[0].life, 20);
    assert_eq!(e.state().players[1].life, 22);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), 0);
}
