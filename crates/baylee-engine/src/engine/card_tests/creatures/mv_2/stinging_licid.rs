//! `cards/creatures/mv_2/stinging_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn stinging_licid() -> CardIndex {
    card_index("c6e48eaf-02f8-43b3-90bf-970d4e412fa6")
}

/// `Stinging Licid` ({1}{U}, 1/1 Creature — Licid, `Coverage::Partial`):
/// activates `{1}{U}, {T}` to deal 2 damage to target creature's controller.
#[test]
fn stinging_licid_deals_two_damage_to_target_creatures_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), stinging_licid()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, stinging_licid(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
}
