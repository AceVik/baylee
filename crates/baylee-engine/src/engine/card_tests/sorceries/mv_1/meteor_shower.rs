//! `cards/sorceries/mv_1/meteor_shower.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn meteor_shower() -> CardIndex {
    card_index("d2110083-b30f-43f1-9352-7536f5e19d3c")
}

/// Meteor Shower (d2110083-b30f-43f1-9352-7536f5e19d3c) — {X}{X}{R} Sorcery:
/// "Meteor Shower deals X plus 1 damage divided as you choose among any number of targets."
#[test]
fn meteor_shower_deals_x_plus_one_damage_to_its_target_when_cast_with_double_x() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[meteor_shower()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // 5 Mountains -> {4}{R} pays {X}{X}{R} with X = 2, dealing X + 1 = 3 damage.
    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, meteor_shower());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        17,
        "X = 2 deals X + 1 = 3 damage to target player"
    );
}
