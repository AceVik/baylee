//! Real independent-equipment execution under generation-five admission.
use super::{Definition, Family, Rarity, RunPack, v2};
use baylee_cards_dsl::{CounterKind, TokenDef};
use baylee_core::{
    ids::{ObjectId, PlayerId},
    preset::{FormatId, GamePreset, SeatController},
};
use baylee_engine::{
    choice::{Pending, PlayerAction},
    engine::{Engine, NoLoopDetection},
    extensions::{NativeLookup, NativeRulePack},
    object::ObjectKind,
    turn::Step,
    zone::ZoneLocation,
};
use baylee_gamehost::RegistryLookup;

type Registry = NativeLookup<RegistryLookup, RunPack>;
type Game = Engine<Registry, NoLoopDetection>;

mod admission;
mod ai;
mod arcane;
mod growth;
mod mulligans;
mod protection;
mod resources;
mod vitality;

fn item(family: Family, rarity: Rarity) -> Definition {
    Definition { family, rarity }
}

fn setup(active: &[Definition], cards: &[&str], seed: u64) -> GamePreset {
    let row =
        |name| baylee_cards::decks::DeckCard::plain(baylee_cards::decks::by_name(name).unwrap());
    let deck = baylee_cards::decks::LoadedDeck {
        name: "Independent run fixture".into(),
        main: cards
            .iter()
            .map(|name| row(name))
            .chain(std::iter::repeat_n(row("Forest"), 39 - cards.len()))
            .collect(),
        commanders: vec![row("Cromat")],
        sideboard: Vec::new(),
    };
    let mut preset = baylee_cards::decks::preset_for(seed, &deck, &deck);
    preset.format = FormatId::Custom;
    let resources = [
        v2::loadout::resources_for(active, [0, 0], 0, v2::generation::RUN).unwrap(),
        v2::loadout::resources_for(&[], [0, 0], 1, v2::generation::RUN).unwrap(),
    ];
    preset.house_rules = v2::loadout::rules_for(&resources, 17, v2::generation::RUN).unwrap();
    preset.seats[0].controller = SeatController::Open;
    preset.seats[1].controller = SeatController::Ai(
        v2::generation::contract(v2::generation::RUN)
            .unwrap()
            .expert(),
    );
    for (seat, resource) in preset.seats.iter_mut().zip(resources) {
        seat.starting_life = Some(resource.life);
        // Match Tower's package projection: reference rules, no physical-print overrides.
        for row in seat.deck.iter_mut().chain(&mut seat.commanders) {
            row.rule_print = None;
        }
    }
    v2::loadout::validate_for(&preset, v2::generation::RUN).expect("derived run resources");
    for seat in &preset.seats {
        baylee_tower_content::validate_seat(seat).expect("native commander fixture");
    }
    preset
}

fn prepared(active: &[Definition], cards: &[&str], seed: u64) -> Game {
    let mut game = Game::with_detection(&setup(active, cards, seed), Registry::default()).unwrap();
    for seat in [0, 1] {
        game.apply(PlayerId::new(seat), PlayerAction::MulliganKeep)
            .unwrap();
    }
    game
}

fn advance(game: &mut Game) {
    let pending = game.pending();
    let player = pending.asked().expect("live fixture question");
    let action = if let Pending::Arrange { cards, piles, .. } = pending {
        PlayerAction::Arrange {
            piles: baylee_engine::choice::default_arrangement(cards, piles)
                .expect("legal unchanged arrangement"),
        }
    } else if let Pending::ChooseColor { options, .. } = pending {
        PlayerAction::ChooseColor(options[0])
    } else if let Pending::ChooseCards {
        options,
        min,
        total: None,
        ..
    } = pending
    {
        PlayerAction::ChooseObjects {
            objects: options.iter().copied().take(usize::from(*min)).collect(),
        }
    } else if let Pending::DiscardChoice { count, .. } = pending {
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
    for _ in 0..1024 {
        if predicate(game) {
            return;
        }
        advance(game);
    }
    panic!("bounded run fixture did not reach its condition");
}

fn main(game: &mut Game, turn: u32, player: PlayerId) {
    until(game, |game| {
        game.state().turn.number >= turn
            && game.state().turn.active == player
            && game.state().turn.step == Step::Main
            && game.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(game.pending(), Pending::Priority { player: asked, .. } if *asked == player)
    });
}

fn source(game: &Game, player: PlayerId, definition: Definition) -> ObjectId {
    let token = baylee_cards::tokens::tower::run::definition(definition.id()).unwrap();
    *game
        .state()
        .zones
        .list(ZoneLocation::Command(player))
        .iter()
        .find(|id| {
            game.state()
                .arena
                .get(**id)
                .unwrap()
                .token
                .is_some_and(|candidate| std::ptr::eq(candidate, token))
        })
        .expect("equipped independent item")
}

fn plants(game: &Game, player: PlayerId) -> usize {
    game.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            let object = game.state().arena.get(**id).unwrap();
            object.controller == player
                && object.token.is_some_and(|token| {
                    std::ptr::eq(token, &raw const baylee_cards::tokens::TOWER_SEEDLING)
                })
        })
        .count()
}

fn charges(game: &Game, source: ObjectId) -> u16 {
    game.state()
        .arena
        .get(source)
        .unwrap()
        .counters
        .get(CounterKind::Charge)
}

fn resolve(game: &mut Game) {
    until(game, |game| {
        game.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(game.pending(), Pending::Priority { .. })
    });
}

fn in_hand(game: &Game, player: PlayerId, name: &str) -> Option<ObjectId> {
    let card = baylee_cards::decks::by_name(name).unwrap();
    game.state()
        .zones
        .list(ZoneLocation::Hand(player))
        .iter()
        .copied()
        .find(|id| {
            game.state()
                .arena
                .get(*id)
                .unwrap()
                .card
                .is_some_and(|row| row.index == card)
        })
}

fn opening(active: &[Definition], cards: &[&str]) -> Game {
    let player = PlayerId::new(0);
    for seed in 0..1024 {
        let mut game = prepared(active, cards, seed);
        if !cards
            .iter()
            .all(|name| in_hand(&game, player, name).is_some())
        {
            continue;
        }
        main(&mut game, 1, player);
        return game;
    }
    panic!("bounded deterministic fixture seed search");
}

fn cast_free(game: &mut Game, name: &str) {
    let player = PlayerId::new(0);
    let card = in_hand(game, player, name).expect("fixture hand");
    game.apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    resolve(game);
}

fn land(game: &mut Game) {
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("land priority")
    };
    game.apply(
        PlayerId::new(0),
        PlayerAction::PlayLand {
            card: legal.lands[0],
        },
    )
    .unwrap();
}

fn float_mana(game: &mut Game, count: usize) {
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("mana priority")
    };
    let sources: Vec<_> = legal.mana_abilities.iter().copied().take(count).collect();
    assert_eq!(sources.len(), count);
    for source in sources {
        game.apply(
            PlayerId::new(0),
            PlayerAction::ActivateManaAbility { source },
        )
        .unwrap();
    }
}

fn permanent(game: &Game, name: &str) -> ObjectId {
    let card = baylee_cards::decks::by_name(name).unwrap();
    *game
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|id| {
            game.state()
                .arena
                .get(**id)
                .unwrap()
                .card
                .is_some_and(|row| row.index == card)
        })
        .expect("played fixture permanent")
}
