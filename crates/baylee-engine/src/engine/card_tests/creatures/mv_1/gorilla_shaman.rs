//! `cards/creatures/mv_1/gorilla_shaman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn gorilla_shaman() -> CardIndex {
    card_index("9f53b691-da24-4264-a2a0-d0b95a385963")
}

fn ashnod_s_cylix() -> CardIndex {
    card_index("fdf4e4c3-fe17-4098-9f76-e98f3f00bd3e")
}

fn aesthir_glider() -> CardIndex {
    card_index("99dcde0b-4f4d-4d7a-89aa-8a89c256e38f")
}

fn floodwater_dam() -> CardIndex {
    card_index("a3947473-0f57-4937-bb1c-f20969144415")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

#[test]
fn gorilla_shaman_destroys_noncreature_artifact_with_mana_value_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[gorilla_shaman(), mountain(), mountain(), mountain()])
        .battlefield(1, &[sol_ring()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shaman = on_battlefield(&engine, p0, gorilla_shaman()).unwrap();
    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: shaman,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, sol_ring()).is_none(),
        "Gorilla Shaman with X=1 should destroy Sol Ring (mana value 1)"
    );
}

/// "Destroy target noncreature artifact with mana value X": with X = 3 only the {3} noncreature
/// artifact is a target, not a {1} or {2} one and not a {3} artifact creature; and it is the only
/// thing destroyed.
#[test]
fn gorilla_shaman_names_only_a_noncreature_artifact_of_exactly_the_chosen_mana_value() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                gorilla_shaman(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(
            1,
            &[
                sol_ring(),
                ashnod_s_cylix(),
                aesthir_glider(),
                floodwater_dam(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shaman = on_battlefield(&engine, p0, gorilla_shaman()).unwrap();
    let ring = on_battlefield(&engine, p1, sol_ring()).unwrap();
    let cylix = on_battlefield(&engine, p1, ashnod_s_cylix()).unwrap();
    let glider = on_battlefield(&engine, p1, aesthir_glider()).unwrap();
    let dam = on_battlefield(&engine, p1, floodwater_dam()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: shaman,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&dam), "mana value 3, noncreature");
    assert!(!options.contains(&ring), "mana value 1");
    assert!(!options.contains(&cylix), "mana value 2");
    assert!(!options.contains(&glider), "mana value 3 but a creature");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![dam],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, floodwater_dam()).is_none());
    for survivor in [ring, cylix, glider] {
        assert_eq!(
            engine.state().object(survivor).unwrap().zone,
            Zone::Battlefield
        );
    }
}
