//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

use crate::casting::{SpendFor, pay_mana_for, spendable_units};
use baylee_core::generated::subtypes::creature;
use baylee_core::mana::ManaCost;

fn settle_starting_land_choices(e: &mut Engine<RegistryLookup>) {
    loop {
        let (player, action) = match e.pending().clone() {
            Pending::ChooseSubtype { player, options } => (
                player,
                PlayerAction::ChooseSubtype(if options.contains(&creature::CONSTRUCT) {
                    creature::CONSTRUCT
                } else {
                    options[0]
                }),
            ),
            Pending::ChooseColor { player, .. } => {
                (player, PlayerAction::ChooseColor(ManaColor::Green))
            }
            Pending::ChooseCards {
                player,
                prompt: crate::choice::ChoicePrompt::RevealOrEnterTapped,
                ..
            } => (player, PlayerAction::ChooseObjects { objects: vec![] }),
            _ => break,
        };
        e.apply(player, action).unwrap();
    }
}

const TRIBAL_LANDS: &[(CardIndex, u32, u32)] = &[
    (index::ELDRAZI_TEMPLE, 1, 2),
    (index::CRUCIBLE_OF_THE_SPIRIT_DRAGON, 2, 4),
    (index::SECLUDED_COURTYARD, 1, 1),
    (index::PRIMAL_BEYOND, 1, 1),
    (index::BASE_CAMP, 1, 1),
    (index::GALLIFREY_COUNCIL_CHAMBER, 2, 1),
    (index::ECLIPSED_REALMS, 1, 1),
    (index::CASTLE_GARENBRIG, 1, 6),
];

#[test]
fn tribal_mana_pays_matching_spells_and_real_activations_but_no_unrelated_costs() {
    let p0 = PlayerId::new(0);
    for &(land, ability, produced) in TRIBAL_LANDS {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    land,
                    index::WALKING_BALLISTA,
                    index::GLORIOUS_ANTHEM,
                    index::MASKWOOD_NEXUS,
                    index::MIND_STONE,
                ],
            )
            .hand(0, &[index::ORNITHOPTER, index::MIND_STONE])
            .start();
        keep_mulligans(&mut e);
        settle_starting_land_choices(&mut e);
        reach_main_phase(&mut e, p0);
        let source = on_battlefield(&e, p0, land).unwrap();
        let ballista = on_battlefield(&e, p0, index::WALKING_BALLISTA).unwrap();
        let stone = on_battlefield(&e, p0, index::MIND_STONE).unwrap();
        let spell = in_hand(&e, p0, index::ORNITHOPTER).unwrap();
        let noncreature = in_hand(&e, p0, index::MIND_STONE).unwrap();
        {
            let state = e.dev_state_mut(p0).unwrap();
            state.set_tapped(source, false);
            let object = state.object_mut(source).unwrap();
            object.counters.add(counters::STORAGE, 4);
            if land == index::CASTLE_GARENBRIG {
                state.players[0].mana_pool.add(ManaColor::Green, 4);
            }
        }
        e.refresh_offer();
        activate(&mut e, p0, land, ability);
        if matches!(e.pending(), Pending::ChooseNumber { .. }) {
            e.apply(p0, PlayerAction::ChooseNumber(4)).unwrap();
        }
        while matches!(e.pending(), Pending::ChooseColor { .. }) {
            e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
                .unwrap();
        }
        assert_eq!(
            e.state().players[0].mana_pool.total(),
            u64::from(produced),
            "land {land:?}"
        );
        assert_eq!(
            spendable_units(e.state(), p0, SpendFor::Spell(spell)),
            produced
        );
        assert_eq!(
            spendable_units(e.state(), p0, SpendFor::Ability(ballista)),
            produced
        );
        for what in [
            SpendFor::Other,
            SpendFor::Ability(stone),
            SpendFor::Spell(noncreature),
        ] {
            let before = e.state().players[0].mana_pool.clone();
            assert!(
                pay_mana_for(
                    e.dev_state_mut(p0).unwrap(),
                    p0,
                    what,
                    &ManaCost::parse("{1}")
                )
                .is_none(),
                "land {land:?}"
            );
            assert_eq!(
                e.state().players[0].mana_pool,
                before,
                "failed payment is atomic"
            );
        }
        let top_up = 4u32.saturating_sub(produced);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, top_up);
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(legal.abilities.contains(&(ballista, 0)), "land {land:?}");
        activate(&mut e, p0, index::WALKING_BALLISTA, 0);
        assert_eq!(
            e.state().players[0].mana_pool.total(),
            u64::from(produced + top_up - 4)
        );
        pass_until(&mut e, stack_is_empty);
        assert_eq!(counters_on(&e, ballista, CounterKind::P1P1), 1);
    }
}

