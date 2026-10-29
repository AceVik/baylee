//! Sparring Golem — {3} — Artifact Creature — Golem
//! Oracle: Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it.

use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

#[test]
fn sparring_golem_gets_plus_one_plus_one_for_each_creature_blocking_it() {
    let golem = card_index("c54879da-0f1d-4ad0-b082-95f0e4d35685");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[golem])
        .battlefield(1, &[index::WALL_OF_STONE, index::WALL_OF_WOOD])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let golem_id = on_battlefield(&engine, p0, golem).expect("Sparring Golem on board");
    let w1 = on_battlefield(&engine, p1, index::WALL_OF_STONE).unwrap();
    let w2 = on_battlefield(&engine, p1, index::WALL_OF_WOOD).unwrap();
    assert_eq!(pt(&engine, golem_id), (2, 2));

    kit::declare_attack(&mut engine, &[golem_id], p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(w1, golem_id), (w2, golem_id)],
            },
        )
        .unwrap();
    kit::resolve_the_stack(&mut engine, |_, _| None);

    assert_eq!(pt(&engine, golem_id), (4, 4));
}
