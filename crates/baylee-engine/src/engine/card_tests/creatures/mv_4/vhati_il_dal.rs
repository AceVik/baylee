//! `cards/creatures/mv_4/vhati_il_dal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn vhati_il_dal() -> CardIndex {
    card_index("1759547a-52b3-4bae-817a-658a6fd047e3")
}

/// `Vhati il-Dal` ({2}{B}{G}, 3/3 Legendary Creature — Human Warrior,
/// `Coverage::Implemented`):
/// `{T}: Until end of turn, target creature has base power 1 or base toughness
/// 1.`
#[test]
fn vhati_il_dal_sets_target_creature_base_toughness_to_one_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[vhati_il_dal()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    activate(&mut engine, p0, vhati_il_dal(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    // Mode 1: base toughness 1 until end of turn.
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let ch = engine.state().object(giant).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(1)));
}

#[test]
fn vhati_can_set_power_instead_and_the_effect_ends_at_cleanup() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[vhati_il_dal()], &[HILL_GIANT], &[], &[]);
    let giant = on_battlefield(&e, p1, HILL_GIANT).unwrap();
    activate(&mut e, p0, vhati_il_dal(), 0);
    lk::aim(&mut e, &[giant], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    lk::settle(&mut e);
    let ch = e.state().object(giant).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(1), Some(3)));
    reach_their_main_phase(&mut e, p1);
    let ch = e.state().object(giant).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(3)));
}
