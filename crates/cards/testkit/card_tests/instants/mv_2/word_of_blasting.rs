//! Word of Blasting — `{1}{R}` Instant (Ice Age)
//!
//! Destroy target Wall. It can't be regenerated. Word of Blasting deals damage
//! equal to that Wall's mana value to the Wall's controller.

#[allow(clippy::wildcard_imports)]
use super::*;

fn word_of_blasting() -> CardIndex {
    card_index("8b391f24-992f-4cc0-ac4d-31456983fc03")
}

fn wall_of_bone() -> CardIndex {
    card_index("8ffe4986-9e09-4421-ad26-296a4c0df9e4")
}

#[test]
fn word_of_blasting_destroys_only_wall_without_regeneration_and_deals_mv_damage_to_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[word_of_blasting()])
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[wall_of_bone(), grizzly_bears(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p1, wall_of_bone()).unwrap();
    let non_wall = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let spell = in_hand(&engine, p0, word_of_blasting()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    // Non-Wall Grizzly Bears is excluded; only Wall of Bone can be targeted.
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(options, &[wall]);
    assert!(!options.contains(&non_wall));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .unwrap();

    // p1 responds by activating Wall of Bone's {B}: Regenerate ability.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(wall).unwrap().zone,
        Zone::Graveyard,
        "Wall of Bone is destroyed through its regeneration shield"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "Wall of Bone's controller takes damage equal to its mana value (3)"
    );
}
