//! Nether Shadow's graveyard upkeep, ordered cards, and returned creature.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::Cause;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Fixture list is bottom to top, like the engine's graveyard storage.
fn setup(cards: &[CardIndex], owner: PlayerId) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let mut e = Duel::new(1130, forest())
        .hand(owner.get() as usize, cards)
        .battlefield(0, &[index::SCAVENGING_OOZE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let mut objects = Vec::new();
    for &card in cards {
        let object = in_hand(&e, owner, card).unwrap();
        move_to(
            &mut e,
            object,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
        );
        objects.push(object);
    }
    (e, objects)
}

fn move_to(
    e: &mut Engine<RegistryLookup>,
    object: ObjectId,
    to: ZoneLocation,
    position: ZonePosition,
) {
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(object, to, position, Cause::Effect)
        .unwrap();
}

fn upkeep(e: &mut Engine<RegistryLookup>, owner: PlayerId) {
    if e.state().turn.active == owner {
        pass_until(e, |e| e.state().turn.active != owner);
    }
    pass_until(e, |e| {
        e.state().turn.active == owner && e.state().turn.step == crate::turn::Step::Upkeep
    });
}

fn triggers(e: &Engine<RegistryLookup>, source: ObjectId) -> usize {
    e.state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .filter(|&&id| {
            e.state()
                .object(id)
                .is_some_and(|o| o.ability.is_some_and(|a| a.source == source))
        })
        .count()
}

fn choose_return(e: &mut Engine<RegistryLookup>, owner: PlayerId, yes: bool) {
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    assert!(matches!(e.pending(), Pending::YesNo { player, .. } if *player == owner));
    e.apply(owner, PlayerAction::YesNo(yes)).unwrap();
    pass_until(e, stack_is_empty);
}

fn eligible() -> Vec<CardIndex> {
    vec![
        nether_shadow(),
        llanowar_elves(),
        llanowar_elves(),
        llanowar_elves(),
    ]
}

#[test]
fn nether_shadow_needs_at_least_three_creature_cards_above_it() {
    for count in 0..=4 {
        let mut cards = vec![nether_shadow()];
        cards.extend(vec![llanowar_elves(); count]);
        let (mut e, ids) = setup(&cards, P0);
        upkeep(&mut e, P0);
        assert_eq!(triggers(&e, ids[0]), usize::from(count >= 3));
        if count >= 3 {
            choose_return(&mut e, P0, true);
        }
        assert_eq!(
            e.state().object(ids[0]).unwrap().zone,
            if count >= 3 {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
    }
}

#[test]
fn nether_shadow_does_not_count_creatures_below_it() {
    let (mut e, ids) = setup(
        &[
            llanowar_elves(),
            llanowar_elves(),
            llanowar_elves(),
            nether_shadow(),
        ],
        P0,
    );
    upkeep(&mut e, P0);
    assert_eq!(triggers(&e, ids[3]), 0);
    assert_eq!(e.state().object(ids[3]).unwrap().zone, Zone::Graveyard);
}

#[test]
fn nether_shadow_counts_creatures_across_intervening_noncreatures() {
    let (mut e, ids) = setup(
        &[
            nether_shadow(),
            llanowar_elves(),
            plains(),
            llanowar_elves(),
            sol_ring(),
            llanowar_elves(),
        ],
        P0,
    );
    upkeep(&mut e, P0);
    assert_eq!(triggers(&e, ids[0]), 1);
    choose_return(&mut e, P0, true);
    assert_eq!(e.state().object(ids[0]).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(P0)), &ids[1..]);
}

#[test]
fn nether_shadow_does_not_count_noncreature_cards_as_creatures() {
    let (mut e, ids) = setup(
        &[
            nether_shadow(),
            llanowar_elves(),
            llanowar_elves(),
            plains(),
            sol_ring(),
        ],
        P0,
    );
    upkeep(&mut e, P0);
    assert_eq!(triggers(&e, ids[0]), 0);
}

#[test]
fn nether_shadow_triggers_only_on_its_owners_upkeep() {
    for owner in [P0, P1] {
        let (mut e, ids) = setup(&eligible(), owner);
        if owner == P0 {
            upkeep(&mut e, P1);
            assert_eq!(triggers(&e, ids[0]), 0);
        }
        upkeep(&mut e, owner);
        assert_eq!(triggers(&e, ids[0]), 1);
        choose_return(&mut e, owner, true);
        assert_eq!(e.state().object(ids[0]).unwrap().controller, owner);
    }
}

#[test]
fn nether_shadow_may_stay_and_try_again_next_upkeep() {
    let (mut e, ids) = setup(&eligible(), P0);
    upkeep(&mut e, P0);
    choose_return(&mut e, P0, false);
    assert_eq!(e.state().object(ids[0]).unwrap().zone, Zone::Graveyard);
    upkeep(&mut e, P0);
    assert_eq!(triggers(&e, ids[0]), 1);
    choose_return(&mut e, P0, true);
    assert_eq!(e.state().object(ids[0]).unwrap().zone, Zone::Battlefield);
}

#[test]
fn nether_shadow_rechecks_creature_count_after_a_real_exile_activation() {
    let (mut e, ids) = setup(&eligible(), P0);
    upkeep(&mut e, P0);
    assert_eq!(triggers(&e, ids[0]), 1);
    let ooze = on_battlefield(&e, P0, index::SCAVENGING_OOZE).unwrap();
    e.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 1);
    e.refresh_offer();
    // Only Ooze's implemented exile clause is needed by this interaction.
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: ooze,
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![ids[1]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(ids[1]).unwrap().zone, Zone::Exile);
    assert_eq!(e.state().object(ids[0]).unwrap().zone, Zone::Graveyard);
}

#[test]
fn nether_shadow_does_not_follow_its_source_out_of_the_graveyard_and_back() {
    let (mut e, ids) = setup(&eligible(), P0);
    upkeep(&mut e, P0);
    assert_eq!(triggers(&e, ids[0]), 1);
    move_to(&mut e, ids[0], ZoneLocation::Exile(P0), ZonePosition::Top);
    move_to(
        &mut e,
        ids[0],
        ZoneLocation::Graveyard(P0),
        ZonePosition::Bottom,
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(P0)), &ids);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(ids[0]).unwrap().zone, Zone::Graveyard);
}

