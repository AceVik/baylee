//! Counted costs, devotion and amass: real cards, including hundreds of payers.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::{index, subtypes};
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn choose(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    e.apply(P0, PlayerAction::ChooseObjects { objects })
        .unwrap();
}
fn options(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    let Pending::ChooseCards { options, .. } = e.pending() else {
        panic!("{:?}", e.pending());
    };
    options.clone()
}
fn offered(e: &Engine<RegistryLookup>, source: ObjectId, ability: u32) -> bool {
    matches!(e.pending(), Pending::Priority { legal, .. } if legal.abilities.contains(&(source, ability)))
}

#[test]
fn counted_cost_starforge_gathers_300_artifacts_before_any_payment_and_forbids_duplicates() {
    let mut board = vec![index::ORNITHOPTER; 300];
    board.extend([index::SECLUDED_STARFORGE, index::GRIZZLY_BEARS]);
    let mut e = prepared(&[], &board, &[index::ORNITHOPTER]);
    let forge = on_battlefield(&e, P0, index::SECLUDED_STARFORGE).unwrap();
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    // Even when it is an artifact, the source cannot pay both taps.
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(forge)
        .unwrap()
        .base_mut()
        .types = TypeSet::LAND.union(TypeSet::ARTIFACT);
    e.refresh_offer();
    let mana = e.state().players[0].mana_pool.total();
    activate(&mut e, P0, index::SECLUDED_STARFORGE, 1);
    assert!(matches!(
        e.pending(),
        Pending::ChooseNumber {
            min: 0,
            max: 300,
            ..
        }
    ));
    e.apply(P0, PlayerAction::ChooseNumber(300)).unwrap();
    choose(&mut e, vec![bear]);
    let artifacts = options(&e);
    assert_eq!(artifacts.len(), 300);
    assert!(!artifacts.contains(&forge));
    let hash = e.state().snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![artifacts[0]; 255]
            }
        )
        .is_err()
    );
    assert_eq!(e.state().snapshot_hash(), hash);
    choose(&mut e, artifacts[..255].to_vec());
    assert!(matches!(
        e.pending(),
        Pending::ChooseCards {
            min: 45,
            max: 45,
            ..
        }
    ));
    assert_eq!(e.state().players[0].mana_pool.total(), mana);
    assert!(artifacts.iter().all(|id| !is_tapped(&e, *id)));
    assert!(!is_tapped(&e, forge));
    choose(&mut e, artifacts[255..].to_vec());
    assert_eq!(e.state().players[0].mana_pool.total(), mana - 2);
    assert!(artifacts.iter().all(|id| is_tapped(&e, *id)));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(bear).unwrap().characteristics().power,
        Some(302)
    );
    reach_their_main_phase(&mut e, P1);
    assert_eq!(
        e.state().object(bear).unwrap().characteristics().power,
        Some(2)
    );
}

#[test]
fn counted_cost_zero_is_legal_for_starforge_and_pasture() {
    for card in [index::SECLUDED_STARFORGE, index::SPRINGJACK_PASTURE] {
        let mut e = prepared(&[], &[card, index::GRIZZLY_BEARS], &[]);
        let source = on_battlefield(&e, P0, card).unwrap();
        let ability = if card == index::SECLUDED_STARFORGE {
            1
        } else {
            2
        };
        activate(&mut e, P0, card, ability);
        assert!(matches!(
            e.pending(),
            Pending::ChooseNumber { min: 0, max: 0, .. }
        ));
        e.apply(P0, PlayerAction::ChooseNumber(0)).unwrap();
        if card == index::SECLUDED_STARFORGE {
            let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
            choose(&mut e, vec![bear]);
            pass_until(&mut e, stack_is_empty);
        }
        assert!(is_tapped(&e, source));
        assert_eq!(e.state().players[0].life, 20);
        assert!(stack_is_empty(&e));
    }
}

