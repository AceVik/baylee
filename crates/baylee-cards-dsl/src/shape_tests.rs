//! What a card file's faces say about the card: which kind of card it is,
//! which face a player may play, and whose abilities it has.

use super::*;
use baylee_core::generated::subtypes::{artifact, creature, enchantment};
use baylee_core::types::TypeSet;

fn face(types: TypeSet) -> FaceDef {
    FaceDef {
        types,
        castable_from_hand: true,
        ..FaceDef::DEFAULT
    }
}

fn card(faces: Vec<FaceDef>) -> CardDef {
    CardDef {
        faces: Box::leak(faces.into_boxed_slice()),
        ..CardDef::DEFAULT
    }
}

fn activated(mana: ManaCost, zone: ActivationZone, effects: &'static [Effect]) -> AbilityDef {
    AbilityDef::Activated {
        cost: Cost { mana, parts: &[] },
        effects,
        targets: None,
        second_targets: None,
        third_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: effects == ANY_COLOR_MANA,
        zone,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }
}

fn abilities(list: Vec<AbilityDef>) -> &'static [AbilityDef] {
    Box::leak(list.into_boxed_slice())
}

fn lands(card: &CardDef) -> Vec<usize> {
    card.land_faces_from_hand().collect()
}

#[test]
fn a_front_land_is_played_from_hand_and_a_back_land_only_when_the_card_is_modal() {
    // CR 712.6a and 712.9.
    let land = TypeSet::LAND;
    let spell = TypeSet::SORCERY;
    let back = |castable| FaceDef {
        castable_from_hand: castable,
        ..face(land)
    };
    assert_eq!(lands(&card(vec![face(land)])), [0]);
    assert_eq!(lands(&card(vec![face(spell), back(true)])), [1]);
    assert_eq!(lands(&card(vec![face(land), back(true)])), [0, 1]);
    // A transforming back is reached by turning over, never by a land drop.
    assert!(lands(&card(vec![face(spell), back(false)])).is_empty());
    assert_eq!(lands(&card(vec![face(land), back(false)])), [0]);
    assert!(lands(&card(vec![face(spell), face(spell)])).is_empty());
    assert!(lands(&card(vec![])).is_empty());
    // The front is playable whatever its own flag says.
    let mut front = face(land);
    front.castable_from_hand = false;
    assert_eq!(lands(&card(vec![front])), [0]);
    // A land creature is a land.
    assert_eq!(lands(&card(vec![face(land.union(TypeSet::CREATURE))])), [0]);
}

#[test]
fn a_split_card_is_two_halves_of_instants_and_sorceries() {
    // CR 709.1.
    let pair = |left, right| card(vec![left, right]).is_split_spell_card();
    let fire = || face(TypeSet::INSTANT);
    let ice = || face(TypeSet::SORCERY);
    assert!(pair(fire(), ice()));
    assert!(pair(ice(), fire()));

    assert!(!card(vec![fire()]).is_split_spell_card());
    assert!(!card(vec![fire(), ice(), fire()]).is_split_spell_card());
    assert!(!pair(face(TypeSet::CREATURE), ice()));
    assert!(!pair(fire(), face(TypeSet::LAND)));
    assert!(!pair(
        fire(),
        face(TypeSet::INSTANT.union(TypeSet::CREATURE))
    ));
    assert!(!pair(fire(), face(TypeSet::EMPTY)));

    let adventure = |mut half: FaceDef| {
        half.adventure = true;
        half
    };
    assert!(!pair(fire(), adventure(ice())));
    assert!(!pair(adventure(fire()), ice()));
}

#[test]
fn an_aftermath_half_is_split_only_behind_a_half_that_is_cast_from_hand() {
    // CR 702.127a: the second half is cast from the graveyard.
    let hand = |types| face(types);
    let grave = || FaceDef {
        castable_from_hand: false,
        aftermath: true,
        ..face(TypeSet::SORCERY)
    };
    let nowhere = || FaceDef {
        castable_from_hand: false,
        ..face(TypeSet::SORCERY)
    };
    assert!(card(vec![hand(TypeSet::INSTANT), grave()]).is_split_spell_card());
    assert!(!card(vec![grave(), hand(TypeSet::INSTANT)]).is_split_spell_card());
    assert!(!card(vec![hand(TypeSet::INSTANT), nowhere()]).is_split_spell_card());
    assert!(!card(vec![nowhere(), hand(TypeSet::INSTANT)]).is_split_spell_card());
}

