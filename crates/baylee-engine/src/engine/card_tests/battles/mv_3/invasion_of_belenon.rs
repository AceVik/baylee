//! Invasion of Belenon: printed front, actual Siege defeat casting, and back anthem.
// oracle_id = "e462e78d-2d26-4a2d-8d8f-5f31abe5c924"

#[allow(clippy::wildcard_imports)] // Shared canonical-card vocabulary.
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::{index, subtypes};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn game() -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::GRIZZLY_BEARS])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(
            0,
            &[
                index::INVASION_OF_BELENON,
                index::LIGHTNING_BOLT,
                index::SHOCK,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn cast_front(e: &mut Engine<RegistryLookup>) -> ObjectId {
    lk::float(e, P0, &[(ManaColor::White, 3)]);
    cast_with_floating(e, P0, index::INVASION_OF_BELENON);
    pass_until(e, stack_is_empty);
    let siege = on_battlefield(e, P0, index::INVASION_OF_BELENON).unwrap();
    assert_eq!(counters_on(e, siege, CounterKind::Defense), 5);
    assert_eq!(crate::battle::protector(e.state(), siege), Some(P1));
    siege
}

fn knights(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.token.is_some()
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::KNIGHT)
            })
        })
        .collect()
}

fn defeat_to_cast_offer(e: &mut Engine<RegistryLookup>, siege: ObjectId) {
    lk::float(e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(e, P0, index::LIGHTNING_BOLT);
    lk::aim(e, &[siege], &[]);
    pass_until(e, stack_is_empty);
    assert_eq!(counters_on(e, siege, CounterKind::Defense), 2);
    assert_eq!(e.state().object(siege).unwrap().damage, 0);
    lk::float(e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(e, P0, index::SHOCK);
    lk::aim(e, &[siege], &[]);
    pass_until(e, |e| {
        counters_on(e, siege, CounterKind::Defense) == 0
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        e.state().object(siege).unwrap().zone,
        Zone::Battlefield,
        "defeat uses a triggered ability rather than immediate transformation"
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Stack).len(), 1);
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::PlayFromExile,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!(*player, P0);
    assert_eq!(options, &vec![siege]);
    assert_eq!((*min, *max), (0, 1));
    assert_eq!(e.state().object(siege).unwrap().zone, Zone::Exile);
}

#[test]
fn battle_belenon_front_cast_enters_with_five_defense_and_exact_knight() {
    let mut e = game();
    let siege = cast_front(&mut e);
    let o = e.state().object(siege).unwrap();
    assert_eq!(
        e.state().names.get(o.characteristics().name),
        "Invasion of Belenon"
    );
    assert_eq!(o.characteristics().types, TypeSet::BATTLE);
    assert_eq!(
        o.characteristics().colors,
        ColorSet::from_slice(&[Color::White])
    );
    assert_eq!(o.characteristics().defense, Some(5));
    assert!(
        o.characteristics()
            .subtypes
            .contains(subtypes::battle::SIEGE)
    );
    let tokens = knights(&e);
    assert_eq!(tokens.len(), 1);
    let knight = e.state().object(tokens[0]).unwrap();
    assert_eq!(knight.controller, P0);
    assert_eq!(pt(&e, tokens[0]), (2, 2));
    assert_eq!(
        knight.characteristics().colors,
        ColorSet::from_slice(&[Color::White, Color::Blue])
    );
    assert!(keywords(&e, tokens[0]).contains(KeywordSet::VIGILANCE));
    assert!(!is_tapped(&e, tokens[0]));
    let own_bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&e, own_bear), (2, 2), "back anthem is inactive on front");
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn battle_belenon_defeat_exiles_then_casts_white_back_and_anthem_only_buffs_controller() {
    let mut e = game();
    let siege = cast_front(&mut e);
    let knight = knights(&e)[0];
    defeat_to_cast_offer(&mut e, siege);
    let casts = e.state().per_turn.spells_cast[0];
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![siege],
        },
    )
    .unwrap();
    let back = e.state().object(siege).unwrap();
    assert_eq!(back.zone, Zone::Stack);
    assert_eq!(
        e.state().names.get(back.characteristics().name),
        "Belenon War Anthem"
    );
    assert_eq!(back.characteristics().types, TypeSet::ENCHANTMENT);
    assert_eq!(
        back.characteristics().colors,
        ColorSet::from_slice(&[Color::White])
    );
    assert_eq!(back.characteristics().defense, None);
    assert_eq!(e.state().per_turn.spells_cast[0], casts + 1);
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        0,
        "defeat cast has no mana cost payment"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(siege).unwrap().zone, Zone::Battlefield);
    assert_eq!(counters_on(&e, siege, CounterKind::Defense), 0);
    assert_eq!(crate::battle::protector(e.state(), siege), None);
    assert_eq!(
        knights(&e),
        vec![knight],
        "back entry does not repeat front ETB"
    );
    let own = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let other = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&e, own), (3, 3));
    assert_eq!(pt(&e, knight), (3, 3));
    assert_eq!(pt(&e, other), (2, 2));
}

#[test]
fn battle_belenon_defeat_cast_is_optional_and_declining_leaves_card_exiled() {
    let mut e = game();
    let siege = cast_front(&mut e);
    defeat_to_cast_offer(&mut e, siege);
    let casts = e.state().per_turn.spells_cast[0];
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(siege).unwrap().zone, Zone::Exile);
    assert_eq!(e.state().per_turn.spells_cast[0], casts);
    assert_eq!(knights(&e).len(), 1);
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&e, bear), (2, 2));
}

#[test]
fn battle_belenon_multiplayer_controller_chooses_opponent_before_entry_is_published() {
    let p2 = PlayerId::new(2);
    let mut e = Duel::table(SEED, index::FOREST, 3)
        .hand(0, &[index::INVASION_OF_BELENON])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    lk::float(&mut e, P0, &[(ManaColor::White, 3)]);
    cast_with_floating(&mut e, P0, index::INVASION_OF_BELENON);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    let Pending::ChoosePlayer { player, options } = e.pending() else {
        unreachable!()
    };
    assert_eq!(*player, P0);
    assert_eq!(options.len(), 2);
    assert!(options.contains(&P1) && options.contains(&p2));
    assert!(!options.contains(&P0));
    assert!(on_battlefield(&e, P0, index::INVASION_OF_BELENON).is_none());
    assert!(knights(&e).is_empty());
    e.apply(P0, PlayerAction::ChoosePlayer(p2)).unwrap();
    pass_until(&mut e, stack_is_empty);
    let siege = on_battlefield(&e, P0, index::INVASION_OF_BELENON).unwrap();
    assert_eq!(crate::battle::protector(e.state(), siege), Some(p2));
    assert_eq!(counters_on(&e, siege, CounterKind::Defense), 5);
    assert_eq!(knights(&e).len(), 1);
}
