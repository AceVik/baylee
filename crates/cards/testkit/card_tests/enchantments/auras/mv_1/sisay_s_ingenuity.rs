//! `cards/enchantments/auras/mv_1/sisay_s_ingenuity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn sisay_s_ingenuity() -> CardIndex {
    card_index("3a487f20-6b1d-4be3-b51f-0c8f0a611601")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn choose_target_color(e: &mut Engine<RegistryLookup>, color: Color) {
    let Pending::ChooseCastMode {
        player, options, ..
    } = e.pending().clone()
    else {
        panic!("expected ChooseCastMode for color, got {:?}", e.pending());
    };
    let target = ColorSet::of(color);
    let choice = options
        .iter()
        .find(|o| matches!(o.kind, CastModeKind::Colors(c) if c == target))
        .expect("color option offered");
    e.apply(
        player,
        PlayerAction::ChooseMode(usize::try_from(choice.index).unwrap()),
    )
    .unwrap();
}

/// ETB draws a card for the Aura's controller and enchanted creature gains
/// "{2}{U}: Target creature becomes the color of your choice until end of turn."
#[test]
fn sisay_s_ingenuity_draws_on_etb_and_grants_color_change_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), plains()],
        &[savannah_lions()],
        &[sisay_s_ingenuity()],
        &[],
    );
    let bear = lk::obj(&e, p0, grizzly_bears());
    let lions = lk::obj(&e, p1, savannah_lions());
    let before_hand = e.state().zones.list(ZoneLocation::Hand(p0)).len();

    lk::float(&mut e, p0, &[(ManaColor::Blue, 1)]);
    lk::cast_at(&mut e, p0, sisay_s_ingenuity(), &[bear], &[]);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before_hand
    );
    assert_eq!(
        e.state().object(lions).unwrap().characteristics().colors,
        ColorSet::of(Color::White)
    );

    lk::float(&mut e, p0, &[(ManaColor::Blue, 3)]);
    activate(
        &mut e,
        p0,
        grizzly_bears(),
        baylee_engine::choice::GRANTED_ABILITY,
    );
    lk::aim(&mut e, &[lions], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    choose_target_color(&mut e, Color::Black);
    lk::settle(&mut e);
    assert_eq!(
        e.state().object(lions).unwrap().characteristics().colors,
        ColorSet::of(Color::Black)
    );

    reach_their_main_phase(&mut e, p1);
    assert_eq!(
        e.state().object(lions).unwrap().characteristics().colors,
        ColorSet::of(Color::White)
    );
}

/// Enchanting an opponent's creature draws for you on ETB while granting the
/// color-changing ability to the enchanted creature's controller.
#[test]
fn sisay_s_ingenuity_on_opponents_creature_grants_ability_to_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[], &[savannah_lions()], &[sisay_s_ingenuity()], &[]);
    let lions = lk::obj(&e, p1, savannah_lions());
    lk::float(&mut e, p0, &[(ManaColor::Blue, 1)]);
    lk::cast_at(&mut e, p0, sisay_s_ingenuity(), &[lions], &[]);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);

    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Blue, 3)]);
    assert!(lk::offered(
        &e,
        savannah_lions(),
        p1,
        baylee_engine::choice::GRANTED_ABILITY
    ));
}
