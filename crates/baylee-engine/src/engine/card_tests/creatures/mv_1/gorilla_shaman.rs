//! `cards/creatures/mv_1/gorilla_shaman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn gorilla_shaman() -> CardIndex {
    card_index("9f53b691-da24-4264-a2a0-d0b95a385963")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

#[test]
fn gorilla_shaman_destroys_noncreature_artifact_with_mana_value_x() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                gorilla_shaman(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
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
    engine
        .apply(p0, PlayerAction::ChooseNumber(1))
        .unwrap();
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
