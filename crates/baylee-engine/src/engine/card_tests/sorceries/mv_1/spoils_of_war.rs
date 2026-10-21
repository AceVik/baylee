//! `cards/sorceries/mv_1/spoils_of_war.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn spoils_of_war() -> CardIndex {
    card_index("f3240a86-28b6-4338-9a59-1fb6f795d713")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Spoils of War (f3240a86-28b6-4338-9a59-1fb6f795d713) — {X}{B} Sorcery:
/// (Coverage::Partial — puts X +1/+1 counters on up to one target creature.)
#[test]
fn spoils_of_war_places_x_plus_one_plus_one_counters_on_target_creature() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), grizzly_bears()])
        .hand(0, &[spoils_of_war()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, spoils_of_war());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears),
        (5, 5),
        "Grizzly Bears received 3 +1/+1 counters from Spoils of War"
    );
}