#[test]
fn counted_cost_pasture_sacrifices_300_goats_then_adds_one_color_and_life_without_the_stack() {
    let mut board = vec![index::GRIZZLY_BEARS; 300];
    board.push(index::SPRINGJACK_PASTURE);
    let mut e = prepared(&[], &board, &[index::GRIZZLY_BEARS]);
    let goats: Vec<_> = e
        .state()
        .battlefield_seen()
        .filter(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.controller == P0 && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .collect();
    for id in &goats {
        e.dev_state_mut(P0)
            .unwrap()
            .object_mut(*id)
            .unwrap()
            .base_mut()
            .subtypes
            .insert(subtypes::creature::GOAT);
    }
    e.refresh_offer();
    let blue = e.state().players[0].mana_pool.available(ManaColor::Blue);
    activate(&mut e, P0, index::SPRINGJACK_PASTURE, 2);
    e.apply(P0, PlayerAction::ChooseNumber(300)).unwrap();
    choose(&mut e, goats[..255].to_vec());
    assert!(
        goats
            .iter()
            .all(|id| e.state().object(*id).unwrap().zone == Zone::Battlefield)
    );
    choose(&mut e, goats[255..].to_vec());
    assert!(
        goats
            .iter()
            .all(|id| e.state().object(*id).unwrap().zone == Zone::Graveyard)
    );
    assert!(stack_is_empty(&e));
    assert!(matches!(e.pending(), Pending::ChooseColor { .. }));
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(
        e.state().players[0].mana_pool.available(ManaColor::Blue),
        blue + 300
    );
    assert_eq!(e.state().players[0].life, 320);
    assert!(stack_is_empty(&e));
}

#[test]
fn counted_cost_urami_sacrifices_all_own_lands_including_itself_before_token_resolution() {
    let mut e = prepared(
        &[],
        &[
            index::TOMB_OF_URAMI,
            index::FOREST,
            index::ISLAND,
            index::GRIZZLY_BEARS,
        ],
        &[index::FOREST],
    );
    let lands: Vec<_> = e
        .state()
        .battlefield_seen()
        .filter(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.controller == P0 && o.characteristics().types.contains(TypeSet::LAND)
            })
        })
        .collect();
    activate(&mut e, P0, index::TOMB_OF_URAMI, 1);
    assert_eq!(lands.len(), 3);
    assert!(
        lands
            .iter()
            .all(|id| e.state().object(*id).unwrap().zone == Zone::Graveyard)
    );
    assert!(on_battlefield(&e, P1, index::FOREST).is_some());
    pass_until(&mut e, stack_is_empty);
    let urami = e
        .state()
        .battlefield_seen()
        .filter_map(|id| e.state().object(id))
        .find(|o| e.state().names.get(o.base.name) == "Urami")
        .unwrap();
    let chars = urami.characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(5), Some(5)));
    assert!(chars.keywords.contains(KeywordSet::FLYING));
    assert!(
        chars
            .supertypes
            .contains(baylee_core::types::SupertypeSet::LEGENDARY)
    );
}

#[test]
fn counted_cost_war_room_distinguishes_no_commander_from_colorless_and_unions_partner_colors() {
    let cases: &[(&[CardIndex], Option<u32>)] = &[
        (&[], None),
        (&[index::KARN_THE_GREAT_CREATOR], Some(0)),
        (&[index::KENRITH_THE_RETURNED_KING], Some(5)),
        (
            &[index::RAGAVAN_NIMBLE_PILFERER, index::RORIX_BLADEWING],
            Some(1),
        ),
    ];
    for &(commanders, colors) in cases {
        let mut e = Duel::new(SEED, index::FOREST)
            .commander(0, commanders)
            .battlefield(0, &[index::WAR_ROOM])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        e.dev_state_mut(P0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 3);
        e.refresh_offer();
        let room = on_battlefield(&e, P0, index::WAR_ROOM).unwrap();
        let Some(colors) = colors else {
            assert!(!offered(&e, room, 1));
            continue;
        };
        let before = library_size(&e, P0);
        activate(&mut e, P0, index::WAR_ROOM, 1);
        assert_eq!(
            e.state().players[0].life,
            20 - i32::try_from(colors).unwrap()
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(library_size(&e, P0), before - 1);
        if colors > 0 {
            e.dev_state_mut(P0).unwrap().set_tapped(room, false);
            e.dev_state_mut(P0).unwrap().players[0].life = i32::try_from(colors).unwrap() - 1;
            e.dev_state_mut(P0).unwrap().players[0]
                .mana_pool
                .add(ManaColor::Colorless, 3);
            e.refresh_offer();
            assert!(!offered(&e, room, 1));
        }
    }
}

#[test]
fn devotion_nykthos_counts_colored_symbols_not_permanents_or_color_identity() {
    for (color, expected) in [
        (ManaColor::White, 2),
        (ManaColor::Blue, 3),
        (ManaColor::Black, 1),
        (ManaColor::Red, 1),
        (ManaColor::Green, 1),
    ] {
        let mut e = prepared(
            &[],
            &[index::NYKTHOS_SHRINE_TO_NYX, index::GRIZZLY_BEARS],
            &[index::SERRA_ANGEL],
        );
        let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
        e.dev_state_mut(P0)
            .unwrap()
            .object_mut(bear)
            .unwrap()
            .base_mut()
            .mana_cost = baylee_core::mana!("{W/U}{2/B}{G/P}{R/W/P}{X}{S}{C}{3}{U}{U}");
        e.refresh_offer();
        activate(&mut e, P0, index::NYKTHOS_SHRINE_TO_NYX, 1);
        let before = e.state().players[0].mana_pool.available(color);
        assert!(stack_is_empty(&e));
        assert!(
            e.apply(P0, PlayerAction::ChooseColor(ManaColor::Colorless))
                .is_err()
        );
        e.apply(P0, PlayerAction::ChooseColor(color)).unwrap();
        assert_eq!(
            e.state().players[0].mana_pool.available(color),
            before + expected
        );
        assert!(stack_is_empty(&e));
    }
    let mut e = prepared(&[], &[index::NYKTHOS_SHRINE_TO_NYX], &[]);
    activate(&mut e, P0, index::NYKTHOS_SHRINE_TO_NYX, 1);
    let before = e.state().players[0].mana_pool.total();
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), before);
}

