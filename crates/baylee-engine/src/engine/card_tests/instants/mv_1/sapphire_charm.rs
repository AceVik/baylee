//! `cards/instants/mv_1/sapphire_charm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

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

/// Mode 1, "target player draws a card at the beginning of the next turn's
/// upkeep": the controller draws at the opponent's upkeep, the opponent only
/// its normal card. (The draw always goes to the caster, whoever is named.)
#[test]
fn sapphire_charm_draw_mode_gives_the_caster_a_card_at_the_next_upkeep() {
    // A creature on each side keeps all three modes legal, so "choose one" is
    // asked (with a single legal mode the engine would not ask at all).
    let mut engine = lk::game(
        SEED,
        &[grizzly_bears()],
        &[gray_ogre()],
        &[sapphire_charm()],
        &[],
    );

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, sapphire_charm());
    engine.apply(P0, PlayerAction::ChooseMode(0)).unwrap();
    engine.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let p0_hand = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    let p1_hand = engine.state().zones.list(ZoneLocation::Hand(P1)).len();

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        p0_hand + 1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        p1_hand + 1,
        "the opponent draws only its own draw-step card"
    );
}

/// Mode 2, "until end of turn": the flying is gone in the next turn.
#[test]
fn sapphire_charm_flying_ends_with_the_turn() {
    let mut engine = lk::game(SEED, &[grizzly_bears()], &[], &[sapphire_charm()], &[]);
    let bear = lk::obj(&engine, P0, grizzly_bears());

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, sapphire_charm());
    engine.apply(P0, PlayerAction::ChooseMode(1)).unwrap();
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bear).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, P1);
    assert!(!keywords(&engine, bear).contains(KeywordSet::FLYING));
}

/// Mode 3: "phases in before its controller untaps": the opponent's creature
/// is back in its controller's own turn.
#[test]
fn sapphire_charm_phase_out_ends_at_the_opponents_untap_step() {
    let mut engine = lk::game(SEED, &[], &[gray_ogre()], &[sapphire_charm()], &[]);
    let ogre = lk::obj(&engine, P1, gray_ogre());

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, sapphire_charm());
    engine.apply(P0, PlayerAction::ChooseMode(2)).unwrap();
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(ogre)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !engine
            .state()
            .object(ogre)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
