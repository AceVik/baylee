//! Pyramids: modal activation and a one-shot destruction replacement, not regeneration.
// oracle_id = "da2f0d16-3cb4-492d-8535-52a31dbae95e"

#[allow(clippy::wildcard_imports)] // Shared canonical-card vocabulary.
use super::*;
use crate::engine::card_tests::{aura_kit as ak, legends_kit as lk, partials_w1b_kit as kit};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn game(board: &[CardIndex], hand: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut own = vec![index::PYRAMIDS];
    own.extend_from_slice(board);
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &own)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn choose_mode(e: &mut Engine<RegistryLookup>, mode: usize) {
    let Pending::ChooseCastMode {
        player, options, ..
    } = e.pending()
    else {
        panic!("mode")
    };
    assert_eq!(*player, P0);
    let slot = options
        .iter()
        .position(|o| o.kind == CastModeKind::Mode(mode))
        .expect("available mode");
    e.apply(P0, PlayerAction::ChooseMode(slot)).unwrap();
}

fn start_mode(e: &mut Engine<RegistryLookup>, mode: usize) -> ObjectId {
    let pyramids = on_battlefield(e, P0, index::PYRAMIDS).unwrap();
    lk::float(e, P0, &[(ManaColor::Colorless, 2)]);
    kit::activate_object(e, P0, pyramids, 0);
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        2,
        "mode precedes payment"
    );
    choose_mode(e, mode);
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        2,
        "targets precede payment"
    );
    assert!(!is_tapped(e, pyramids));
    pyramids
}

fn protect(e: &mut Engine<RegistryLookup>, target: ObjectId) {
    let pyramids = start_mode(e, 1);
    lk::aim(e, &[target], &[]);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert!(!is_tapped(e, pyramids));
    pass_until(e, stack_is_empty);
}

fn destroy(e: &mut Engine<RegistryLookup>, card: CardIndex, target: ObjectId) {
    let mana = if card == index::FISSURE { 5 } else { 3 };
    lk::float(e, P0, &[(ManaColor::Red, mana)]);
    cast_with_floating(e, P0, card);
    lk::aim(e, &[target], &[]);
    pass_until(e, stack_is_empty);
}

#[test]
fn batch60_pyramids_aura_mode_destroys_only_aura_on_land_and_does_not_tap() {
    let mut e = game(
        &[index::FOREST, index::GRIZZLY_BEARS, index::BONESPLITTER],
        &[index::WILD_GROWTH, index::HOLY_STRENGTH],
    );
    let land = on_battlefield(&e, P0, index::FOREST).unwrap();
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let growth = ak::cast_aura(
        &mut e,
        P0,
        index::WILD_GROWTH,
        land,
        &[(ManaColor::Green, 1)],
    );
    let strength = ak::cast_aura(
        &mut e,
        P0,
        index::HOLY_STRENGTH,
        bear,
        &[(ManaColor::White, 1)],
    );
    let pyramids = start_mode(&mut e, 0);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("Aura target")
    };
    assert_eq!(options, &vec![growth]);
    assert!(!options.contains(&strength));
    lk::aim(&mut e, &[growth], &[]);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, index::WILD_GROWTH).is_some());
    assert!(on_battlefield(&e, P0, index::HOLY_STRENGTH).is_some());
    assert!(!is_tapped(&e, pyramids));
    protect(&mut e, land); // Same permanent can activate again without untapping.
}

#[test]
fn batch60_pyramids_land_mode_replaces_exactly_one_destruction_even_without_damage() {
    let mut e = game(&[index::FOREST], &[index::STONE_RAIN, index::STONE_RAIN]);
    let land = on_battlefield(&e, P0, index::FOREST).unwrap();
    protect(&mut e, land);
    destroy(&mut e, index::STONE_RAIN, land);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
    assert!(!is_tapped(&e, land));
    assert_eq!(e.state().object(land).unwrap().damage, 0);
    destroy(&mut e, index::STONE_RAIN, land);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
}

#[test]
fn batch60_pyramids_replacement_works_when_regeneration_is_forbidden_and_expires() {
    for expired in [false, true] {
        let mut e = game(&[index::FOREST], &[index::FISSURE]);
        let land = on_battlefield(&e, P0, index::FOREST).unwrap();
        protect(&mut e, land);
        if expired {
            reach_their_main_phase(&mut e, P1);
            reach_their_main_phase(&mut e, P0);
        }
        destroy(&mut e, index::FISSURE, land);
        assert_eq!(
            e.state().object(land).unwrap().zone,
            if expired {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            }
        );
        assert!(!is_tapped(&e, land));
    }
}