#[test]
fn nether_shadow_does_not_return_from_exile_or_hand() {
    for zone in [ZoneLocation::Exile(P0), ZoneLocation::Hand(P0)] {
        let (mut e, ids) = setup(&eligible(), P0);
        move_to(&mut e, ids[0], zone, ZonePosition::Top);
        upkeep(&mut e, P0);
        assert_eq!(triggers(&e, ids[0]), 0);
    }
}

#[test]
fn nether_shadow_gaining_the_third_card_after_upkeep_began_does_not_trigger_late() {
    let (mut e, ids) = setup(&[nether_shadow(), llanowar_elves(), llanowar_elves()], P0);
    upkeep(&mut e, P0);
    assert_eq!(triggers(&e, ids[0]), 0);
    let ooze = on_battlefield(&e, P0, index::SCAVENGING_OOZE).unwrap();
    move_to(&mut e, ooze, ZoneLocation::Graveyard(P0), ZonePosition::Top);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    assert_eq!(triggers(&e, ids[0]), 0);
    assert_eq!(e.state().object(ids[0]).unwrap().zone, Zone::Graveyard);
}

#[test]
fn nether_shadow_returns_untapped_and_can_attack_that_turn() {
    let (mut e, ids) = setup(&eligible(), P0);
    upkeep(&mut e, P0);
    choose_return(&mut e, P0, true);
    assert!(!is_tapped(&e, ids[0]));
    assert_eq!(pt(&e, ids[0]), (1, 1));
    assert!(keywords(&e, ids[0]).contains(KeywordSet::HASTE));
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending() else {
        unreachable!()
    };
    assert!(attackers.contains(&ids[0]));
    let before = e.state().players[1].life;
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(ids[0], Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().players[1].life, before - 1);
}

