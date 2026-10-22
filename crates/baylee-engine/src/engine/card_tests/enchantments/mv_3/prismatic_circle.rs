//! `cards/enchantments/mv_3/prismatic_circle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "022dae2e-7fc3-486e-9168-59652d9ab21c"
fn prismatic_circle() -> CardIndex {
    card_index("022dae2e-7fc3-486e-9168-59652d9ab21c")
}

#[test]
fn prismatic_circle_chooses_a_color_and_prevents_damage_from_a_chosen_source_of_that_color() {
    let mut engine = lk::game(SEED, &[], &[index::HILL_GIANT], &[prismatic_circle()], &[]);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 3), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, prismatic_circle());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut engine);

    let selected = baylee_core::ids::DamageSourceRef {
        object: giant,
        version: engine.state().object(giant).unwrap().version,
    };

    activate(&mut engine, P0, prismatic_circle(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        options, choice, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(options, vec![selected]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: selected,
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(engine.state().shields.len(), 1);
}
