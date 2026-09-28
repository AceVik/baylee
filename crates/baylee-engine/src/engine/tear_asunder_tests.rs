use super::{testkit::*, *};
use crate::{
    choice::CastModeKind,
    event::Cause,
    zone::{ZoneLocation, ZonePosition},
};
use baylee_core::ids::CardIndex;
const P0: PlayerId = PlayerId::new(0);
fn card(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).unwrap()
}
fn setup(artifact: bool, four_mana: bool) -> Engine<RegistryLookup> {
    let lands = if four_mana {
        vec![card("Forest"), card("Forest"), card("Swamp"), card("Swamp")]
    } else {
        vec![card("Forest"), card("Forest")]
    };
    let other = if artifact {
        vec![card("Mind Stone"), card("Llanowar Elves"), card("Swamp")]
    } else {
        vec![card("Llanowar Elves"), card("Swamp")]
    };
    let mut e = Duel::table(972, card("Forest"), 3)
        .battlefield(0, &lands)
        .battlefield(1, &other)
        .battlefield(2, &[card("Llanowar Elves")])
        .hand(0, &[card("Tear Asunder")])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}
fn choose_mode(e: &mut Engine<RegistryLookup>, mode: CastModeKind) {
    let Pending::ChooseCastMode { options, .. } = e.pending() else {
        panic!("expected cast mode: {:?}", e.pending())
    };
    let index = options.iter().position(|o| o.kind == mode).unwrap();
    e.apply(P0, PlayerAction::ChooseMode(index)).unwrap();
}
fn options(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    match e.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        q => panic!("expected targets, got {q:?}"),
    }
}
#[test]
fn tear_asunder_kicked_cast_offers_creatures_on_every_seat_but_no_lands() {
    let mut e = setup(true, true);
    cast_from_hand(&mut e, P0, card("Tear Asunder"));
    choose_mode(&mut e, CastModeKind::Kicked);
    let options = options(&e);
    assert_eq!(options.len(), 3);
    let elf = on_battlefield(&e, PlayerId::new(2), card("Llanowar Elves")).unwrap();
    let land = on_battlefield(&e, PlayerId::new(1), card("Swamp")).unwrap();
    assert!(options.contains(&elf));
    assert!(!options.contains(&land));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![land]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    let spell = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    assert!(e.state.object(spell).unwrap().kicked);
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        0,
        "base plus kicker paid once"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(elf).unwrap().zone, crate::zone::Zone::Exile);
}
#[test]
fn tear_asunder_ordinary_cast_keeps_the_narrow_targets_and_cost() {
    let mut e = setup(true, true);
    cast_from_hand(&mut e, P0, card("Tear Asunder"));
    choose_mode(&mut e, CastModeKind::Normal);
    let stone = on_battlefield(&e, PlayerId::new(1), card("Mind Stone")).unwrap();
    assert_eq!(options(&e), vec![stone]);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![stone],
        },
    )
    .unwrap();
    assert_eq!(e.state.players[0].mana_pool.total(), 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(stone).unwrap().zone,
        crate::zone::Zone::Exile
    );
}
#[test]
fn tear_asunder_creature_only_board_requires_the_complete_kicked_cost() {
    let mut poor = setup(false, false);
    tap_all_mana(&mut poor, P0);
    let spell = in_hand(&poor, P0, card("Tear Asunder")).unwrap();
    assert!(
        poor.apply(P0, PlayerAction::CastSpell { card: spell })
            .is_err()
    );
    assert_eq!(poor.state.players[0].mana_pool.total(), 2);
    let mut e = setup(false, true);
    cast_from_hand(&mut e, P0, card("Tear Asunder"));
    let targets = options(&e);
    assert_eq!(targets.len(), 2);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![targets[0]],
        },
    )
    .unwrap();
    let spell = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    assert!(
        e.state.object(spell).unwrap().kicked,
        "the sole affordable way is kicked"
    );
    // The broader requirement still rejects a land at resolution.
    std::sync::Arc::make_mut(&mut e.state.object_mut(targets[0]).unwrap().base).types =
        baylee_core::types::TypeSet::LAND;
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(targets[0]).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}
#[test]
fn tear_asunder_free_cast_still_pays_kicker_and_chooses_it_before_targets() {
    let mut e = setup(false, true);
    tap_all_mana(&mut e, P0);
    let spell = in_hand(&e, P0, card("Tear Asunder")).unwrap();
    e.state
        .move_object(
            spell,
            ZoneLocation::Exile(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.start_free_cast(P0, spell).unwrap();
    assert!(matches!(
        e.pending(),
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::Kicker,
            ..
        }
    ));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    let target = options(&e)[0];
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    )
    .unwrap();
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        2,
        "only kicker is paid"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(target).unwrap().zone,
        crate::zone::Zone::Exile
    );
}