#[test]
fn batch60_pyramids_clears_lethal_damage_without_tapping_or_removing_attacker() {
    let mut e = game(
        &[index::INKMOTH_NEXUS, index::SERRA_S_BLESSING],
        &[index::LIGHTNING_BOLT],
    );
    let land = on_battlefield(&e, P0, index::INKMOTH_NEXUS).unwrap();
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    kit::activate_object(&mut e, P0, land, 1);
    pass_until(&mut e, stack_is_empty);
    protect(&mut e, land);
    kit::declare_attack(&mut e, &[land], P1);
    assert!(e.state().combat.is_attacking(land));
    assert!(!is_tapped(&e, land));
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut e, &[land], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(land).unwrap().damage, 0);
    assert!(!is_tapped(&e, land));
    assert!(e.state().combat.is_attacking(land));
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().players[1].poison, 1);
}

#[test]
fn batch60_pyramids_departed_land_returns_as_new_object_without_old_protection() {
    let mut e = game(&[index::FOREST], &[index::BOOMERANG, index::STONE_RAIN]);
    let land = on_battlefield(&e, P0, index::FOREST).unwrap();
    protect(&mut e, land);
    lk::float(&mut e, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut e, P0, index::BOOMERANG);
    lk::aim(&mut e, &[land], &[]);
    pass_until(&mut e, stack_is_empty);
    let returned = in_hand(&e, P0, index::FOREST).unwrap();
    e.apply(P0, PlayerAction::PlayLand { card: returned })
        .unwrap();
    destroy(&mut e, index::STONE_RAIN, returned);
    assert_eq!(e.state().object(returned).unwrap().zone, Zone::Graveyard);
}

#[test]
fn batch60_pyramids_modes_are_filtered_and_two_mana_required() {
    let mut e = game(&[index::FOREST], &[]);
    let source = on_battlefield(&e, P0, index::PYRAMIDS).unwrap();
    assert!(!priority_offer(&e).abilities.contains(&(source, 0)));
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    assert!(!priority_offer(&e).abilities.contains(&(source, 0)));
    lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
    kit::activate_object(&mut e, P0, source, 0);
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        panic!("mode")
    };
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].kind, CastModeKind::Mode(1));
    choose_mode(&mut e, 1);
    let land = on_battlefield(&e, P0, index::FOREST).unwrap();
    lk::aim(&mut e, &[land], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert!(!is_tapped(&e, source));
}

#[test]
fn batch60_pyramids_land_shield_does_not_replace_exile_or_zero_toughness() {
    for removal in [index::SWORDS_TO_PLOWSHARES, index::DISFIGURE] {
        let mut e = game(&[index::INKMOTH_NEXUS], &[removal]);
        let land = on_battlefield(&e, P0, index::INKMOTH_NEXUS).unwrap();
        lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
        kit::activate_object(&mut e, P0, land, 1);
        pass_until(&mut e, stack_is_empty);
        protect(&mut e, land);
        let color = if removal == index::DISFIGURE {
            ManaColor::Black
        } else {
            ManaColor::White
        };
        lk::float(&mut e, P0, &[(color, 1)]);
        cast_with_floating(&mut e, P0, removal);
        lk::aim(&mut e, &[land], &[]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().object(land).unwrap().zone,
            if removal == index::DISFIGURE {
                Zone::Graveyard
            } else {
                Zone::Exile
            }
        );
    }
}

#[test]
fn batch60_pyramids_aura_target_leaving_battlefield_is_not_destroyed_from_hand() {
    let mut e = game(&[index::FOREST], &[index::WILD_GROWTH, index::BOOMERANG]);
    let land = on_battlefield(&e, P0, index::FOREST).unwrap();
    let aura = ak::cast_aura(
        &mut e,
        P0,
        index::WILD_GROWTH,
        land,
        &[(ManaColor::Green, 1)],
    );
    start_mode(&mut e, 0);
    lk::aim(&mut e, &[aura], &[]);
    lk::float(&mut e, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut e, P0, index::BOOMERANG);
    lk::aim(&mut e, &[aura], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, P0, index::WILD_GROWTH).is_some());
    assert!(in_graveyard(&e, P0, index::WILD_GROWTH).is_none());
}

#[test]
fn batch60_pyramids_protects_opponents_land_after_pyramids_itself_leaves() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::PYRAMIDS])
        .battlefield(1, &[index::FOREST])
        .hand(0, &[index::DISENCHANT, index::STONE_RAIN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let land = on_battlefield(&e, P1, index::FOREST).unwrap();
    let source = on_battlefield(&e, P0, index::PYRAMIDS).unwrap();
    protect(&mut e, land);
    lk::float(&mut e, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut e, P0, index::DISENCHANT);
    lk::aim(&mut e, &[source], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, index::PYRAMIDS).is_some());
    destroy(&mut e, index::STONE_RAIN, land);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(land).unwrap().controller, P1);
}
