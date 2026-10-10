//! `cards/lands/towns/balamb_garden_see_d_academy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The land enters tapped and offers green or blue mana after untapping.
#[test]
fn balamb_garden_enters_tapped_and_taps_for_green_or_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(222, forest())
        .hand(0, &[balamb_garden_see_d_academy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, balamb_garden_see_d_academy());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, balamb_garden_see_d_academy(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn balamb_discounts_other_own_towns_transforms_then_crews_and_draws_on_attack() {
    use crate::engine::card_tests::legends_kit as kit;
    let garden = balamb_garden_see_d_academy();
    let mut e = kit::game(
        977,
        &[garden, garden, garden, llanowar_elves()],
        &[garden],
        &[],
        &[],
    );
    let vehicle = kit::obj(&e, kit::P0, garden);
    let elf = kit::obj(&e, kit::P0, llanowar_elves());
    let original = identity(&e, vehicle);
    kit::float(
        &mut e,
        kit::P0,
        &[
            (ManaColor::Green, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
        ],
    );
    assert!(!priority_offer(&e).abilities.contains(&(vehicle, 1)));
    kit::float(&mut e, kit::P0, &[(ManaColor::Colorless, 1)]);
    e.apply(
        kit::P0,
        PlayerAction::ActivateAbility {
            source: vehicle,
            ability_index: 1,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(identity(&e, vehicle), original);
    assert_eq!(kit::pool_total(&e, kit::P0), 0);
    assert!(is_tapped(&e, vehicle));
    assert_eq!(types(&e, vehicle), TypeSet::ARTIFACT);
    reach_their_main_phase(&mut e, kit::P1);
    reach_their_main_phase(&mut e, kit::P0);
    e.apply(
        kit::P0,
        PlayerAction::ActivateAbility {
            source: vehicle,
            ability_index: 1,
        },
    )
    .unwrap();
    e.apply(kit::P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(types(&e, vehicle).contains(TypeSet::CREATURE));
    assert_eq!(pt(&e, vehicle), (5, 4));
    let before = library_size(&e, kit::P0);
    kit::attack(&mut e, &[vehicle]);
    assert_eq!(library_size(&e, kit::P0), before - 1);
}
