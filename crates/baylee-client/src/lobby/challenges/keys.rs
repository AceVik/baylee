//! Semantic focus identities survive panel redraws and ordinary refreshes.
use super::{Action, State};
use crate::lobby::orders;
use crate::shellkit::focus::Stop;
use baylee_client_core::challenges::api::Board;

pub(super) fn stop(action: Action, model: &State) -> Stop {
    let (id, item) = match action {
        Action::Confirm => return Stop::new(orders::CHALLENGE_CONFIRM.name, "confirm"),
        Action::Cancel => return Stop::new(orders::CHALLENGE_CONFIRM.name, "cancel"),
        Action::Select(id) => {
            let live = |task: &&baylee_client_core::challenges::api::Challenge| {
                task.description.period.live_at(model.now)
            };
            let position = model
                .catalogue
                .iter()
                .filter(live)
                .chain(model.catalogue.iter().filter(|task| !live(task)))
                .position(|task| task.id == id)
                .unwrap_or(0);
            ("tasks", u8::try_from(position).unwrap_or(u8::MAX))
        }
        Action::Refresh => ("refresh", 0),
        Action::Back => ("back", 0),
        Action::Older => ("older", 0),
        Action::Play => ("play", 0),
        Action::Abandon => ("abandon", 0),
        Action::Hint => ("hint", 0),
        Action::Deck => ("deck", 0),
        Action::Rules => ("rules", 0),
        Action::Visibility => ("visibility", 0),
        Action::Board(board) => (
            "boards",
            match board {
                Board::FirstWin => 0,
                Board::BestTurns => 1,
                Board::Sprint => 2,
            },
        ),
        Action::Page(next) => ("pager", u8::from(next)),
    };
    Stop::item(orders::CHALLENGES.name, id, item)
}
