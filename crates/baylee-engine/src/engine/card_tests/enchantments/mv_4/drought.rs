//! `cards/enchantments/mv_4/drought.rs`, played.
//!
//! Oracle:
//! - "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}."
//! - "Spells cost an additional 'Sacrifice a Swamp' to cast for each black mana symbol in their mana costs."
//! - "Activated abilities cost an additional 'Sacrifice a Swamp' to activate for each black mana symbol in their activation costs."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "ecba74bd-850e-4cbe-9b1c-b0d4d6f27d1c"
fn drought() -> CardIndex {
    card_index("ecba74bd-850e-4cbe-9b1c-b0d4d6f27d1c")
}

/// Keep the starting Drought through its first-upkeep trigger.
fn pay_upkeep(engine: &mut Engine<RegistryLookup>) {
    lk::float(engine, P0, &[(ManaColor::White, 2)]);
    lk::yes_no(engine, true);
}

#[track_caller]
fn sacrifice_options(engine: &Engine<RegistryLookup>, player: PlayerId) -> Vec<ObjectId> {
    let Pending::ChooseCards {
        player: asked,
        options,
        min,
        max,
        prompt: ChoicePrompt::CostSacrifice,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a sacrifice cost, got {:?}", engine.pending());
    };
    assert_eq!((asked, min, max), (player, 1, 1));
    options
}

#[track_caller]
fn sacrifice(engine: &mut Engine<RegistryLookup>, player: PlayerId, object: ObjectId) {
    assert!(sacrifice_options(engine, player).contains(&object));
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![object],
            },
        )
        .expect("the offered permanent pays the sacrifice cost");
}

#[test]
fn drought_requires_paying_two_white_at_your_upkeep_or_is_sacrificed() {
    let mut engine = lk::game(SEED, &[], &[], &[drought()], &[]);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut engine, P0, drought());
    pass_until(&mut engine, stack_is_empty);

    // Advance to P0's next upkeep and pay {W}{W}.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("pay {W}{W} to keep Drought");
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        on_battlefield(&engine, P0, drought()).is_some(),
        "Drought remains on the battlefield when {{W}}{{W}} is paid"
    );

    // Advance to P0's following upkeep and decline paying {W}{W}: Drought is sacrificed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(P0, PlayerAction::YesNo(false))
        .expect("decline paying {W}{W}");
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        on_battlefield(&engine, P0, drought()).is_none(),
        "Drought is sacrificed when {{W}}{{W}} is not paid"
    );
}

#[test]
fn drought_charges_dark_ritual_one_swamp_and_the_ritual_makes_three_black() {
    let mut engine = lk::game_with(
        SEED,
        &[drought(), swamp(), forest()],
        &[],
        &[index::DARK_RITUAL],
        &[],
        pay_upkeep,
    );
    let sw = lk::obj(&engine, P0, swamp());
    let forest = lk::obj(&engine, P0, forest());
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);

    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    assert_eq!(
        sacrifice_options(&engine, P0),
        vec![sw],
        "only a Swamp pays the additional cost"
    );
    sacrifice(&mut engine, P0, sw);
    assert_eq!(lk::pool_total(&engine, P0), 0, "the printed {{B}} was paid");
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, sw), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, forest), Zone::Battlefield);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Black), 3);
}

#[test]
fn drought_does_not_tax_a_spell_without_black_mana_symbols() {
    let mut engine = lk::game_with(
        SEED,
        &[drought(), swamp()],
        &[],
        &[index::SAVANNAH_LIONS],
        &[],
        pay_upkeep,
    );
    let sw = lk::obj(&engine, P0, swamp());
    lk::float(&mut engine, P0, &[(ManaColor::White, 1)]);

    cast_with_floating(&mut engine, P0, index::SAVANNAH_LIONS);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "a white spell asks no sacrifice question"
    );
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, sw), Zone::Battlefield);
    assert!(on_battlefield(&engine, P0, index::SAVANNAH_LIONS).is_some());
}