#[test]
fn nether_shadow_four_copies_do_not_chain_more_triggers_during_the_same_upkeep() {
    let (mut e, ids) = setup(&[nether_shadow(); 4], P0);
    upkeep(&mut e, P0);
    assert_eq!(triggers(&e, ids[0]), 1);
    for &id in &ids[1..] {
        assert_eq!(triggers(&e, id), 0);
    }
    choose_return(&mut e, P0, true);
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(P0)), &ids[1..]);
    move_to(
        &mut e,
        ids[0],
        ZoneLocation::Graveyard(P0),
        ZonePosition::Top,
    );
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    for id in ids {
        assert_eq!(triggers(&e, id), 0);
    }
}

#[test]
fn nether_shadow_real_wrath_allows_its_owner_to_choose_whether_three_creatures_are_above() {
    for shadow_on_top in [false, true] {
        let mut e = Duel::new(1131, forest())
            .battlefield(
                0,
                &[
                    nether_shadow(),
                    llanowar_elves(),
                    llanowar_elves(),
                    llanowar_elves(),
                ],
            )
            .hand(0, &[index::WRATH_OF_GOD])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        let shadow = on_battlefield(&e, P0, nether_shadow()).unwrap();
        let elves = all_on_battlefield(&e, P0, llanowar_elves());
        let wrath = in_hand(&e, P0, index::WRATH_OF_GOD).unwrap();
        e.dev_state_mut(P0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::White, 4);
        e.refresh_offer();
        e.apply(P0, PlayerAction::CastSpell { card: wrath })
            .unwrap();
        pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
        let Pending::Arrange {
            player,
            cards,
            piles,
            prompt,
        } = e.pending()
        else {
            unreachable!()
        };
        assert_eq!(*player, P0);
        assert_eq!(*prompt, ArrangePrompt::Order);
        assert_eq!(cards.len(), 4);
        assert!(cards.contains(&shadow));
        assert_eq!(piles.len(), 1);
        assert_eq!(piles[0].place, ArrangePlace::Graveyard);
        assert!(piles[0].ordered);
        assert_eq!(
            e.state().object(wrath).unwrap().zone,
            Zone::Stack,
            "the resolving spell enters the graveyard after its destroyed creatures"
        );
        let mut order = elves;
        if shadow_on_top {
            order.insert(0, shadow);
        } else {
            order.push(shadow);
        }
        e.apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![order.clone()],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        let mut expected: Vec<_> = order.into_iter().rev().collect();
        expected.push(wrath);
        assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(P0)), &expected);
        upkeep(&mut e, P0);
        assert_eq!(triggers(&e, shadow), usize::from(!shadow_on_top));
        if !shadow_on_top {
            choose_return(&mut e, P0, true);
            assert_eq!(e.state().object(shadow).unwrap().zone, Zone::Battlefield);
        }
    }
}

#[test]
fn nether_shadow_return_is_an_untargeted_ability_not_a_counterspell_target() {
    let mut cards = eligible();
    cards.push(index::COUNTERSPELL);
    let (mut e, ids) = setup(&cards, P0);
    let counter = *ids.last().unwrap();
    move_to(&mut e, counter, ZoneLocation::Hand(P0), ZonePosition::Top);
    upkeep(&mut e, P0);
    let stack = e.state().zones.list(ZoneLocation::Stack);
    assert_eq!(stack.len(), 1);
    let trigger = e.state().object(stack[0]).unwrap();
    assert_eq!(trigger.kind, crate::object::ObjectKind::AbilityOnStack);
    assert!(trigger.targets.is_empty());
    e.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Blue, 2);
    e.refresh_offer();
    assert!(!priority_offer(&e).castable.contains(&counter));
    choose_return(&mut e, P0, true);
    assert_eq!(e.state().object(ids[0]).unwrap().zone, Zone::Battlefield);
}