#[test]
fn a_room_is_two_doors_and_one_door_is_no_room() {
    // CR 709.5.
    let door = || FaceDef {
        subtypes: &[enchantment::ROOM],
        ..face(TypeSet::ENCHANTMENT)
    };
    let plain = || face(TypeSet::ENCHANTMENT);
    assert!(card(vec![door(), door()]).has_shared_type_line());
    assert!(card(vec![door(), door()]).is_split_spell_card());
    for no_room in [
        vec![door()],
        vec![door(), plain()],
        vec![plain(), door()],
        vec![door(), door(), door()],
        vec![],
    ] {
        let no_room = card(no_room);
        assert!(!no_room.has_shared_type_line());
        assert!(!no_room.is_split_spell_card());
    }
}

const LEFT: &[Effect] = &[Effect::TransformSource];
const RIGHT: &[Effect] = &[Effect::TransformSource, Effect::TransformSource];

#[test]
fn a_room_has_the_abilities_of_the_doors_that_are_open() {
    let of = |effects| {
        abilities(vec![activated(
            ManaCost::ZERO,
            ActivationZone::Battlefield,
            effects,
        )])
    };
    let (left, right) = (of(LEFT), of(RIGHT));
    let both = abilities(left.iter().chain(right).copied().collect());
    let room = CardDef {
        abilities: both,
        ..card(vec![
            FaceDef {
                abilities: left,
                ..face(TypeSet::ENCHANTMENT)
            },
            FaceDef {
                abilities: right,
                ..face(TypeSet::ENCHANTMENT)
            },
        ])
    };
    assert!(room.door_abilities(0).is_empty());
    assert_eq!(room.door_abilities(0b01), left);
    assert_eq!(room.door_abilities(0b10), right);
    assert_eq!(room.door_abilities(0b11), both);
    // Only the two low bits are doors.
    assert!(room.door_abilities(0b100).is_empty());
    assert_eq!(room.door_abilities(0b101), left);
}

#[test]
fn a_back_face_never_inherits_the_fronts_abilities() {
    let front = abilities(vec![activated(
        ManaCost::ZERO,
        ActivationZone::Battlefield,
        &[Effect::TransformSource],
    )]);
    let shared = CardDef {
        abilities: front,
        ..card(vec![face(TypeSet::ARTIFACT), face(TypeSet::ARTIFACT)])
    };
    assert_eq!(shared.abilities_for_face(0), front);
    assert!(shared.abilities_for_face(1).is_empty());
    // A front with a list of its own does not read the card's.
    let own = abilities(vec![]);
    let listed = abilities(vec![activated(
        ManaCost::ZERO,
        ActivationZone::Hand,
        &[Effect::TransformSource],
    )]);
    let split = CardDef {
        abilities: front,
        ..card(vec![
            FaceDef {
                abilities: listed,
                ..face(TypeSet::ARTIFACT)
            },
            FaceDef {
                abilities: own,
                ..face(TypeSet::ARTIFACT)
            },
        ])
    };
    assert_eq!(split.abilities_for_face(0), listed);
    assert!(split.abilities_for_face(1).is_empty());
}

