//! Sunglasses of Urza: payment permission, not mana conversion.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const GLASSES: CardIndex = index::SUNGLASSES_OF_URZA;

fn setup(board: &[CardIndex], hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = Duel::new(1140, forest())
        .battlefield(0, board)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn add_mana(e: &mut Engine<RegistryLookup>, player: PlayerId, color: ManaColor, n: u16) {
    e.dev_state_mut(player).unwrap().players[usize::from(player.get())]
        .mana_pool
        .add(color, n);
    e.refresh_offer();
}

fn offered(e: &Engine<RegistryLookup>, player: PlayerId, card: CardIndex) -> bool {
    let card = in_hand(e, player, card).unwrap();
    priority_offer(e).castable.contains(&card)
}

fn target_player(e: &mut Engine<RegistryLookup>, player: PlayerId, target: PlayerId) {
    e.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![target],
        },
    )
    .unwrap();
}

#[test]
fn sunglasses_of_urza_lets_each_controller_cast_a_red_spell_with_white_mana() {
    for seat in 0..2 {
        let player = PlayerId::new(seat as u8);
        let other = PlayerId::new(1 - seat as u8);
        let mut e = Duel::new(1141, forest())
            .battlefield(seat, &[GLASSES, plains()])
            .hand(seat, &[index::LIGHTNING_BOLT])
            .start();
        keep_mulligans(&mut e);
        reach_their_main_phase(&mut e, player);
        tap_all_mana(&mut e, player);
        assert!(offered(&e, player, index::LIGHTNING_BOLT));
        let life = life_of(&e, other);
        cast_with_floating(&mut e, player, index::LIGHTNING_BOLT);
        target_player(&mut e, player, other);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(life_of(&e, other), life - 3);
        assert_eq!(e.state().players[seat].mana_pool.total(), 0);
    }
}

#[test]
fn sunglasses_of_urza_does_not_produce_mana_or_enable_other_colors() {
    for color in [
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Green,
        ManaColor::Colorless,
    ] {
        let mut e = setup(&[GLASSES], &[index::LIGHTNING_BOLT]);
        assert!(!offered(&e, P0, index::LIGHTNING_BOLT));
        add_mana(&mut e, P0, color, 1);
        assert!(!offered(&e, P0, index::LIGHTNING_BOLT), "{color:?}");
    }
}

#[test]
fn sunglasses_of_urza_does_not_allow_red_mana_to_pay_white_or_green_costs() {
    for (mana, card) in [
        (ManaColor::Red, index::SAVANNAH_LIONS),
        (ManaColor::White, index::LLANOWAR_ELVES),
    ] {
        let mut e = setup(&[GLASSES], &[card]);
        add_mana(&mut e, P0, mana, 1);
        assert!(!offered(&e, P0, card));
    }
}

#[test]
fn sunglasses_of_urza_does_not_help_an_opponent() {
    let mut e = Duel::new(1142, forest())
        .battlefield(1, &[GLASSES])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    add_mana(&mut e, P0, ManaColor::White, 1);
    assert!(!offered(&e, P0, index::LIGHTNING_BOLT));
}