#[test]
fn drought_adds_two_distinct_swamps_to_carrions_printed_creature_sacrifice() {
    let mut engine = lk::game_with(
        SEED,
        &[drought(), index::GRIZZLY_BEARS, swamp(), swamp()],
        &[],
        &[index::CARRION],
        &[],
        pay_upkeep,
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let swamps = all_on_battlefield(&engine, P0, swamp());
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
    );

    cast_with_floating(&mut engine, P0, index::CARRION);
    assert_eq!(
        sacrifice_options(&engine, P0),
        vec![bears],
        "Carrion's printed creature cost comes first"
    );
    sacrifice(&mut engine, P0, bears);
    assert_eq!(sacrifice_options(&engine, P0), swamps);
    sacrifice(&mut engine, P0, swamps[0]);
    assert_eq!(
        sacrifice_options(&engine, P0),
        vec![swamps[1]],
        "the first Swamp cannot pay for the second black symbol"
    );
    sacrifice(&mut engine, P0, swamps[1]);
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
    assert!(
        swamps
            .iter()
            .all(|id| lk::zone_of(&engine, *id) == Zone::Graveyard)
    );
    // "The sacrificed creature" is the printed cost's, not the last Swamp's
    // (CR 607.2i): the Bears' power of 2, two Insects.
    let insects = engine
        .state()
        .battlefield_seen()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|object| {
                object.controller == P0
                    && object
                        .characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
            })
        })
        .count();
    assert_eq!(insects, 2);
}

/// The same receipt on an activation: Starlit Sanctum's {B} is taxed one Swamp,
/// and the life lost is still the sacrificed Cleric's power (CR 607.2i).
#[test]
fn drought_leaves_an_activations_sacrificed_creature_to_its_printed_cost() {
    let mut engine = lk::game_with(
        SEED,
        &[
            drought(),
            index::STARLIT_SANCTUM,
            index::WAR_PRIEST_OF_THUNE,
            swamp(),
        ],
        &[],
        &[],
        &[],
        pay_upkeep,
    );
    let sanctum = lk::obj(&engine, P0, index::STARLIT_SANCTUM);
    let cleric = lk::obj(&engine, P0, index::WAR_PRIEST_OF_THUNE);
    let sw = lk::obj(&engine, P0, swamp());
    let power = pt(&engine, cleric).0;
    assert!(power > 0, "a Cleric whose power can be told from a Swamp's");
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    let before = engine.state().players[1].life;

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: sanctum,
                ability_index: 2,
            },
        )
        .unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![P1],
            },
        )
        .unwrap();
    sacrifice(&mut engine, P0, cleric);
    sacrifice(&mut engine, P0, sw);
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().players[1].life,
        before - i32::from(power),
        "the Swamp the rule imposed is no sacrificed creature"
    );
}

#[test]
fn drought_charges_mire_shades_printed_and_imposed_swamps_distinctly() {
    let mut engine = lk::game_with(
        SEED,
        &[drought(), index::MIRE_SHADE, swamp(), swamp()],
        &[],
        &[],
        &[],
        pay_upkeep,
    );
    let shade = lk::obj(&engine, P0, index::MIRE_SHADE);
    let swamps = all_on_battlefield(&engine, P0, swamp());
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: shade,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(sacrifice_options(&engine, P0), swamps);
    sacrifice(&mut engine, P0, swamps[0]);
    assert_eq!(sacrifice_options(&engine, P0), vec![swamps[1]]);
    sacrifice(&mut engine, P0, swamps[1]);
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, shade), (2, 2));
    assert!(
        swamps
            .iter()
            .all(|id| lk::zone_of(&engine, *id) == Zone::Graveyard)
    );
    assert_eq!(lk::pool_total(&engine, P0), 0);
}

#[test]
fn drought_preserves_dark_rituals_sacrifice_through_the_mana_window() {
    let mut engine = lk::game_with(
        SEED,
        &[drought(), swamp()],
        &[],
        &[index::DARK_RITUAL],
        &[],
        pay_upkeep,
    );
    let sw = lk::obj(&engine, P0, swamp());
    let ritual = in_hand(&engine, P0, index::DARK_RITUAL).unwrap();

    engine
        .apply(P0, PlayerAction::CastSpell { card: ritual })
        .expect("Dark Ritual can make its mana while being cast");
    sacrifice(&mut engine, P0, sw);
    assert_eq!(engine.payment_window().map(|(player, _)| player), Some(P0));
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: sw })
        .expect("a chosen Swamp is not sacrificed before the mana window");
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, sw), Zone::Graveyard);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Black), 3);
}