#[test]
fn a_permanent_is_kept_back_for_what_it_does_on_the_battlefield_besides_mana() {
    const TWO: ManaCost = baylee_core::mana!("{2}");
    const EFFECT: &[Effect] = &[Effect::TransformSource];
    let wanted = |list: Vec<AbilityDef>| {
        CardDef {
            abilities: abilities(list),
            ..card(vec![face(TypeSet::ARTIFACT)])
        }
        .use_beyond_mana(0)
    };
    let at = |mana, zone| activated(mana, zone, EFFECT);
    assert_eq!(wanted(vec![]), UseBeyondMana::Nothing);
    assert_eq!(
        wanted(vec![at(TWO, ActivationZone::Battlefield)]),
        UseBeyondMana::Paid
    );
    assert_eq!(
        wanted(vec![at(ManaCost::ZERO, ActivationZone::Battlefield)]),
        UseBeyondMana::Free
    );
    assert_eq!(
        wanted(vec![at(ManaCost::ZERO, ActivationZone::BattlefieldOwner)]),
        UseBeyondMana::Free
    );
    // The cheapest use decides, in either order.
    for list in [
        vec![
            at(TWO, ActivationZone::Battlefield),
            at(ManaCost::ZERO, ActivationZone::Battlefield),
        ],
        vec![
            at(ManaCost::ZERO, ActivationZone::Battlefield),
            at(TWO, ActivationZone::Battlefield),
        ],
    ] {
        assert_eq!(wanted(list), UseBeyondMana::Free);
    }
    // Tapping it for mana costs an ability used from elsewhere nothing.
    for zone in [
        ActivationZone::Hand,
        ActivationZone::Graveyard,
        ActivationZone::Command,
    ] {
        assert_eq!(wanted(vec![at(TWO, zone)]), UseBeyondMana::Nothing);
    }
    // A mana ability is what it is tapped for (CR 605.1a).
    assert_eq!(
        wanted(vec![activated(
            ManaCost::ZERO,
            ActivationZone::Battlefield,
            ANY_COLOR_MANA
        )]),
        UseBeyondMana::Nothing
    );
}

#[test]
fn generic_partner_is_a_keyword_of_the_front_face_alone() {
    let pair = |partner| CardDef {
        partner,
        ..card(vec![face(TypeSet::CREATURE), face(TypeSet::CREATURE)])
    };
    let partner = pair(PartnerKind::Partner);
    assert!(partner.keywords_for_face(0).contains(KeywordSet::PARTNER));
    assert!(!partner.keywords_for_face(1).contains(KeywordSet::PARTNER));
    assert!(partner.all_keywords().contains(KeywordSet::PARTNER));
    let single = pair(PartnerKind::None);
    assert!(!single.keywords_for_face(0).contains(KeywordSet::PARTNER));
    assert!(!single.all_keywords().contains(KeywordSet::PARTNER));
}

#[test]
fn a_face_with_keywords_of_its_own_does_not_take_the_cards() {
    let flying = CardDef {
        keywords: KeywordSet::FLYING,
        ..card(vec![
            face(TypeSet::CREATURE),
            FaceDef {
                keywords: KeywordSet::TRAMPLE,
                ..face(TypeSet::CREATURE)
            },
        ])
    };
    assert_eq!(flying.keywords_for_face(0), KeywordSet::FLYING);
    assert_eq!(flying.keywords_for_face(1), KeywordSet::TRAMPLE);
    assert_eq!(
        flying.all_keywords(),
        KeywordSet::FLYING.union(KeywordSet::TRAMPLE)
    );
    let own = CardDef {
        keywords: KeywordSet::FLYING,
        ..card(vec![FaceDef {
            keywords: KeywordSet::REACH,
            ..face(TypeSet::CREATURE)
        }])
    };
    assert_eq!(own.keywords_for_face(0), KeywordSet::REACH);
}

