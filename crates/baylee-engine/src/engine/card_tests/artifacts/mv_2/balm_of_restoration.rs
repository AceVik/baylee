//! `cards/artifacts/mv_2/balm_of_restoration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn balm_of_restoration() -> CardIndex {
    card_index("45a40b4c-1d37-4d87-adf1-abf0ea4c86b4")
}

fn bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

/// Answers the pending target question with the given objects and players.
#[track_caller]
fn aim(engine: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let seat = engine
        .pending()
        .asked()
        .expect("a target question is pending");
    engine
        .apply(seat, PlayerAction::ChooseTargets { objects, players })
        .expect("the targets were on the offer");
}

/// Pays the Balm's {1} with a Forest and nothing else, activates it in `mode`.
#[track_caller]
fn activate_balm(engine: &mut Engine<RegistryLookup>, mode: usize) {
    let p0 = PlayerId::new(0);
    let forest_id = on_battlefield(engine, p0, forest()).expect("a Forest");
    assert_eq!(tap_mana_where(engine, p0, |id| id == forest_id), 1);
    activate(engine, p0, balm_of_restoration(), 0);
    engine
        .apply(p0, PlayerAction::ChooseMode(mode))
        .expect("the mode was on the offer");
}

/// Casts a Bolt from hand at the given target, paying with the Mountains.
#[track_caller]
fn bolt_it(engine: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p0 = PlayerId::new(0);
    let mountain_id = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == p0
                    && o.card.map(|c| c.index) == Some(mountain())
                    && !o.status.contains(crate::object::Status::TAPPED)
            })
        })
        .expect("an untapped Mountain");
    assert_eq!(tap_mana_where(engine, p0, |id| id == mountain_id), 1);
    cast_with_floating(engine, p0, bolt());
    aim(engine, objects, players);
    pass_until(engine, stack_is_empty);
}

fn a_board() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                balm_of_restoration(),
                forest(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[bolt(), bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    engine
}

/// Balm of Restoration (45a40b4c-1d37-4d87-adf1-abf0ea4c86b4) — "{1}, {T},
/// Sacrifice this artifact: Choose one — • You gain 2 life. • Prevent the next
/// 2 damage that would be dealt to any target this turn."
#[test]
fn balm_of_restoration_first_mode_gains_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = a_board();
    activate_balm(&mut engine, 0);
    assert!(
        on_battlefield(&engine, p0, balm_of_restoration()).is_none(),
        "tap and sacrifice are paid up front"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 22);
    assert!(in_graveyard(&engine, p0, balm_of_restoration()).is_some());
}

/// Second mode at a player: the next two points of damage to them are
/// prevented, no more. The first Bolt does 3 - 2 = 1, the second is whole.
#[test]
fn balm_of_restoration_second_mode_prevents_the_next_two_damage_to_a_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_board();
    activate_balm(&mut engine, 1);
    aim(&mut engine, vec![], vec![p1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 20, "this mode gains nothing");

    bolt_it(&mut engine, vec![], vec![p1]);
    assert_eq!(life(&engine, p1), 19, "3 damage, 2 prevented");
    bolt_it(&mut engine, vec![], vec![p1]);
    assert_eq!(
        life(&engine, p1),
        16,
        "the shield is spent: the next Bolt is whole"
    );
}

/// "Any target": a creature will do. Its controller's Bolt would kill the
/// Bears (2 toughness); 3 - 2 = 1 damage leaves them standing.
#[test]
fn balm_of_restoration_second_mode_protects_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = a_board();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears");
    activate_balm(&mut engine, 1);
    aim(&mut engine, vec![bears], vec![]);
    pass_until(&mut engine, stack_is_empty);

    bolt_it(&mut engine, vec![bears], vec![]);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "1 damage on a 2/2 does not kill it"
    );
    assert_eq!(life(&engine, p0), 20);
}
