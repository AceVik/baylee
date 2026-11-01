//! Withering Wisps — `{1}{B}{B}` Enchantment (Ice Age)
//!
//! At the beginning of the end step, if no creatures are on the battlefield,
//! sacrifice Withering Wisps.
//! `{B}`: Withering Wisps deals 1 damage to each creature and each player.
//! Activate no more times each turn than the number of snow Swamps you
//! control.
//! (`Coverage::Partial`: the cap is one activation per turn while a snow Swamp
//! is controlled, which is the printed cap for exactly one snow Swamp.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn withering_wisps() -> CardIndex {
    card_index("ce65c6ed-e5d4-48d8-b569-4e2774b2912c")
}

fn snow_covered_swamp() -> CardIndex {
    card_index("d8239a86-7184-4005-ba1e-2dddcd756c47")
}

fn tundra_wolves() -> CardIndex {
    card_index("2155864e-5788-4e72-a800-e2cf25cf59a7")
}

#[test]
fn withering_wisps_requires_snow_swamp_to_activate_and_sacrifices_at_end_step_only_when_no_creatures_remain()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[withering_wisps(), snow_covered_swamp()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[tundra_wolves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wisps_hand = in_hand(&engine, p0, withering_wisps()).unwrap();
    let wolves = on_battlefield(&engine, p1, tundra_wolves()).unwrap();

    // Cast Withering Wisps ({1}{B}{B}) with 1 {B} left floating.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: wisps_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let wisps = on_battlefield(&engine, p0, withering_wisps()).unwrap();

    // Without a snow Swamp, {B} ability (index 1) is not offered even with {B} floating.
    assert!(
        !ability_offered(&engine, wisps, 1),
        "Withering Wisps cannot be activated without a snow Swamp"
    );

    // At p0's end step, Tundra Wolves is on the battlefield, so Withering Wisps is NOT sacrificed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().object(wisps).unwrap().zone,
        Zone::Battlefield,
        "Withering Wisps stays on the battlefield when a creature is present at end step"
    );

    // On p0's next turn, play Snow-Covered Swamp and activate Withering Wisps to kill Tundra Wolves.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    let snow_swamp = in_hand(&engine, p0, snow_covered_swamp()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: snow_swamp })
        .unwrap();
    tap_all_mana(&mut engine, p0);
    assert!(
        ability_offered(&engine, wisps, 1),
        "controlling a snow Swamp enables Withering Wisps' ability"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wisps,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(wolves).unwrap().zone,
        Zone::Graveyard,
        "1/1 Tundra Wolves died to Withering Wisps' 1 damage"
    );
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);

    // One snow Swamp, one activation this turn: with the mana of four more
    // Swamps still floating, the ability is not offered a second time.
    assert!(
        !ability_offered(&engine, wisps, 1),
        "Withering Wisps' {{B}} ability can be activated no more times each turn than snow Swamps controlled (one)"
    );

    // Advance to the end step with no creatures on the battlefield: Withering Wisps triggers and sacrifices itself.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(
        engine.state().object(wisps).unwrap().zone,
        Zone::Graveyard,
        "Withering Wisps sacrificed itself at the end step when no creatures were on the battlefield"
    );
}