#[test]
fn the_predefined_tokens_are_what_the_rules_print() {
    // CR 111.10i and 701.53: Incubator, and the Phyrexian it turns into.
    let incubator = &INCUBATOR_TOKEN;
    assert_eq!(incubator.name, "Incubator");
    assert_eq!(incubator.types, TypeSet::ARTIFACT);
    assert_eq!(incubator.subtypes, [artifact::INCUBATOR]);
    assert!(incubator.colors.is_empty());
    assert_eq!((incubator.power, incubator.toughness), (None, None));
    let [
        AbilityDef::Activated {
            cost,
            effects,
            zone,
            mana_ability,
            ..
        },
    ] = incubator.abilities
    else {
        panic!("{:?}", incubator.abilities)
    };
    assert_eq!(cost.mana, baylee_core::mana!("{2}"));
    assert!(cost.parts.is_empty());
    assert_eq!(*effects, [Effect::TransformSource]);
    assert_eq!(*zone, ActivationZone::Battlefield);
    assert!(!mana_ability);

    let phyrexian = incubator.back_face.expect("it transforms");
    assert_eq!(phyrexian.name, "Phyrexian");
    assert_eq!(phyrexian.types, TypeSet::ARTIFACT.union(TypeSet::CREATURE));
    assert_eq!(phyrexian.subtypes, [creature::PHYREXIAN]);
    assert!(phyrexian.colors.is_empty());
    assert_eq!((phyrexian.power, phyrexian.toughness), (Some(0), Some(0)));
    assert!(phyrexian.abilities.is_empty());

    // CR 702.181a (mobilize), 702.135a (afterlife), 701.63a (endure).
    let warrior = &WARRIOR_1_1_RED;
    assert_eq!(warrior.name, "Warrior");
    assert_eq!(warrior.types, TypeSet::CREATURE);
    assert_eq!(warrior.subtypes, [creature::WARRIOR]);
    assert_eq!(warrior.colors, ColorSet::from_slice(&[Color::Red]));
    assert_eq!((warrior.power, warrior.toughness), (Some(1), Some(1)));
    assert!(warrior.keywords.is_empty());

    let afterlife = &SPIRIT_1_1_WHITE_BLACK_FLYING;
    assert_eq!(afterlife.name, "Spirit");
    assert_eq!(afterlife.types, TypeSet::CREATURE);
    assert_eq!(afterlife.subtypes, [creature::SPIRIT]);
    assert_eq!(
        afterlife.colors,
        ColorSet::from_slice(&[Color::White, Color::Black])
    );
    assert_eq!((afterlife.power, afterlife.toughness), (Some(1), Some(1)));
    assert_eq!(afterlife.keywords, KeywordSet::FLYING);

    let endure = &SPIRIT_X_X_WHITE;
    assert_eq!(endure.name, "Spirit");
    assert_eq!(endure.types, TypeSet::CREATURE);
    assert_eq!(endure.subtypes, [creature::SPIRIT]);
    assert_eq!(endure.colors, ColorSet::from_slice(&[Color::White]));
    assert_eq!((endure.power, endure.toughness), (None, None));
}

const VOTE: Effect = Effect::Vote {
    options: &[
        VoteOption {
            name: "time",
            effects: &[],
        },
        VoteOption {
            name: "knowledge",
            effects: &[],
        },
    ],
    outcome: VoteOutcome::AllWinners,
    secret: false,
};

#[test]
fn a_vote_is_named_wherever_the_face_prints_it() {
    // CR 701.38.
    let named = |list: Vec<AbilityDef>, face_index, option| {
        CardDef {
            abilities: abilities(list),
            ..card(vec![face(TypeSet::SORCERY), face(TypeSet::SORCERY)])
        }
        .vote_option_name(face_index, option)
    };
    let plain = || {
        activated(
            ManaCost::ZERO,
            ActivationZone::Battlefield,
            &[Effect::TransformSource, VOTE],
        )
    };
    let none = || {
        activated(
            ManaCost::ZERO,
            ActivationZone::Battlefield,
            &[Effect::TransformSource],
        )
    };
    let modal = || AbilityDef::ModalActivated {
        cost: Cost::FREE,
        modes: &[
            SpellMode {
                effects: &[Effect::TransformSource],
                targets: None,
                second_targets: None,
                cost_override: None,
                additional_cost: None,
            },
            SpellMode {
                effects: &[VOTE],
                targets: None,
                second_targets: None,
                cost_override: None,
                additional_cost: None,
            },
        ],
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        condition: None,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    };
    for list in [vec![plain()], vec![none(), plain()], vec![modal()]] {
        assert_eq!(named(list.clone(), 0, 0), Some("time"));
        assert_eq!(named(list.clone(), 0, 1), Some("knowledge"));
        assert_eq!(named(list.clone(), 0, 2), None);
        // The back face prints no vote.
        assert_eq!(named(list, 1, 0), None);
    }
    assert_eq!(named(vec![none()], 0, 0), None);
    assert_eq!(named(vec![], 0, 0), None);
}

