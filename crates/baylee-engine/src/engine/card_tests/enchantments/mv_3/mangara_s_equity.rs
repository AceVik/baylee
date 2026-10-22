//! `cards/enchantments/mv_3/mangara_s_equity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_core::ids::Defender;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c7986014-30e0-4999-ab95-b86620471419"
fn mangara_s_equity() -> CardIndex {
    card_index("c7986014-30e0-4999-ab95-b86620471419")
}

#[test]
fn mangara_s_equity_deals_equal_damage_back_to_creature_of_chosen_color_that_damages_you() {
    let mut engine = lk::game(SEED, &[], &[index::HILL_GIANT], &[mangara_s_equity()], &[]);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut engine, P0, mangara_s_equity());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's combat and attack P0 with red Hill Giant (3/3).
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P0))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(engine.state().players[0].life, 17);
    assert!(
        on_battlefield(&engine, P1, index::HILL_GIANT).is_none(),
        "Mangara's Equity dealt 3 damage back to the red Hill Giant, destroying it"
    );
}
