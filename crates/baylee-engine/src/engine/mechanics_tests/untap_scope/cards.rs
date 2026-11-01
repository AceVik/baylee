//! Real-card regressions share an untargeted control-change probe. The
//! printed ability, its cost, targets and untap restriction remain real.

use super::*;
use baylee_core::{generated::index, ids::Defender, mana::ManaColor};

pub(in crate::engine) fn assert_untaps_after_control_change(printed: CardIndex) {
    let mut engine = bench(
        7413,
        vec![
            card(
                VICTIM,
                creature_face("Victim", "{0}", 2, 2),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                STEWARD,
                face("Steward", "{0}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                STEWARDSHIP,
            ),
        ],
        [
            Seat::with(&[printed.get(), VICTIM, STEWARD]),
            Seat::with(&[STEWARD]),
        ],
    );
    reach_initial_main(&mut engine, printed);
    let source = the(&engine, ZoneLocation::Battlefield, printed.get());
    let affected = if printed == index::SAMITE_ALCHEMIST {
        the(&engine, ZoneLocation::Battlefield, VICTIM)
    } else {
        source
    };
    // Mana is fixture setup; the real activation pays its printed cost.
    engine.state.players[0].mana_pool.add(ManaColor::White, 2);
    engine.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    engine.refresh_offer();
    if printed == index::SPECTRAL_BEARS {
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        engine
            .apply(
                me(),
                PlayerAction::DeclareAttackers {
                    attackers: vec![(source, Defender::Player(them()))],
                },
            )
            .unwrap();
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        engine
            .apply(them(), PlayerAction::DeclareBlockers { blockers: vec![] })
            .unwrap();
        walk_until(&mut engine, |e| {
            e.state().turn.phase == Phase::SecondMain && e.state().zones.stack_is_empty()
        });
    } else {
        let ability = u32::from(printed == index::DEEP_SPAWN);
        activate(&mut engine, me(), source, ability);
        if printed == index::SAMITE_ALCHEMIST {
            engine
                .apply(
                    me(),
                    PlayerAction::ChooseTargets {
                        objects: vec![affected],
                        players: vec![],
                    },
                )
                .unwrap();
        } else if printed == index::REVEKA_WIZARD_SAVANT {
            engine
                .apply(
                    me(),
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![them()],
                    },
                )
                .unwrap();
        }
        settle(&mut engine, me());
    }
    assert!(
        tapped(&engine, affected),
        "the real ability or attack tapped the creature"
    );
    take(&mut engine, them(), affected);
    walk_until(&mut engine, |e| {
        e.state().turn.active == them() && e.state().turn.step == Step::Upkeep
    });
    assert!(
        !tapped(&engine, affected),
        "the new controller's untap is not the ability controller's next untap"
    );
}

fn reach_initial_main(engine: &mut Bench, printed: CardIndex) {
    // Deep Spawn can ask for its upkeep mill while the initial board starts.
    for _ in 0..100 {
        if holds_priority_in(engine, me(), Step::Main) {
            break;
        }
        let pending = engine.pending().clone();
        if let Pending::YesNo { player, .. } = pending {
            assert_eq!(printed, index::DEEP_SPAWN);
            engine.apply(player, PlayerAction::YesNo(true)).unwrap();
        } else {
            assert!(
                walk_past(engine, &pending),
                "unexpected question: {pending:?}"
            );
        }
    }
    assert!(holds_priority_in(engine, me(), Step::Main));
}
