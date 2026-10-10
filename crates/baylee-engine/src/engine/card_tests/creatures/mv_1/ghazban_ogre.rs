//! `cards/creatures/mv_1/ghazban_ogre.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ghazban_ogre() -> CardIndex {
    card_index("d361bdd4-afb8-493d-9091-ebe22f215834")
}

fn controller_of(engine: &Engine<RegistryLookup>, id: ObjectId) -> PlayerId {
    engine
        .state()
        .object(id)
        .expect("the Ogre is out")
        .controller
}

fn set_life(engine: &mut Engine<RegistryLookup>, seat: usize, life: i32) {
    engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up")
        .players[seat]
        .life = life;
}

fn stack_len(engine: &Engine<RegistryLookup>) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .len()
}

/// A duel with the Ogre on p0's side and the given life totals, past the
/// mulligans; the game opens in p0's upkeep.
fn ogre_duel(life0: i32, life1: i32) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ghazban_ogre()])
        .life(0, life0)
        .life(1, life1)
        .start();
    keep_mulligans(&mut engine);
    let ogre = on_battlefield(&engine, p0, ghazban_ogre()).expect("the Ogre is out");
    (engine, ogre)
}

/// Ghazbán Ogre: "At the beginning of your upkeep, if a player has more life
/// than each other player, the player with the most life gains control of
/// this creature."
///
/// p0 owns the Ogre and trails, so at p0's upkeep it crosses the table. At
/// p1's own upkeep p0 is ahead (set by the harness), so it comes back.
#[test]
fn ghazban_ogre_goes_to_whoever_leads_at_its_controllers_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, ogre) = ogre_duel(10, 20);
    assert_eq!(stack_len(&engine), 1, "the upkeep trigger is on the stack");
    reach_main_phase(&mut engine, p0);
    assert_eq!(controller_of(&engine, ogre), p1, "the leader took it");

    // p0 now leads; the Ogre's controller (p1) has its upkeep next.
    set_life(&mut engine, 0, 30);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        controller_of(&engine, ogre),
        p0,
        "at p1's upkeep the new leader, p0, gets it back"
    );
}

/// A tie for the most life is nobody leading: it does not trigger and
/// nothing changes hands, at either player's upkeep.
#[test]
fn ghazban_ogre_stays_put_when_the_lives_are_tied() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, ogre) = ogre_duel(20, 20);
    assert_eq!(stack_len(&engine), 0, "no one has more life: no trigger");
    reach_main_phase(&mut engine, p0);
    assert_eq!(controller_of(&engine, ogre), p0);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(controller_of(&engine, ogre), p0);
}

/// If the Ogre's controller is the one with the most life, it stays.
#[test]
fn ghazban_ogre_stays_with_its_controller_when_they_lead() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, ogre) = ogre_duel(25, 20);
    reach_main_phase(&mut engine, p0);
    assert_eq!(controller_of(&engine, ogre), p0, "the controller leads");
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        controller_of(&engine, ogre),
        p0,
        "nobody else's upkeep matters"
    );
}
