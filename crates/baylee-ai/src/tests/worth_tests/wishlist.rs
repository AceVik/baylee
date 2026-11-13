//! Wishlist loyalty decisions and their contextual effect values.
use super::*;
use baylee_cards_dsl::Effect;
use baylee_engine::choice::ChoicePrompt;

fn walker_face(name: &str, face: u8, loyalty: u16) -> PublicObject {
    let mut object = carded(walker(obj(1), ME, loyalty), name, TypeSet::PLANESWALKER);
    object.card.as_mut().unwrap().face = face;
    object.rules.as_mut().unwrap().face = face;
    object
}

fn loyalty_effects(name: &str, face: usize, ability: usize) -> &'static [Effect] {
    let index = baylee_cards::decks::by_name(name).unwrap();
    let AbilityDef::Loyalty { effects, .. } = &baylee_cards::by_index(index)
        .unwrap()
        .abilities_for_face(face)[ability]
    else {
        panic!("expected a loyalty ability")
    };
    effects
}

#[test]
fn vivien_selects_exile_with_a_library_and_pluses_when_it_is_empty() {
    let vivien = walker_face("Vivien, Champion of the Wilds", 0, 4);
    let mut v = view(0, &[20, 20], vec![vivien]);
    main_phase(&mut v);
    let choices = offering(vec![(obj(1), 1), (obj(1), 2)]);
    v.seats[0].library_count = 10;
    assert_eq!(expert().act(&v, &choices), activates(1, 2));
    v.seats[0].library_count = 0;
    assert_eq!(expert().act(&v, &choices), activates(1, 1));
}

#[test]
fn vivien_privately_exiles_a_creature_or_a_legal_fallback() {
    let mut v = view(0, &[20, 20], vec![]);
    v.looking_at = vec![
        card(10, ME, "Forest", TypeSet::LAND),
        card(11, ME, "Grizzly Bears", TypeSet::CREATURE),
        card(12, ME, "Lightning Bolt", TypeSet::INSTANT),
    ];
    let question = |options| Pending::ChooseCards {
        player: ME,
        options,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::Generic,
        total: None,
    };
    let context = asked("Vivien, Champion of the Wilds", 2, obj(1));
    assert_eq!(
        expert().act_with_context(&v, &question(vec![obj(10), obj(11), obj(12)]), &context),
        PlayerAction::ChooseObjects {
            objects: vec![obj(11)]
        }
    );
    let PlayerAction::ChooseObjects { objects } =
        expert().act_with_context(&v, &question(vec![obj(10), obj(12)]), &context)
    else {
        panic!("the required exile must choose a card")
    };
    assert_eq!(objects.len(), 1);
    assert!([obj(10), obj(12)].contains(&objects[0]));
}

#[test]
fn grist_pluses_with_no_creatures_to_copy_and_ultimates_with_several() {
    let grist = walker_face("Grist, Voracious Larva", 1, 7);
    let mut v = view(0, &[20, 20], vec![grist]);
    main_phase(&mut v);
    v.seats[0].library_count = 20;
    let choices = offering(vec![(obj(1), 0), (obj(1), 2)]);
    assert_eq!(expert().act(&v, &choices), activates(1, 0));
    v.graveyards[0] = (10..15)
        .map(|id| card(id, ME, "Grizzly Bears", TypeSet::CREATURE))
        .collect();
    assert_eq!(expert().act(&v, &choices), activates(1, 2));
}

#[test]
fn grist_copy_value_uses_the_copy_size_and_only_own_creature_cards() {
    let effects = loyalty_effects("Grist, Voracious Larva", 1, 2);
    let mut v = view(0, &[20, 20], vec![]);
    let origin = Origin {
        id: obj(1),
        object: None,
    };
    let worth = |view: &PlayerView| {
        expert()
            .effects_worth(view, origin, effects, Aim::Source, 0)
            .unwrap()
    };
    let empty = worth(&v);
    let mut bear = card(10, ME, "Grizzly Bears", TypeSet::CREATURE);
    bear.power = Some(2);
    bear.toughness = Some(2);
    v.graveyards[0].push(bear);
    let one = worth(&v);
    assert!(one > empty);
    v.graveyards[0][0].power = Some(20);
    v.graveyards[0][0].toughness = Some(20);
    assert_eq!(
        worth(&v),
        one,
        "the copy is a 1/1 regardless of the original size"
    );
    v.graveyards[0].push(card(11, ME, "Forest", TypeSet::LAND));
    v.graveyards[1].push(card(12, THEM, "Grizzly Bears", TypeSet::CREATURE));
    assert_eq!(
        worth(&v),
        one,
        "neither a land nor an opponent's creature is copied"
    );
    v.graveyards[0].push(card(13, ME, "Grizzly Bears", TypeSet::CREATURE));
    assert!(worth(&v) > one);
}

#[test]
fn grist_conditional_counter_values_only_a_previously_created_token() {
    let effects = loyalty_effects("Grist, Voracious Larva", 1, 0);
    let v = view(0, &[20, 20], vec![]);
    let origin = Origin {
        id: obj(1),
        object: None,
    };
    let worth = |effects: &[Effect]| {
        expert()
            .effects_worth(&v, origin, effects, Aim::Source, 0)
            .unwrap()
    };
    assert_eq!(
        worth(&effects[2..3]),
        0,
        "no token was created in this resolution"
    );
    let token = worth(&effects[..1]);
    assert!(token > 0);
    assert!(
        worth(&[effects[0], effects[2]]) > token,
        "the conditional counter retains the created-token context"
    );
}

#[test]
fn older_generations_keep_unread_effects_and_the_original_generic_choice() {
    let grist_plus = loyalty_effects("Grist, Voracious Larva", 1, 0);
    let Effect::IfCondition { then, .. } = grist_plus[2] else {
        panic!("the counter is conditional")
    };
    let effects = [
        loyalty_effects("Vivien, Champion of the Wilds", 0, 2),
        then,
        loyalty_effects("Grist, Voracious Larva", 1, 2),
    ];
    let mut v = view(0, &[20, 20], vec![]);
    v.seats[0].library_count = 10;
    v.looking_at = vec![
        card(10, ME, "Forest", TypeSet::LAND),
        card(11, ME, "Grizzly Bears", TypeSet::CREATURE),
    ];
    let origin = Origin {
        id: obj(1),
        object: None,
    };
    let choice = Pending::ChooseCards {
        player: ME,
        options: vec![obj(10), obj(11)],
        min: 1,
        max: 1,
        prompt: ChoicePrompt::Generic,
        total: None,
    };
    let context = asked("Vivien, Champion of the Wilds", 2, obj(1));
    for profile in [
        AIProfile::SHARP_2026_11,
        AIProfile::EXPERT.at_generation(AIProfile::TOKEN_AND_EXILE_CHOICES - 1),
    ] {
        let old = HeuristicAgent::new(profile);
        for effect in effects {
            assert_eq!(old.effects_worth(&v, origin, effect, Aim::Source, 0), None);
        }
        assert_eq!(
            old.act_with_context(&v, &choice, &context),
            PlayerAction::ChooseObjects {
                objects: vec![obj(10)]
            }
        );
    }
    for effect in effects {
        assert!(
            expert()
                .effects_worth(&v, origin, effect, Aim::Source, 0)
                .is_some()
        );
    }
    assert_eq!(
        expert().act_with_context(&v, &choice, &context),
        PlayerAction::ChooseObjects {
            objects: vec![obj(11)]
        }
    );
}
