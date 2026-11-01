//! `cards/creatures/mv_1/soldier_of_fortune.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldier_of_fortune() -> CardIndex {
    card_index("8b2367b3-3a8d-442f-8d58-6e4c03fa5a46")
}

/// How many times `who`'s library has been shuffled so far. The game's own
/// setup may shuffle, so a test compares counts before and after.
fn shuffles(engine: &Engine<RegistryLookup>, who: PlayerId) -> usize {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|entry| {
            matches!(
                entry.event,
                crate::event::GameEvent::Shuffled { player, .. } if player == who
            )
        })
        .count()
}

/// Activates Soldier of Fortune at `target` and lets it resolve.
fn shuffle_at(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    let p0 = PlayerId::new(0);
    tap_all_mana_but(engine, p0, Some(soldier_of_fortune()));
    activate(engine, p0, soldier_of_fortune(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// Soldier of Fortune ({R} 1/1 Creature — Human Mercenary):
/// - "{R}, {T}: Target player shuffles their library."
#[test]
fn soldier_of_fortune_taps_to_shuffle_target_player_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[soldier_of_fortune(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let soldier = on_battlefield(&engine, p0, soldier_of_fortune()).unwrap();
    let (mine_before, theirs_before) = (shuffles(&engine, p0), shuffles(&engine, p1));

    shuffle_at(&mut engine, p1);

    assert!(is_tapped(&engine, soldier));
    assert_eq!(
        shuffles(&engine, p1),
        theirs_before + 1,
        "target player's library must be shuffled"
    );
    assert_eq!(
        shuffles(&engine, p0),
        mine_before,
        "the activator's own library is not"
    );
}

/// "Target player" may be the activator.
#[test]
fn soldier_of_fortune_can_shuffle_the_activators_own_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[soldier_of_fortune(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let (mine_before, theirs_before) = (shuffles(&engine, p0), shuffles(&engine, p1));

    shuffle_at(&mut engine, p0);

    assert_eq!(shuffles(&engine, p0), mine_before + 1);
    assert_eq!(shuffles(&engine, p1), theirs_before);
}
