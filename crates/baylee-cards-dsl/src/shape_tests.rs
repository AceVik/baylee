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
