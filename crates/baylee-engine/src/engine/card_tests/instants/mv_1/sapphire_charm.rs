//! `cards/instants/mv_1/sapphire_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sapphire_charm() -> CardIndex {
    card_index("2ea2bf6c-a427-4d0c-b447-159959327697")
}

/// Sapphire Charm (`{U}`, `Coverage::Partial`): modal instant whose three modes
/// schedule a delayed next-upkeep draw, grant target creature flying until end
/// of turn, or phase out target creature an opponent controls.
#[test]
fn sapphire_charm_modes_grant_flying_and_phase_out_opposing_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[sapphire_charm(), sapphire_charm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    // Mode 1: target creature gains flying until end of turn.
    let my_islands = lands_of(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| id == my_islands[0]);
    cast_with_floating(&mut engine, p0, sapphire_charm());
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bear).contains(KeywordSet::FLYING));

    // Mode 2: target creature an opponent controls phases out.
    tap_mana_where(&mut engine, p0, |id| id == my_islands[1]);
    cast_with_floating(&mut engine, p0, sapphire_charm());
    engine.apply(p0, PlayerAction::ChooseMode(2)).unwrap();
    let menu = aim_at(&mut engine, p0, ogre);
    assert_eq!(menu, vec![ogre], "only opponent's creature is offered");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(ogre)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
