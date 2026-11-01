#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn jhoira_s_toolbox() -> CardIndex {
    card_index("a08475a6-1fac-4a3f-b343-aed16a61bc16")
}

#[test]
fn jhoira_s_toolbox_regenerates_target_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), jhoira_s_toolbox()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let toolbox = on_battlefield(&engine, p0, jhoira_s_toolbox()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: toolbox,
                ability_index: 0,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![toolbox],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![toolbox],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, jhoira_s_toolbox()).is_some());
    assert!(is_tapped(&engine, toolbox));
}

#[test]
fn jhoira_s_toolbox_does_not_regenerate_plain_creatures_or_noncreature_artifacts() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(
        SEED,
        &[jhoira_s_toolbox(), gray_ogre(), sol_ring()],
        &[],
        &[],
        &[],
    );
    lk::float(&mut engine, p0, &[(ManaColor::Colorless, 2)]);
    activate(&mut engine, p0, jhoira_s_toolbox(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!options.contains(&lk::obj(&engine, p0, gray_ogre())));
    assert!(!options.contains(&lk::obj(&engine, p0, sol_ring())));
    let toolbox = lk::obj(&engine, p0, jhoira_s_toolbox());
    lk::aim(&mut engine, &[toolbox], &[]);
    lk::settle(&mut engine);
}