#[test]
fn drought_refuses_unpayable_spells_and_activations_without_changing_state() {
    let mut engine = lk::game_with(
        SEED,
        &[drought(), index::MIRE_SHADE, swamp()],
        &[],
        &[index::CARRION],
        &[],
        pay_upkeep,
    );
    let shade = lk::obj(&engine, P0, index::MIRE_SHADE);
    let carrion = in_hand(&engine, P0, index::CARRION).unwrap();
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
    );

    assert!(!lk::castable(&engine, P0, index::CARRION));
    let before = engine.snapshot_hash();
    assert!(
        engine
            .apply(P0, PlayerAction::CastSpell { card: carrion })
            .is_err()
    );
    assert_eq!(engine.snapshot_hash(), before);
    assert_eq!(lk::pool_total(&engine, P0), 3);

    assert!(!lk::offered(&engine, index::MIRE_SHADE, P0, 0));
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: shade,
                    ability_index: 0,
                },
            )
            .is_err()
    );
    assert_eq!(engine.snapshot_hash(), before);
    assert_eq!(lk::zone_of(&engine, shade), Zone::Battlefield);
    assert_eq!(
        lk::zone_of(&engine, lk::obj(&engine, P0, swamp())),
        Zone::Battlefield
    );
}

#[test]
fn two_droughts_charge_dark_ritual_two_swamps() {
    let mut engine = lk::game_with(
        SEED,
        &[drought(), drought(), swamp(), swamp()],
        &[],
        &[index::DARK_RITUAL],
        &[],
        pay_upkeep,
    );
    let swamps = all_on_battlefield(&engine, P0, swamp());
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);

    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    sacrifice(&mut engine, P0, swamps[0]);
    assert_eq!(sacrifice_options(&engine, P0), vec![swamps[1]]);
    sacrifice(&mut engine, P0, swamps[1]);
    lk::settle(&mut engine);

    assert!(
        swamps
            .iter()
            .all(|id| lk::zone_of(&engine, *id) == Zone::Graveyard)
    );
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Black), 3);
}

#[test]
fn destroying_drought_ends_its_spell_and_activation_costs() {
    let mut engine = lk::game_with(
        SEED,
        &[drought(), index::MIRE_SHADE, swamp()],
        &[],
        &[index::DISENCHANT, index::DARK_RITUAL],
        &[],
        pay_upkeep,
    );
    let drought = lk::obj(&engine, P0, drought());
    let shade = lk::obj(&engine, P0, index::MIRE_SHADE);
    let sw = lk::obj(&engine, P0, swamp());
    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
            (ManaColor::Black, 2),
        ],
    );

    lk::cast_at(&mut engine, P0, index::DISENCHANT, &[drought], &[]);
    assert_eq!(lk::zone_of(&engine, drought), Zone::Graveyard);

    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: shade,
                ability_index: 0,
            },
        )
        .unwrap();
    sacrifice(&mut engine, P0, sw);
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, shade), (2, 2));

    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
    lk::settle(&mut engine);
    assert_eq!(lk::pool_of(&engine, P0, ManaColor::Black), 3);
}

#[test]
fn drought_taxes_the_opponents_spells_and_activated_abilities() {
    let mut engine = lk::game_with(
        SEED,
        &[drought()],
        &[index::ORDER_OF_THE_EBON_HAND, swamp(), swamp()],
        &[],
        &[index::DARK_RITUAL],
        pay_upkeep,
    );
    let order = lk::obj(&engine, P1, index::ORDER_OF_THE_EBON_HAND);
    let swamps = all_on_battlefield(&engine, P1, swamp());
    lk::pass_once(&mut engine);
    lk::float(&mut engine, P1, &[(ManaColor::Black, 1)]);

    cast_with_floating(&mut engine, P1, index::DARK_RITUAL);
    sacrifice(&mut engine, P1, swamps[0]);
    lk::settle(&mut engine);
    assert_eq!(lk::pool_of(&engine, P1, ManaColor::Black), 3);

    lk::pass_once(&mut engine);
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: order,
                ability_index: 1,
            },
        )
        .unwrap();
    sacrifice(&mut engine, P1, swamps[1]);
    lk::settle(&mut engine);

    assert!(
        swamps
            .iter()
            .all(|id| lk::zone_of(&engine, *id) == Zone::Graveyard)
    );
    assert_eq!(lk::pool_of(&engine, P1, ManaColor::Black), 2);
    assert!(
        engine
            .state()
            .object(order)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );
}