#[test]
fn citadel_pays_for_a_land_sources_channel_from_hand_not_a_land_spell_or_ordinary_payment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::SUNKEN_CITADEL])
        .battlefield(1, &[index::MIND_STONE])
        .hand(0, &[index::BOSEIJU_WHO_ENDURES, index::ORNITHOPTER])
        .start();
    keep_mulligans(&mut e);
    settle_starting_land_choices(&mut e);
    reach_main_phase(&mut e, p0);
    let citadel = on_battlefield(&e, p0, index::SUNKEN_CITADEL).unwrap();
    let boseiju = in_hand(&e, p0, index::BOSEIJU_WHO_ENDURES).unwrap();
    let victim = on_battlefield(&e, p1, index::MIND_STONE).unwrap();
    {
        let state = e.dev_state_mut(p0).unwrap();
        state.set_tapped(citadel, false);
        state.object_mut(citadel).unwrap().chosen_color = Some(ManaColor::Green);
    }
    e.refresh_offer();
    activate(&mut e, p0, index::SUNKEN_CITADEL, 1);
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Ability(boseiju)),
        2
    );
    assert_eq!(spendable_units(e.state(), p0, SpendFor::Spell(boseiju)), 0);
    assert_eq!(spendable_units(e.state(), p0, SpendFor::Other), 0);
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: boseiju,
            ability_index: 1,
        },
    )
    .unwrap();
    aim_at(&mut e, p0, victim);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert_eq!(e.state().object(boseiju).unwrap().zone, Zone::Graveyard);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    e.apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(victim).unwrap().zone, Zone::Graveyard);
}

#[test]
fn floated_courtyard_mana_keeps_its_chosen_type_when_the_land_returns_with_a_new_choice() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::SECLUDED_COURTYARD,
                index::WALKING_BALLISTA,
                index::GLORIOUS_ANTHEM,
            ],
        )
        .hand(0, &[index::LLANOWAR_ELVES])
        .start();
    keep_mulligans(&mut e);
    settle_starting_land_choices(&mut e);
    reach_main_phase(&mut e, p0);
    let land = on_battlefield(&e, p0, index::SECLUDED_COURTYARD).unwrap();
    let ballista = on_battlefield(&e, p0, index::WALKING_BALLISTA).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(land)
        .unwrap()
        .chosen_subtype = Some(creature::CONSTRUCT);
    e.refresh_offer();
    activate(&mut e, p0, index::SECLUDED_COURTYARD, 1);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .move_object(
            land,
            ZoneLocation::Hand(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    e.apply(p0, PlayerAction::PlayLand { card: land }).unwrap();
    e.apply(p0, PlayerAction::ChooseSubtype(creature::ELF))
        .unwrap();
    let elf = in_hand(&e, p0, index::LLANOWAR_ELVES).unwrap();
    assert_eq!(spendable_units(e.state(), p0, SpendFor::Spell(elf)), 0);
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Ability(ballista)),
        1
    );
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    activate(&mut e, p0, index::WALKING_BALLISTA, 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, ballista, CounterKind::P1P1), 1);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

/// Untaps `source` with four storage counters on it and taps it for its
/// restricted mana through printed ability `ability`, answering whatever it asks.
fn float_restricted_mana(
    e: &mut Engine<RegistryLookup>,
    source: ObjectId,
    land: CardIndex,
    ability: u32,
) {
    let p0 = PlayerId::new(0);
    {
        let state = e.dev_state_mut(p0).unwrap();
        state.set_tapped(source, false);
        state
            .object_mut(source)
            .unwrap()
            .counters
            .add(counters::STORAGE, 4);
    }
    e.refresh_offer();
    activate(e, p0, land, ability);
    if matches!(e.pending(), Pending::ChooseNumber { .. }) {
        e.apply(p0, PlayerAction::ChooseNumber(4)).unwrap();
    }
    while matches!(e.pending(), Pending::ChooseColor { .. }) {
        e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
            .unwrap();
    }
}

