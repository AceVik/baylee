mod bounds;
mod choosers;
mod combat;
mod ending;
mod picks;
mod priority;
mod prompts;

use super::*;
use baylee_engine::choice::ArrangePile;

// ------------------------------------------------- how a game ends
fn ended(winner: Option<Victor>, reason: EndReason) -> GameResult {
    GameResult { winner, reason }
}

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// A seat as a defender.
fn seat(id: u8) -> Defender {
    Defender::Player(PlayerId::new(id))
}

/// The attacker choice with a given list of legal attackers and defenders.
fn attack_choice(attackers: Vec<ObjectId>, defenders: Vec<Defender>) -> Pending {
    Pending::ChooseAttackers {
        player: me(),
        attackers,
        defenders,
    }
}

fn interaction(pending: Pending) -> Interaction {
    Interaction::new(pending, me())
}

/// "Put them back in any order": one pile, the top, holding every card.
fn put_back(cards: Vec<ObjectId>) -> Pending {
    let n = u32::try_from(cards.len()).expect("a handful of cards");
    Pending::Arrange {
        player: me(),
        cards,
        piles: vec![ArrangePile::all_of(ArrangePlace::LibraryTop, n)],
        prompt: ArrangePrompt::Order,
    }
}

/// A blocking choice with one attacker per listed blocker.
fn block_choice(options: Vec<BlockOption>) -> Pending {
    Pending::ChooseBlockers {
        player: me(),
        attacker: PlayerId::new(1),
        blockers: options,
    }
}

/// A target prompt over objects and seats.
fn target_choice(
    options: Vec<ObjectId>,
    player_options: Vec<PlayerId>,
    min: u8,
    max: u8,
) -> Pending {
    Pending::ChooseTargets {
        player: me(),
        options,
        player_options,
        min,
        max,
        reason: TargetPrompt::Targets,
    }
}
