//! `cards/creatures/mv_4/flailing_drake.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn flailing_drake() -> CardIndex {
    card_index("93b6a170-a816-4556-aa27-a729231715e7")
}

/// Flailing Drake (`93b6a170-a816-4556-aa27-a729231715e7`) — {3}{G} Creature — Drake:
/// "Flying. Whenever this creature blocks or becomes blocked by a creature, that creature gets +1/+1 until end of turn."
#[test]
fn flailing_drake_pumps_creature_it_blocks_by_plus_one_plus_one() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::GRIZZLY_BEARS])
        .battlefield(1, &[flailing_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let drake = on_battlefield(&engine, P1, flailing_drake()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(drake, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 3));
}