#[test]
fn sunglasses_of_urza_only_functions_on_the_battlefield() {
    for destination in [
        ZoneLocation::Hand(P0),
        ZoneLocation::Graveyard(P0),
        ZoneLocation::Exile(P0),
    ] {
        let mut e = setup(&[GLASSES], &[index::LIGHTNING_BOLT]);
        let glasses = on_battlefield(&e, P0, GLASSES).unwrap();
        add_mana(&mut e, P0, ManaColor::White, 1);
        assert!(offered(&e, P0, index::LIGHTNING_BOLT));
        e.dev_state_mut(P0)
            .unwrap()
            .move_object(
                glasses,
                destination,
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        e.sync_static_effects();
        e.refresh_offer();
        assert!(!offered(&e, P0, index::LIGHTNING_BOLT));
        assert_eq!(
            e.state().players[0].mana_pool.total(),
            1,
            "the white mana was never converted"
        );
    }
}

#[test]
fn sunglasses_of_urza_phasing_suspends_and_restores_its_permission() {
    let mut e = setup(&[GLASSES], &[index::LIGHTNING_BOLT]);
    let glasses = on_battlefield(&e, P0, GLASSES).unwrap();
    add_mana(&mut e, P0, ManaColor::White, 1);
    e.dev_state_mut(P0).unwrap().phase_out(&[glasses]);
    e.sync_static_effects();
    e.refresh_offer();
    assert!(!offered(&e, P0, index::LIGHTNING_BOLT));
    e.dev_state_mut(P0).unwrap().phase_in(glasses);
    e.sync_static_effects();
    e.refresh_offer();
    assert!(offered(&e, P0, index::LIGHTNING_BOLT));
}

#[test]
fn sunglasses_of_urza_white_mana_still_pays_white_costs() {
    let mut e = setup(&[GLASSES], &[index::SAVANNAH_LIONS]);
    add_mana(&mut e, P0, ManaColor::White, 1);
    assert!(offered(&e, P0, index::SAVANNAH_LIONS));
    cast_with_floating(&mut e, P0, index::SAVANNAH_LIONS);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::SAVANNAH_LIONS).is_some());
}

#[test]
fn sunglasses_of_urza_reserves_white_for_a_mixed_red_white_cost() {
    for (white, red) in [(1, 1), (2, 0)] {
        let mut e = setup(&[GLASSES], &[index::THORIN_OAKENSHIELD]);
        add_mana(&mut e, P0, ManaColor::White, white);
        add_mana(&mut e, P0, ManaColor::Red, red);
        assert!(offered(&e, P0, index::THORIN_OAKENSHIELD));
        cast_with_floating(&mut e, P0, index::THORIN_OAKENSHIELD);
        pass_until(&mut e, stack_is_empty);
        let thorin = on_battlefield(&e, P0, index::THORIN_OAKENSHIELD).unwrap();
        assert_eq!(pt(&e, thorin), (3, 2));
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
    }
}

#[test]
fn sunglasses_of_urza_spending_white_is_recorded_as_white_not_red() {
    let mut e = setup(&[GLASSES], &[index::LIGHTNING_BOLT]);
    add_mana(&mut e, P0, ManaColor::White, 1);
    let bolt = in_hand(&e, P0, index::LIGHTNING_BOLT).unwrap();
    cast_with_floating(&mut e, P0, index::LIGHTNING_BOLT);
    target_player(&mut e, P0, P1);
    let paid = e.state().object(bolt).unwrap().paid.as_ref().unwrap();
    assert_eq!(paid.mana_spent, 1);
    assert_eq!(paid.colors_spent, ColorSet::from_slice(&[Color::White]));
    assert_eq!(
        e.state().object(bolt).unwrap().characteristics().colors,
        ColorSet::from_slice(&[Color::Red])
    );
}

#[test]
fn sunglasses_of_urza_pays_red_activated_abilities() {
    let mut e = setup(&[GLASSES, index::SHIVAN_DRAGON], &[]);
    let dragon = on_battlefield(&e, P0, index::SHIVAN_DRAGON).unwrap();
    assert!(!priority_offer(&e).abilities.contains(&(dragon, 0)));
    add_mana(&mut e, P0, ManaColor::White, 1);
    assert!(priority_offer(&e).abilities.contains(&(dragon, 0)));
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: dragon,
            ability_index: 0,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, dragon), (6, 5));
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn sunglasses_of_urza_pays_x_without_spending_a_mana_unit_twice() {
    let mut e = setup(&[GLASSES], &[index::EARTHQUAKE]);
    add_mana(&mut e, P0, ManaColor::White, 3);
    let before = [life_of(&e, P0), life_of(&e, P1)];
    cast_with_floating(&mut e, P0, index::EARTHQUAKE);
    e.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life_of(&e, P0), before[0] - 2);
    assert_eq!(life_of(&e, P1), before[1] - 2);
}