#[test]
fn every_extra_target_cost_on_a_face_is_paid_together() {
    const ONE: ManaCost = baylee_core::mana!("{1}");
    const RED: ManaCost = baylee_core::mana!("{R}");
    let cost = |list: Vec<AbilityDef>| {
        CardDef {
            abilities: abilities(list),
            ..card(vec![face(TypeSet::INSTANT)])
        }
        .per_extra_target_cost(0)
    };
    assert_eq!(cost(vec![]), None);
    assert_eq!(cost(vec![AbilityDef::PerExtraTargetCost(ONE)]), Some(ONE));
    assert_eq!(
        cost(vec![
            AbilityDef::PerExtraTargetCost(ONE),
            AbilityDef::PerExtraTargetCost(RED)
        ]),
        Some(ONE.combine(&RED))
    );
}

#[test]
fn a_face_down_spell_is_turned_up_for_the_cost_its_own_keyword_prints() {
    // CR 702.37e, 702.37b (megamorph), 702.168 (disguise).
    const ONE: ManaCost = baylee_core::mana!("{1}");
    const TWO: ManaCost = baylee_core::mana!("{2}");
    const THREE: ManaCost = baylee_core::mana!("{3}");
    let of = |disguise, megamorph, morph| FaceDef {
        disguise,
        morph,
        megamorph,
        ..face(TypeSet::CREATURE)
    };
    let up = |f: &FaceDef| f.turn_face_up_cost().map(|(mana, _, mega)| (mana, mega));
    let plain = of(None, None, None);
    assert!(!plain.has_face_down_cast());
    assert_eq!(up(&plain), None);
    for (printed, expected) in [
        (of(Some(ONE), None, None), (ONE, false)),
        (of(None, Some(TWO), None), (TWO, true)),
        (of(None, None, Some(THREE)), (THREE, false)),
    ] {
        assert!(printed.has_face_down_cast());
        assert_eq!(up(&printed), Some(expected));
    }
}

#[test]
fn casualty_asks_for_a_creature_of_at_least_the_printed_power() {
    // CR 702.153a.
    for power in [0, 1, 2, 127, 255] {
        let Filter::And(clauses) = casualty_creature_filter(power) else {
            panic!("casualty {power}")
        };
        assert_eq!(
            *clauses,
            [Filter::CREATURE, Filter::PowerAtLeast(i16::from(power))]
        );
    }
}

#[test]
fn taking_keywords_away_leaves_exactly_the_others() {
    let two = KeywordSet::FLYING.union(KeywordSet::TRAMPLE);
    assert_eq!(two.difference(KeywordSet::FLYING), KeywordSet::TRAMPLE);
    assert_eq!(two.difference(KeywordSet::REACH), two);
    assert_eq!(two.difference(two), KeywordSet::EMPTY);
    assert_eq!(
        KeywordSet::EMPTY.difference(KeywordSet::FLYING),
        KeywordSet::EMPTY
    );
    assert_eq!(
        two.bits(),
        KeywordSet::FLYING.bits() | KeywordSet::TRAMPLE.bits()
    );
    assert_eq!(KeywordSet::EMPTY.bits(), 0);
    assert_ne!(KeywordSet::FLYING.bits(), KeywordSet::TRAMPLE.bits());
}

#[test]
fn an_energy_cost_adds_up_and_never_wraps_into_a_discount() {
    let pays = |parts: &'static [CostPart], x| {
        Cost {
            mana: ManaCost::ZERO,
            parts,
        }
        .energy_payment(x)
    };
    assert_eq!(pays(&[], 7), 0);
    assert_eq!(pays(&[CostPart::PayEnergy(3)], 7), 3);
    assert_eq!(pays(&[CostPart::PayEnergyX], 7), 7);
    assert_eq!(
        pays(
            &[
                CostPart::PayEnergy(3),
                CostPart::PayEnergyX,
                CostPart::PayEnergyX
            ],
            7
        ),
        17
    );
    assert_eq!(
        pays(&[CostPart::PayEnergyX, CostPart::PayEnergy(1)], u32::MAX),
        u32::MAX
    );
}

