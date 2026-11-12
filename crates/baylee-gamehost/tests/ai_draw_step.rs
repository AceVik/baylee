//! Nothing begun in the seat's own draw step (`AIProfile::QUIET_DRAW_STEP`,
//! generation 13), on a table a real engine dealt.

use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_engine::choice::PlayerAction;
use baylee_engine::turn::Step;

use crate::house_table::Table;

/// A level that does not time its instants (casual, its noise taken out so
/// that its own choice is the one read) holds a Giant Growth in its draw
/// step beside its Grizzly Bears, where at generation 12 it cast it on
/// sight, with no combat for it to turn.
#[test]
fn an_instant_waits_out_the_draw_step() {
    let quiet = AIProfile {
        temperature_milli: 0,
        ..AIProfile::CASUAL
    };
    for (generation, waits) in [
        (AIProfile::GENERATION, true),
        (AIProfile::EQUIPMENT_AND_X, false),
    ] {
        let mut table = Table::open(
            "Forest",
            &["Giant Growth"],
            &["Forest", "Grizzly Bears"],
            &[],
        );
        table.play_to_my(Step::Draw, 3);
        let view = table.view();
        let pending = table.engine.pending().clone();
        let action = HeuristicAgent::new(quiet.at_generation(generation)).act_with_context(
            &view,
            &pending,
            &table.engine.decision_context(),
        );
        assert_eq!(
            action == PlayerAction::PassPriority,
            waits,
            "generation {generation}: {action:?}"
        );
    }
}