#[test]
fn sunglasses_of_urza_multiple_copies_do_not_multiply_available_mana() {
    let mut e = setup(&[GLASSES, GLASSES], &[index::THORIN_OAKENSHIELD]);
    add_mana(&mut e, P0, ManaColor::White, 1);
    assert!(!offered(&e, P0, index::THORIN_OAKENSHIELD));
    add_mana(&mut e, P0, ManaColor::White, 1);
    assert!(offered(&e, P0, index::THORIN_OAKENSHIELD));
}

#[test]
fn sunglasses_of_urza_follows_control_after_a_real_steal_artifact() {
    let mut e = Duel::new(1143, forest())
        .battlefield(1, &[GLASSES])
        .hand(0, &[index::STEAL_ARTIFACT, index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let glasses = on_battlefield(&e, P1, GLASSES).unwrap();
    add_mana(&mut e, P0, ManaColor::White, 1);
    assert!(!offered(&e, P0, index::LIGHTNING_BOLT));
    add_mana(&mut e, P0, ManaColor::Blue, 4);
    cast_with_floating(&mut e, P0, index::STEAL_ARTIFACT);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![glasses],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(glasses).unwrap().owner, P1);
    assert_eq!(e.state().object(glasses).unwrap().controller, P0);
    // Generic payment may have used the earlier white unit; add a fresh one.
    add_mana(&mut e, P0, ManaColor::White, 1);
    assert!(offered(&e, P0, index::LIGHTNING_BOLT));
    cast_with_floating(&mut e, P0, index::LIGHTNING_BOLT);
    let life = life_of(&e, P1);
    target_player(&mut e, P0, P1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(life_of(&e, P1), life - 3);
}

#[test]
fn sunglasses_of_urza_losing_its_ability_removes_the_payment_permission() {
    let mut e = setup(
        &[GLASSES, index::OKO_THIEF_OF_CROWNS],
        &[index::LIGHTNING_BOLT],
    );
    let glasses = on_battlefield(&e, P0, GLASSES).unwrap();
    let oko = on_battlefield(&e, P0, index::OKO_THIEF_OF_CROWNS).unwrap();
    add_mana(&mut e, P0, ManaColor::White, 1);
    assert!(offered(&e, P0, index::LIGHTNING_BOLT));
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: oko,
            ability_index: 1,
        },
    )
    .unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![glasses],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, glasses), (3, 3));
    assert!(
        e.state()
            .object(glasses)
            .unwrap()
            .characteristics()
            .abilities_lost
            .is_some()
    );
    assert!(
        !offered(&e, P0, index::LIGHTNING_BOLT),
        "Oko's Elk has lost the spending ability"
    );
    assert_eq!(e.state().players[0].mana_pool.total(), 1);
}

#[test]
fn sunglasses_of_urza_preserves_ancient_ziggurats_creature_only_restriction() {
    let mut e = setup(
        &[GLASSES, index::ANCIENT_ZIGGURAT],
        &[index::RAGING_GOBLIN, index::LIGHTNING_BOLT],
    );
    activate(&mut e, P0, index::ANCIENT_ZIGGURAT, 0);
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    assert!(
        !offered(&e, P0, index::LIGHTNING_BOLT),
        "white creature-only mana still cannot pay an instant"
    );
    assert!(
        offered(&e, P0, index::RAGING_GOBLIN),
        "the same white mana can pay this red creature"
    );
    let goblin = in_hand(&e, P0, index::RAGING_GOBLIN).unwrap();
    cast_with_floating(&mut e, P0, index::RAGING_GOBLIN);
    assert_eq!(
        e.state()
            .object(goblin)
            .unwrap()
            .paid
            .as_ref()
            .unwrap()
            .colors_spent,
        ColorSet::of(Color::White)
    );
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::RAGING_GOBLIN).is_some());
    assert!(!offered(&e, P0, index::LIGHTNING_BOLT));
}