#[test]
fn only_a_controllers_constraint_binds_a_target_group() {
    let target = TargetReq::one(TargetSpec::Object(&Filter::CREATURE));
    assert_eq!(target.constraint(), None);
    assert_eq!(target.chosen_by_opponent().constraint(), None);
    for constraint in [
        TargetConstraint::SameController,
        TargetConstraint::OnePerPlayer,
    ] {
        assert_eq!(
            target.with_constraint(constraint).constraint(),
            Some(constraint)
        );
    }
}

const UP: &Amount = &Amount::Fixed(2);
const DOWN: &Amount = &Amount::NegX;

#[test]
fn a_conditional_amount_counts_down_only_when_both_branches_do() {
    let either = |then, otherwise| {
        Amount::IfCondition {
            condition: &Condition::OptionalCostPaid,
            then,
            otherwise,
        }
        .is_negative()
    };
    assert!(either(DOWN, DOWN));
    assert!(!either(DOWN, UP));
    assert!(!either(UP, DOWN));
    assert!(!either(UP, UP));
    assert!(Amount::Negated(UP).is_negative());
    assert!(!Amount::Negated(DOWN).is_negative());
}

#[test]
fn a_found_card_goes_where_its_search_says_and_as_it_says() {
    use crate::effect::{Find, SearchDest};
    for (find, dest) in [
        (Find::HAND, SearchDest::Hand),
        (Find::EXILE, SearchDest::Exile),
        (Find::EXILE_LINKED, SearchDest::Exile),
        (Find::GRAVEYARD, SearchDest::Graveyard),
        (Find::BATTLEFIELD, SearchDest::Battlefield),
        (Find::BATTLEFIELD_TAPPED, SearchDest::Battlefield),
        (Find::TOP_OF_LIBRARY, SearchDest::TopOfLibrary),
    ] {
        assert_eq!(find.dest, dest);
        assert_eq!(find.tapped, find == Find::BATTLEFIELD_TAPPED);
        assert_eq!(find.linked_exile, find == Find::EXILE_LINKED);
        assert!(!find.repeats);
    }
    let any = Find::BATTLEFIELD_TAPPED.any_number();
    assert!(any.repeats && any.tapped);
    assert_eq!(any.dest, SearchDest::Battlefield);
    let instead = Find::HAND.when_matching(&Filter::CREATURE, &Find::BATTLEFIELD);
    assert_eq!(
        instead.instead_if,
        Some((&Filter::CREATURE, &Find::BATTLEFIELD))
    );
    assert_eq!(instead.dest, SearchDest::Hand);
}

#[test]
fn a_comparison_holds_below_at_and_above_exactly_as_it_reads() {
    use AmountCmp::{AtLeast, AtMost, Equal, GreaterThan, LessThan, NotEqual};
    for (cmp, below, at, above) in [
        (AtLeast, false, true, true),
        (GreaterThan, false, false, true),
        (AtMost, true, true, false),
        (LessThan, true, false, false),
        (Equal, false, true, false),
        (NotEqual, true, false, true),
    ] {
        assert_eq!(
            [cmp.holds(4, 5), cmp.holds(5, 5), cmp.holds(6, 5)],
            [below, at, above],
            "{cmp:?}"
        );
    }
}

#[test]
fn an_eminence_trigger_functions_in_both_zones_and_the_others_in_one() {
    // CR 113.6b.
    for (zone, battlefield, command) in [
        (TriggerZone::Battlefield, true, false),
        (TriggerZone::Command, false, true),
        (TriggerZone::BattlefieldOrCommand, true, true),
    ] {
        assert_eq!(zone.functions_on_battlefield(), battlefield, "{zone:?}");
        assert_eq!(zone.functions_in_command(), command, "{zone:?}");
    }
}

