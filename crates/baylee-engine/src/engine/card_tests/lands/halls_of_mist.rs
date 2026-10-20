//! `cards/lands/halls_of_mist.rs`, played.
//!
//! Oracle:
//! - "Cumulative upkeep {1}"
//! - "Creatures that attacked during their controller's last turn can't attack."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn halls_of_mist() -> CardIndex {
    card_index("57d7deaf-ee08-49ab-b239-1c77a1f65225")
}

#[test]
fn halls_of_mist_cumulative_upkeep_and_attack_restriction() {
    let mut engine = lk::game_with(
        SEED,
        &[halls_of_mist(), index::GRIZZLY_BEARS],
        &[],
        &[],
        &[],
        |e| {
            lk::float(e, P0, &[(ManaColor::Colorless, 1)]);
            lk::yes_no(e, true);
        },
    );
    let halls = lk::obj(&engine, P0, halls_of_mist());
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    assert_eq!(
        engine
            .state()
            .object(halls)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        1
    );

    // Attack with Grizzly Bears on P0's turn.
    let _ = lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    // Advance to P0's next turn, paying the {2} cumulative upkeep.
    for _ in 0..80 {
        if engine.state().turn.active == P0
            && engine.state().turn.number > 1
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && matches!(engine.pending(), Pending::Priority { .. })
        {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { .. } => {
                lk::float(&mut engine, P0, &[(ManaColor::Colorless, 2)]);
                lk::yes_no(&mut engine, true);
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }

    let offered = lk::attackers_offered(&mut engine);
    assert!(
        !offered.contains(&bears),
        "creature that attacked during its controller's last turn cannot attack while Halls of Mist is on the battlefield"
    );
}
