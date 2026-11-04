//! Mana plans must follow the active board and the chosen spell form.

use super::{base_view, test_identity, test_public_object};
use crate::policy::{sources, spell_mana_increase};
use baylee_client_core::manaplan;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;
use baylee_engine::choice::{CastModeKind, LegalActions};
use baylee_view::ObjectStatus;

#[test]
fn inactive_bonus_abilities_cannot_make_an_unpayable_plan_reachable() {
    for (land, bonus, status, cost) in [
        ("Forest", "Mana Flare", ObjectStatus::PHASED_OUT, "{G}{G}"),
        (
            "Swamp",
            "Nirkana Revenant",
            ObjectStatus::NO_ABILITIES,
            "{B}{B}",
        ),
    ] {
        let mut view = base_view();
        let land = test_public_object(1, land, 0, None);
        let bonus = test_public_object(2, bonus, 0, None);
        let legal = LegalActions {
            mana_abilities: vec![land.id],
            ..LegalActions::default()
        };
        view.battlefield = vec![land, bonus];
        let cost = ManaCost::parse(cost);
        let plan = |view: &baylee_view::PlayerView| {
            manaplan::plan(&cost, &view.seats[0].mana_pool, &sources(view, &legal))
        };
        assert_eq!(plan(&view).map(|plan| plan.taps()), Some(1));
        view.battlefield[1].status = status;
        assert!(
            plan(&view).is_none(),
            "inactive {bonus} cannot pay {cost}",
            bonus = view.battlefield[1].name
        );
    }
}

#[test]
fn an_inactive_damage_trigger_cannot_hide_a_safe_mana_source() {
    let mut view = base_view();
    view.battlefield = vec![
        test_public_object(1, "Forest", 0, None),
        test_public_object(2, "Manabarbs", 1, None),
    ];
    view.seats[0].life = 1;
    let legal = LegalActions {
        mana_abilities: vec![view.battlefield[0].id],
        ..LegalActions::default()
    };
    assert!(
        sources(&view, &legal).is_empty(),
        "the active trigger is lethal"
    );
    view.battlefield[1].status = ObjectStatus::PHASED_OUT;
    let sources = sources(&view, &legal);
    assert_eq!(sources.len(), 1);
    assert!(!sources[0].priced);
}

#[test]
fn an_inactive_derelor_cannot_recolor_a_generic_tax() {
    let mut view = base_view();
    view.battlefield = vec![
        test_public_object(1, "Derelor", 0, None),
        test_public_object(2, "Sphere of Resistance", 0, None),
    ];
    let spell = ObjectId::new(50, 0);
    let legal = LegalActions {
        spell_increases: vec![(spell, CastModeKind::Normal, 1)],
        ..LegalActions::default()
    };
    for status in [ObjectStatus::PHASED_OUT, ObjectStatus::NO_ABILITIES] {
        view.battlefield[0].status = status;
        assert_eq!(
            spell_mana_increase(
                &view,
                spell,
                test_identity("Dark Ritual"),
                &legal,
                CastModeKind::Normal
            ),
            ManaCost::parse("{1}"),
        );
    }
}

#[test]
fn a_black_prototype_keeps_the_colored_tax_in_its_mana_plan() {
    let mut view = base_view();
    view.battlefield = vec![
        test_public_object(1, "Derelor", 0, None),
        test_public_object(2, "Swamp", 0, None),
        test_public_object(3, "Swamp", 0, None),
        test_public_object(4, "Forest", 0, None),
        test_public_object(5, "Forest", 0, None),
    ];
    let spell = ObjectId::new(50, 0);
    let legal = LegalActions {
        mana_abilities: view.battlefield[1..].iter().map(|o| o.id).collect(),
        spell_increases: vec![(spell, CastModeKind::Prototype, 1)],
        ..LegalActions::default()
    };
    let card = test_identity("Phyrexian Fleshgorger");
    let increase = spell_mana_increase(&view, spell, card, &legal, CastModeKind::Prototype);
    assert_eq!(increase, ManaCost::parse("{B}"));
    assert_eq!(
        spell_mana_increase(&view, spell, card, &legal, CastModeKind::Normal),
        ManaCost::ZERO
    );
    let cost = ManaCost::parse("{1}{B}{B}").combine(&increase);
    assert!(manaplan::plan(&cost, &view.seats[0].mana_pool, &sources(&view, &legal)).is_none());
    view.battlefield[4] = test_public_object(5, "Swamp", 0, None);
    assert_eq!(
        manaplan::plan(&cost, &view.seats[0].mana_pool, &sources(&view, &legal)).map(|p| p.taps()),
        Some(4)
    );
}

#[test]
fn a_noted_activation_offer_keeps_its_fixed_bundle() {
    let mut view = base_view();
    let mut cauldron = test_public_object(10, "Ice Cauldron", 0, None);
    cauldron.board_mana = Some(baylee_view::BoardMana {
        index: 1,
        colors: vec![
            baylee_core::mana::ManaColor::Red,
            baylee_core::mana::ManaColor::Red,
            baylee_core::mana::ManaColor::Colorless,
        ],
        restricted_to: Some(ObjectId::new(50, 0)),
    });
    let id = cauldron.id;
    view.battlefield = vec![cauldron];
    let legal = LegalActions {
        abilities: vec![(id, 1)],
        ..LegalActions::default()
    };
    let offers = crate::policy::offers(&view, &legal);
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].source.amount, 3);
    assert!(offers[0].source.bundle);
    assert_eq!(offers[0].source.colors.len(), 3);
    assert!(offers[0].only_for.is_some());
}

#[test]
fn phased_out_permanents_do_not_satisfy_or_block_casting_conditions() {
    use crate::policy::spell_can_be_cast_now;

    let mut view = base_view();
    let spell = ObjectId::new(50, 0);
    let blizzard = test_identity("Blizzard");
    // The engine would cast it once paid; what is asked here is the view.
    let legal = baylee_engine::choice::LegalActions {
        payable: vec![spell],
        ..Default::default()
    };
    view.battlefield = vec![test_public_object(1, "Snow-Covered Forest", 0, None)];
    assert!(spell_can_be_cast_now(&view, &legal, spell, blizzard));
    view.battlefield[0].status = ObjectStatus::PHASED_OUT;
    assert!(!spell_can_be_cast_now(&view, &legal, spell, blizzard));

    let tidal = test_identity("Tidal Influence");
    view.battlefield = vec![test_public_object(1, "Tidal Influence", 1, None)];
    assert!(!spell_can_be_cast_now(&view, &legal, spell, tidal));
    view.battlefield[0].status = ObjectStatus::PHASED_OUT;
    assert!(spell_can_be_cast_now(&view, &legal, spell, tidal));
}