#[test]
fn a_saga_chapter_is_read_with_its_numbers_and_nothing_else_is_one() {
    let chapter = |chapter| AbilityDef::SagaChapter {
        chapter,
        effects: &[],
        targets: None,
    };
    for number in [1, 2, 5] {
        let chapter = chapter(number);
        assert_eq!(chapter.saga_chapters(), [number]);
        assert!(chapter.is_saga_chapter());
        assert_eq!(chapter.max_saga_chapter(), number);
    }
    let plain = activated(ManaCost::ZERO, ActivationZone::Battlefield, &[]);
    assert!(plain.saga_chapters().is_empty());
    assert!(!plain.is_saga_chapter());
    assert_eq!(plain.max_saga_chapter(), 0);
}

#[test]
fn a_modal_spell_says_its_modes_its_count_and_whether_they_repeat() {
    const MODES: &[SpellMode] = &[SpellMode {
        effects: &[Effect::TransformSource],
        targets: None,
        second_targets: None,
        cost_override: None,
        additional_cost: None,
    }];
    let spell = AbilityDef::ModalSpell {
        modes: MODES,
        choose: ModeCount::TWO,
    };
    assert_eq!(spell.modal_spell(), Some((MODES, ModeCount::TWO, false)));
    for can_repeat_modes in [false, true] {
        let modal = AbilityDef::Modal {
            modes: MODES,
            choose: ModeCount::THREE,
            can_repeat_modes,
        };
        assert_eq!(
            modal.modal_spell(),
            Some((MODES, ModeCount::THREE, can_repeat_modes))
        );
    }
    let plain = activated(ManaCost::ZERO, ActivationZone::Battlefield, &[]);
    assert_eq!(plain.modal_spell(), None);

    assert!(ModeCount::ONE.is_one());
    for count in [
        ModeCount::TWO,
        ModeCount::ONE_OR_MORE,
        ModeCount { min: 0, max: 1 },
        ModeCount { min: 2, max: 1 },
    ] {
        assert!(!count.is_one(), "{count:?}");
    }
}

#[test]
fn a_mana_bundle_is_two_or_more_decided_manas_for_no_mana() {
    use crate::manaread::{mana_bundle, tap_only};
    use baylee_core::mana::ManaColor::{Blue, White};
    const W: Effect = Effect::mana(White, 1);
    const U: Effect = Effect::mana(Blue, 1);
    const TAP: Cost = Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf],
    };
    const PRICED: Cost = Cost {
        mana: baylee_core::mana!("{1}"),
        parts: &[CostPart::TapSelf],
    };
    const LIFE: Cost = Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf, CostPart::PayLife(1)],
    };
    assert_eq!(mana_bundle(&TAP, &[W, U]), Some(vec![White, Blue]));
    assert_eq!(
        mana_bundle(&TAP, &[W, W, U]),
        Some(vec![White, White, Blue])
    );
    // A rider changes no mana, so it is no reason to refuse.
    assert_eq!(
        mana_bundle(&TAP, &[W, Effect::TransformSource, U]),
        Some(vec![White, Blue])
    );
    // One mana is another reader's question.
    assert_eq!(mana_bundle(&TAP, &[W]), None);
    assert_eq!(mana_bundle(&TAP, &[]), None);
    // A count that would be wrong is refused, never guessed.
    assert_eq!(mana_bundle(&PRICED, &[W, U]), None);
    assert_eq!(mana_bundle(&TAP, &[W, Effect::mana(Blue, 2)]), None);
    assert_eq!(mana_bundle(&TAP, &[W, Effect::mana_of_any_color()]), None);
    // A draw moves the library: this is no mana ability at all.
    assert_eq!(mana_bundle(&TAP, &[W, U, Effect::draw(1)]), None);

    assert!(tap_only(&TAP));
    assert!(tap_only(&Cost::FREE));
    assert!(!tap_only(&PRICED));
    assert!(!tap_only(&LIFE));
}
