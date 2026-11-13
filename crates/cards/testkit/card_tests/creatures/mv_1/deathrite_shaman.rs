//! `cards/creatures/mv_1/deathrite_shaman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

/// The land and instant/sorcery abilities have their full graveyard-targeting
/// and resolution tests beside Clone. This plays the remaining green ability.
#[test]
fn deathrite_shaman_exiles_a_creature_and_gains_two_life() {
    let mut e = Duel::new(SEED, grizzly_bears())
        .battlefield(0, &[deathrite_shaman(), forest()])
        .life(0, 10)
        .start();
    keep_mulligans(&mut e);
    seed_graveyard(&mut e, P0, 1);
    reach_main_phase(&mut e, P0);
    let bears = in_graveyard(&e, P0, grizzly_bears()).unwrap();
    tap_all_mana_but(&mut e, P0, Some(deathrite_shaman()));

    activate(&mut e, P0, deathrite_shaman(), 2);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected a creature-card target, got {:?}", e.pending())
    };
    assert_eq!(options, vec![bears]);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![bears],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, P0, grizzly_bears()).is_none());
    assert!(
        e.state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .contains(&bears)
    );
    assert_eq!(e.state().players[0].life, 12);
}