#[test]
fn amass_barad_dur_requires_a_death_and_chooses_among_doubled_or_existing_armies() {
    for existing in [false, true] {
        let mut e = prepared(
            &[],
            &[
                index::BARAD_DUR,
                index::DOUBLING_SEASON,
                index::GRIZZLY_BEARS,
                index::ORNITHOPTER,
            ],
            &[index::GRIZZLY_BEARS],
        );
        let barad = on_battlefield(&e, P0, index::BARAD_DUR).unwrap();
        let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
        let thopter = on_battlefield(&e, P0, index::ORNITHOPTER).unwrap();
        assert!(!offered(&e, barad, 1));
        if existing {
            for army in [bear, thopter] {
                e.dev_state_mut(P0)
                    .unwrap()
                    .object_mut(army)
                    .unwrap()
                    .base_mut()
                    .subtypes
                    .insert(subtypes::creature::ARMY);
            }
        }
        let victim = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
        e.dev_state_mut(P0)
            .unwrap()
            .move_object(
                victim,
                ZoneLocation::Graveyard(P1),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        e.refresh_offer();
        activate(&mut e, P0, index::BARAD_DUR, 1);
        let mana = e.state().players[0].mana_pool.total();
        e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
        assert_eq!(e.state().players[0].mana_pool.total(), mana - 7);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::Amass,
                    ..
                }
            )
        });
        let armies = options(&e);
        assert_eq!(armies.len(), 2);
        if !existing {
            assert!(armies.iter().all(|id| {
                e.state()
                    .object(*id)
                    .unwrap()
                    .base
                    .subtypes
                    .contains(subtypes::creature::ORC)
            }));
        }
        let chosen = if existing { thopter } else { armies[1] };
        choose(&mut e, vec![chosen]);
        pass_until(&mut e, stack_is_empty);
        let o = e.state().object(chosen).unwrap();
        assert_eq!(o.counters.get(CounterKind::P1P1), 6);
        assert!(
            o.characteristics()
                .subtypes
                .contains(subtypes::creature::ORC)
        );
        if existing {
            assert!(
                !o.base.subtypes.contains(subtypes::creature::ORC),
                "the subtype addition is not copiable"
            );
            assert_eq!(
                e.state()
                    .object(bear)
                    .unwrap()
                    .counters
                    .get(CounterKind::P1P1),
                0
            );
        } else {
            assert!(
                e.state()
                    .object(armies[0])
                    .is_none_or(|o| o.zone != Zone::Battlefield)
            );
        }
        reach_their_main_phase(&mut e, P1);
        assert!(
            !offered(&e, barad, 1),
            "a previous turn's death no longer qualifies"
        );
    }
}