/// The creature-type restriction of a tribal land, with no Maskwood Nexus on
/// the board to give every creature every type (the sweep above has one, so
/// there the restriction cannot be told from "any creature"). The land's mana
/// pays for a spell and an ability of each named creature type, and for
/// nothing of a Bear or a Construct, nor for a payment that is neither.
fn tribal_mana_without_a_nexus(
    land: CardIndex,
    ability: u32,
    produced: u32,
    kinds: &[(CardIndex, baylee_core::ids::SubtypeId)],
    permanent: (CardIndex, baylee_core::ids::SubtypeId),
) {
    let p0 = PlayerId::new(0);
    let mut hand = vec![index::GRIZZLY_BEARS];
    hand.extend(kinds.iter().map(|(card, _)| *card));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                land,
                index::WALKING_BALLISTA,
                index::GLORIOUS_ANTHEM,
                permanent.0,
            ],
        )
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut e);
    settle_starting_land_choices(&mut e);
    reach_main_phase(&mut e, p0);
    let source = on_battlefield(&e, p0, land).unwrap();
    let ballista = on_battlefield(&e, p0, index::WALKING_BALLISTA).unwrap();
    let member = on_battlefield(&e, p0, permanent.0).unwrap();
    let bears = in_hand(&e, p0, index::GRIZZLY_BEARS).unwrap();
    float_restricted_mana(&mut e, source, land, ability);
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        u64::from(produced),
        "the land made its restricted mana"
    );

    let subtypes_of = |id| e.state().object(id).unwrap().characteristics().subtypes;
    assert!(subtypes_of(member).contains(permanent.1));
    assert!(!subtypes_of(bears).contains(permanent.1));
    assert!(!subtypes_of(ballista).contains(permanent.1));
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Ability(member)),
        produced,
        "an ability of a real {:?} may be paid with it",
        permanent.1
    );
    for (card, subtype) in kinds {
        let spell = in_hand(&e, p0, *card).unwrap();
        assert!(
            subtypes_of(spell).contains(*subtype),
            "{card:?} is a {subtype:?}"
        );
        assert_eq!(
            spendable_units(e.state(), p0, SpendFor::Spell(spell)),
            produced,
            "a {subtype:?} spell may be paid with it"
        );
    }
    for (what, label) in [
        (SpendFor::Spell(bears), "a Grizzly Bears spell"),
        (
            SpendFor::Ability(ballista),
            "an ability of Walking Ballista",
        ),
        (SpendFor::Other, "a payment that is no spell or ability"),
    ] {
        assert_eq!(
            spendable_units(e.state(), p0, what),
            0,
            "{label}: none of it is spendable there"
        );
        let before = e.state().players[0].mana_pool.clone();
        assert!(
            pay_mana_for(
                e.dev_state_mut(p0).unwrap(),
                p0,
                what,
                &ManaCost::parse("{1}")
            )
            .is_none(),
            "{label}: cannot be paid with it"
        );
        assert_eq!(e.state().players[0].mana_pool, before, "atomic refusal");
    }
}

#[test]
fn crucible_of_the_spirit_dragon_mana_is_for_real_dragons_only() {
    tribal_mana_without_a_nexus(
        index::CRUCIBLE_OF_THE_SPIRIT_DRAGON,
        2,
        4,
        &[(index::SHIVAN_DRAGON, creature::DRAGON)],
        (index::SHIVAN_DRAGON, creature::DRAGON),
    );
}

#[test]
fn base_camp_mana_is_for_clerics_rogues_warriors_and_wizards_only() {
    tribal_mana_without_a_nexus(
        index::BASE_CAMP,
        1,
        1,
        &[
            (index::DEVOUT_MONK, creature::CLERIC),
            (index::OPPOSITION_AGENT, creature::ROGUE),
            (index::GOBLIN_SKY_RAIDER, creature::WARRIOR),
            (index::THUNDERSCAPE_APPRENTICE, creature::WIZARD),
        ],
        (index::DEVOUT_MONK, creature::CLERIC),
    );
}

#[test]
fn primal_beyond_mana_is_for_real_elementals_only() {
    tribal_mana_without_a_nexus(
        index::PRIMAL_BEYOND,
        1,
        1,
        &[(index::FLAMEKIN_HARBINGER, creature::ELEMENTAL)],
        (index::BOGARDAN_FIREFIEND, creature::ELEMENTAL),
    );
}
