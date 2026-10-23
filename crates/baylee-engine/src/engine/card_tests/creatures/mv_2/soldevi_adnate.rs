//! `cards/creatures/mv_2/soldevi_adnate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldevi_adnate() -> CardIndex {
    card_index("e3174070-542f-429e-9149-b572f6614537")
}

fn sengir_vampire() -> CardIndex {
    baylee_core::generated::index::SENGIR_VAMPIRE
}

fn soldevi_sentry() -> CardIndex {
    card_index("d2a61132-d881-4c80-92f1-a151d835832a")
}

fn grizzly_bears() -> CardIndex {
    baylee_core::generated::index::GRIZZLY_BEARS
}

/// Soldevi Adnate ({1}{B} 1/2 Creature — Human Cleric):
/// - "{T}, Sacrifice a black or artifact creature: Add an amount of {B} equal to the sacrificed creature's mana value."
#[test]
fn soldevi_adnate_sacrifices_black_creature_to_add_black_mana_equal_to_mana_value() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[soldevi_adnate(), sengir_vampire(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let adnate = on_battlefield(&engine, p0, soldevi_adnate()).unwrap();
    let vampire = on_battlefield(&engine, p0, sengir_vampire()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: adnate,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the cost asks which creature to sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert!(options.contains(&vampire), "a black creature: {options:?}");
    assert!(
        !options.contains(&bears),
        "a green non-artifact creature may not be sacrificed: {options:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears]
                }
            )
            .is_err(),
        "the Bears are not an answer to the question"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![vampire],
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, p0, sengir_vampire()).is_some());
    assert!(is_tapped(&engine, adnate));
    // Sengir Vampire has mana value 5: five {B}.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        5
    );
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}

/// "Sacrifice a black or artifact creature": a colorless artifact creature is
/// a legal sacrifice too, and its mana value (1) is the amount of {B}.
#[test]
fn soldevi_adnate_also_sacrifices_an_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[soldevi_adnate(), soldevi_sentry(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let adnate = on_battlefield(&engine, p0, soldevi_adnate()).unwrap();
    let sentry = on_battlefield(&engine, p0, soldevi_sentry()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: adnate,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "the cost asks which creature to sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&sentry),
        "an artifact creature: {options:?}"
    );
    assert!(!options.contains(&bears), "the Bears: {options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![sentry],
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, p0, soldevi_sentry()).is_some());
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1
    );
}
