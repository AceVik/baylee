//! Played original items: real setup, pending choices, costs and normal stack.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::{CounterKind, TokenDef};
use baylee_core::ids::ObjectId;
use baylee_engine::turn::Step;
use baylee_gamehost::record::{Line, Source};

mod amber;
mod chalice;
mod coil;
mod hearth;

fn item(family: Family, rarity: Rarity) -> Definition {
    Definition { family, rarity }
}

fn setup(active: &[Definition], cards: &[(&str, usize)]) -> GamePreset {
    let deck = baylee_cards::decks::LoadedDeck {
        name: "Native life fixture".into(),
        main: cards
            .iter()
            .flat_map(|(name, count)| {
                std::iter::repeat_n(
                    baylee_cards::decks::DeckCard::plain(
                        baylee_cards::decks::by_name(name).unwrap(),
                    ),
                    *count,
                )
            })
            .collect(),
        sideboard: Vec::new(),
        commanders: Vec::new(),
    };
    let mut setup = baylee_cards::decks::preset_for(19, &deck, &deck);
    setup.format = FormatId::Custom;
    let resources = [
        v2::loadout::resources(active, [0, 0], 0).unwrap(),
        v2::loadout::resources(active, [0, 0], 1).unwrap(),
    ];
    setup.house_rules = v2::loadout::rules(&resources, 17).unwrap();
    setup.seats[0].controller = SeatController::Open;
    setup.seats[1].controller = SeatController::Ai(v2::generation::CURRENT.expert());
    for (seat, resource) in setup.seats.iter_mut().zip(resources) {
        seat.starting_life = Some(resource.life);
    }
    setup
}

fn prepared(active: &[Definition], cards: &[(&str, usize)]) -> Game {
    let mut game = Game::with_detection(&setup(active, cards), Registry::default()).unwrap();
    for seat in [0, 1] {
        game.apply(PlayerId::new(seat), PlayerAction::MulliganKeep)
            .unwrap();
    }
    game
}

fn advance(game: &mut Game) {
    let pending = game.pending();
    let player = pending.asked().expect("live test question");
    let action = if let Pending::DiscardChoice { count, .. } = pending {
        PlayerAction::ChooseObjects {
            objects: game
                .state()
                .zones
                .list(ZoneLocation::Hand(player))
                .iter()
                .copied()
                .take(usize::from(*count))
                .collect(),
        }
    } else {
        baylee_engine::choice::timeout_answer(pending)
            .unwrap_or_else(|| panic!("unexpected {pending:?}"))
    };
    game.apply(player, action).unwrap();
}

fn until(game: &mut Game, predicate: impl Fn(&Game) -> bool) {
    for _ in 0..2048 {
        if predicate(game) {
            return;
        }
        advance(game);
    }
    panic!("bounded fixture failed to reach its condition");
}

fn main(game: &mut Game, turn: u32, seat: PlayerId) {
    until(game, |game| {
        game.state().turn.number >= turn
            && game.state().turn.active == seat
            && game.state().turn.step == Step::Main
            && game.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(game.pending(), Pending::Priority { player, .. } if *player == seat)
    });
}

fn resolve(game: &mut Game) {
    until(game, |game| {
        game.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(game.pending(), Pending::Priority { .. })
    });
}

fn source(game: &Game, seat: PlayerId, token: &TokenDef) -> ObjectId {
    *game
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            let object = game.state().arena.get(**id).unwrap();
            object.base_controller == seat
                && object
                    .token
                    .is_some_and(|candidate| std::ptr::eq(candidate, token))
        })
        .expect("equipped original artifact")
}

fn mana(game: &mut Game, seat: PlayerId, count: usize) {
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("priority");
    };
    let sources: Vec<_> = legal.mana_abilities.iter().copied().take(count).collect();
    assert_eq!(sources.len(), count);
    for source in sources {
        game.apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
}

fn land(game: &mut Game, seat: PlayerId) {
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("priority");
    };
    game.apply(
        seat,
        PlayerAction::PlayLand {
            card: legal.lands[0],
        },
    )
    .unwrap();
}

fn ai(game: &Game, seat: PlayerId, profile: AIProfile) -> PlayerAction {
    let agent = baylee_ai::HeuristicAgent::new(profile);
    let view = baylee_gamehost::player_view(
        game.state(),
        seat,
        1,
        Some(game.pending()),
        &baylee_gamehost::SeatContext {
            awaiting: Some(seat),
            ..Default::default()
        },
        &[],
    );
    let action = agent.act(&view, game.pending());
    assert_eq!(agent.fallbacks(), 0);
    action
}

fn recorded_expert_actions(record: &[u8]) -> impl Iterator<Item = PlayerAction> + '_ {
    record
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<Line>(line).unwrap())
        .filter_map(|line| match line {
            Line::Input {
                seat: 1,
                by: Source::House,
                action,
                ..
            } => Some(action),
            _ => None,
        })
}
