//! Consecrate Land: indestructibility and the independent Aura restriction.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::{Cause, GameEvent};
use baylee_core::generated::{index, subtypes};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn setup() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(1110, forest())
        .battlefield(0, &[plains(), swamp(), index::ZURAN_ORB])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(
            0,
            &[
                consecrate_land(),
                consecrate_land(),
                index::EVIL_PRESENCE,
                index::EVIL_PRESENCE,
                index::DISENCHANT,
                index::STONE_RAIN,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let land = on_battlefield(&e, P1, forest()).unwrap();
    (e, land)
}

fn cast(e: &mut Engine<RegistryLookup>, card: CardIndex, target: ObjectId) -> ObjectId {
    pass_until(e, |e| at_rest(e, P0));
    for color in [ManaColor::White, ManaColor::Black, ManaColor::Red] {
        e.dev_state_mut(P0).unwrap().players[0]
            .mana_pool
            .add(color, 4);
    }
    e.refresh_offer();
    let source = in_hand(e, P0, card).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card: source })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("target choice")
    };
    assert!(options.contains(&target));
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    )
    .unwrap();
    pass_until(e, stack_is_empty);
    source
}

fn entered(e: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    e.journal().entries().iter().any(|entry| {
        matches!(entry.event,
        GameEvent::ZoneChanged { object: id, to: Zone::Battlefield, .. } if id == object)
    })
}

fn indestructible(e: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    e.state()
        .object(object)
        .unwrap()
        .characteristics()
        .keywords
        .contains(KeywordSet::INDESTRUCTIBLE)
}

#[test]
fn consecrate_land_removes_existing_auras_but_keeps_itself() {
    let (mut e, land) = setup();
    let evil = cast(&mut e, index::EVIL_PRESENCE, land);
    assert!(
        e.state()
            .object(land)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::SWAMP)
    );
    let evil_two = cast(&mut e, index::EVIL_PRESENCE, land);
    let consecrate = cast(&mut e, consecrate_land(), land);
    assert_eq!(e.state().object(evil_two).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(evil).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        e.state().object(consecrate).unwrap().attached_to,
        Some(land)
    );
    assert!(
        e.state()
            .object(land)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(subtypes::land::FOREST)
    );
    assert!(indestructible(&e, land));
}

#[test]
fn consecrate_land_allows_aura_targeting_but_prevents_entry_entirely() {
    let (mut e, land) = setup();
    cast(&mut e, consecrate_land(), land);
    let evil = cast(&mut e, index::EVIL_PRESENCE, land);
    assert_eq!(e.state().object(evil).unwrap().zone, Zone::Graveyard);
    assert!(
        !entered(&e, evil),
        "forbidden Aura never enters and causes no ETB trigger"
    );
    assert!(indestructible(&e, land));
}

#[test]
fn consecrate_land_refuses_another_copy_of_consecrate_land() {
    let (mut e, land) = setup();
    let first = cast(&mut e, consecrate_land(), land);
    let second = cast(&mut e, consecrate_land(), land);
    assert_eq!(e.state().object(first).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(second).unwrap().zone, Zone::Graveyard);
    assert!(!entered(&e, second));
}

#[test]
fn consecrate_land_still_allows_land_destruction_targeting_but_land_survives() {
    let (mut e, land) = setup();
    cast(&mut e, consecrate_land(), land);
    let rain = cast(&mut e, index::STONE_RAIN, land);
    assert_eq!(e.state().object(rain).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
}

#[test]
fn consecrate_land_restrictions_end_when_it_leaves() {
    let (mut e, land) = setup();
    let consecrate = cast(&mut e, consecrate_land(), land);
    cast(&mut e, index::DISENCHANT, consecrate);
    assert!(!indestructible(&e, land));
    let evil = cast(&mut e, index::EVIL_PRESENCE, land);
    assert_eq!(e.state().object(evil).unwrap().attached_to, Some(land));
    assert_eq!(e.state().object(evil).unwrap().zone, Zone::Battlefield);
    cast(&mut e, index::STONE_RAIN, land);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
}

#[test]
fn consecrate_land_prevents_attached_aura_entry_from_a_graveyard() {
    let (mut e, land) = setup();
    cast(&mut e, consecrate_land(), land);
    let evil = in_hand(&e, P0, index::EVIL_PRESENCE).unwrap();
    let state = e.dev_state_mut(P0).unwrap();
    state
        .move_object(
            evil,
            ZoneLocation::Graveyard(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state.object_mut(evil).unwrap().attached_to = Some(land);
    state
        .move_object(
            evil,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_eq!(state.object(evil).unwrap().zone, Zone::Graveyard);
    assert!(!entered(&e, evil));
}

#[test]
fn consecrate_land_does_not_stop_exile_and_falls_off_with_its_land() {
    let (mut e, land) = setup();
    let consecrate = cast(&mut e, consecrate_land(), land);
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(
            land,
            ZoneLocation::Exile(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let Pending::Priority { player, .. } = e.pending() else {
        panic!("priority")
    };
    let player = *player;
    e.apply(player, PlayerAction::PassPriority).unwrap();
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Exile);
    assert_eq!(e.state().object(consecrate).unwrap().zone, Zone::Graveyard);
}

#[test]
fn consecrate_land_keeps_an_existing_aura_on_its_old_host_if_a_move_is_forbidden() {
    use baylee_cards_dsl::{AbilityDef, Effect, Filter, TargetSpec};
    static MOVE_AURA: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!("{0}"),
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::LAND)
        }],
        target = Some(TargetSpec::Object(&Filter::LAND))
    )];
    let (mut e, land) = setup();
    let other = on_battlefield(&e, P0, plains()).unwrap();
    let evil = cast(&mut e, index::EVIL_PRESENCE, other);
    cast(&mut e, consecrate_land(), land);
    pass_until(&mut e, |e| at_rest(e, P0));
    // Exercise an on-battlefield attachment effect through a real activation.
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(evil)
        .unwrap()
        .own_abilities = Some(MOVE_AURA);
    e.refresh_offer();
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: evil,
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![land],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(evil).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(evil).unwrap().attached_to, Some(other));
}

#[test]
fn consecrate_land_does_not_prevent_sacrificing_its_land() {
    let (mut e, _) = setup();
    let land = on_battlefield(&e, P0, plains()).unwrap();
    let consecrate = cast(&mut e, consecrate_land(), land);
    pass_until(&mut e, |e| at_rest(e, P0));
    let orb = on_battlefield(&e, P0, index::ZURAN_ORB).unwrap();
    let life = e.state().players[0].life;
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: orb,
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![land],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(consecrate).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().players[0].life, life + 2);
}
