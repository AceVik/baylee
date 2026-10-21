//! `cards/creatures/mv_5/illusionary_wall.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn illusionary_wall() -> CardIndex {
    card_index("77a8945f-4d52-4445-8913-425007e7948c")
}

/// Illusionary Wall (77a8945f-4d52-4445-8913-425007e7948c) — {4}{U} 7/4 Creature — Illusion Wall:
/// - "Defender, flying, first strike"
/// - "Cumulative upkeep {U}"
#[test]
fn illusionary_wall_has_defender_blocks_fliers_with_first_strike_and_scales_cumulative_upkeep() {
    let mut engine = lk::game_with(
        SEED,
        &[illusionary_wall(), index::GRIZZLY_BEARS, island(), island()],
        &[index::SHIVAN_DRAGON],
        &[],
        &[],
        |e| {
            lk::float(e, P0, &[(ManaColor::Blue, 1)]);
            lk::yes_no(e, true);
        },
    );
    let wall = lk::obj(&engine, P0, illusionary_wall());
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let dragon = lk::obj(&engine, P1, index::SHIVAN_DRAGON);

    assert_eq!(pt(&engine, wall), (7, 4));
    assert_eq!(
        engine
            .state()
            .object(wall)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    // Defender: Grizzly Bears is offered as an attacker, Illusionary Wall is not.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("stopped on ChooseAttackers")
    };
    assert!(attackers.contains(&bears));
    assert!(
        !attackers.contains(&wall),
        "defender prevents Illusionary Wall from attacking"
    );
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    // P1's turn: 5/5 flying Shivan Dragon attacks; Illusionary Wall blocks it in the air
    // and kills it in the first-strike combat damage step without taking lethal 5 back.
    reach_their_main_phase(&mut engine, P1);
    let offer = lk::attack(&mut engine, &[dragon]);
    assert!(
        lk::may_block(&offer, wall, dragon),
        "flying Illusionary Wall can block flying Shivan Dragon"
    );
    lk::block(&mut engine, &[(wall, dragon)]);
    lk::through_combat(&mut engine);
    assert!(
        in_graveyard(&engine, P1, index::SHIVAN_DRAGON).is_some(),
        "7 first-strike damage kills the 5/5 Shivan Dragon before regular combat damage"
    );
    assert!(
        on_battlefield(&engine, P0, illusionary_wall()).is_some(),
        "Illusionary Wall takes 0 damage back because Shivan Dragon died in the first-strike step"
    );

    // Second upkeep: 2 age counters, pay {U}{U}.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    assert_eq!(
        engine
            .state()
            .object(wall)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        2
    );
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    lk::yes_no(&mut engine, true);
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, P0, illusionary_wall()).is_some());

    // Third upkeep: decline -> sacrificed.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, illusionary_wall()).is_some());
}
