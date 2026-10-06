use super::*;

fn reflecting_pool() -> CardIndex {
    card_named("67f43ac6-2a58-4b53-b5d7-0330e2a252e2")
}

fn exotic_orchard() -> CardIndex {
    card_named("27b047e3-0d41-45e2-98e9-9391d7923a1e")
}

fn fellwar_stone() -> CardIndex {
    card_named("95560508-7ac9-4be9-8a3f-3c7d5b52807b")
}

fn command_tower() -> CardIndex {
    card_named("0895c9b7-ae7d-4bb3-af17-3b75deb50a25")
}

fn plains() -> CardIndex {
    card_named("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

/// A duel with a named board on each side, past the mulligans.
///
/// The preset is `mixed_print_preset`'s, so the library is Islands and
/// nothing draws a card that matters; what each test writes is the two
/// `starting_battlefield`s, which is the only input `board_mana` reads.
fn board(seat0: &[CardIndex], seat1: &[CardIndex]) -> (Engine<Registry>, PlayerView) {
    let entry = |&card: &CardIndex| DeckEntry {
        card,
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = seat0.iter().map(entry).collect();
    preset.seats[1].starting_battlefield = seat1.iter().map(entry).collect();

    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    let view = player_view(
        engine.state(),
        PlayerId::new(0),
        1,
        None,
        &SeatContext::default(),
        &[],
    );
    (engine, view)
}

/// What the host projected onto the one permanent of that name.
fn projection<'a>(view: &'a PlayerView, name: &str) -> Option<&'a baylee_view::BoardMana> {
    view.battlefield
        .iter()
        .find(|o| o.name == name)
        .unwrap_or_else(|| panic!("{name} is on the battlefield"))
        .board_mana
        .as_ref()
}

#[test]
fn mana_spending_is_projected_per_seat_without_recoloring_the_pool() {
    let (engine, _) = board(&[baylee_core::generated::index::SUNGLASSES_OF_URZA], &[]);
    let mut state = engine.state().clone();
    for player in &mut state.players {
        player.mana_pool.add(ManaColor::White, 1);
    }
    for viewer in [PlayerId::new(0), PlayerId::new(1)] {
        let view = player_view(&state, viewer, 1, None, &SeatContext::default(), &[]);
        for seat in view.seats {
            assert_eq!((seat.mana_pool.white, seat.mana_pool.red), (1, 0));
            assert_eq!(
                seat.mana_pool
                    .spending
                    .permits(ManaColor::White, ManaColor::Red),
                seat.player == PlayerId::new(0),
                "the controller's permission is public but belongs only to that seat"
            );
            assert!(
                !seat
                    .mana_pool
                    .spending
                    .permits(ManaColor::Red, ManaColor::White)
            );
        }
    }
}

/// The defect this whole field exists for, measured at the board it was
/// found on. A Reflecting Pool beside a Forest and a Plains makes white
/// and green — and said nothing at all before, because the colours are a
/// union over `produced_colors`, a *projected* characteristic no view
/// carried. In the game it was found in, a `{3}{R}` creature would not
/// arm with four untapped lands on the table.
#[test]
fn a_reflecting_pool_says_what_the_lands_beside_it_make() {
    let (_, view) = board(&[reflecting_pool(), forest(), plains()], &[]);
    let pool = projection(&view, "Reflecting Pool").expect("the Pool has a board to read");
    assert_eq!(
        pool.colors,
        vec![ManaColor::White, ManaColor::Green],
        "the union of what the lands you control could produce"
    );
}

/// And the other half of that comparison, which is what makes the first
/// one a measurement: a Pool with no other land is a Pool that makes
/// nothing, and the host says so by projecting nothing at all. A client
/// that read an empty list as "any colour" would tap it and stall.
#[test]
fn a_lone_reflecting_pool_is_nothing_to_plan_with() {
    let (_, view) = board(&[reflecting_pool()], &[forest()]);
    assert!(
        projection(&view, "Reflecting Pool").is_none(),
        "the Pool contributes nothing to its own union, and the Forest is not yours"
    );
}

/// Exotic Orchard reads the *other* side of the table, which is the same
/// rule with `mine` flipped — and the case a projection built from "the
/// lands I can see" would get exactly backwards.
#[test]
fn an_exotic_orchard_reads_the_other_seats_lands() {
    let (_, view) = board(&[exotic_orchard(), forest()], &[plains()]);
    let orchard = projection(&view, "Exotic Orchard").expect("the opponent has a land");
    assert_eq!(
        orchard.colors,
        vec![ManaColor::White],
        "the Plains opposite, and not the Forest beside it"
    );
}

/// Fellwar Stone is the same source on a card that is not a land, which
/// is why the projection is offered for every permanent rather than for
/// lands alone.
#[test]
fn a_fellwar_stone_is_a_board_reader_that_is_not_a_land() {
    let (_, view) = board(&[fellwar_stone()], &[forest(), plains()]);
    let stone = projection(&view, "Fellwar Stone").expect("the opponent has lands");
    assert_eq!(stone.colors, vec![ManaColor::White, ManaColor::Green]);
}

/// Intrinsic mana follows current land types rather than printed symbols,
/// so even an unchanged basic land carries the host's current answer.
#[test]
fn a_forest_needs_no_projection() {
    let (_, view) = board(&[forest(), plains()], &[]);
    let forest = projection(&view, "Forest").expect("intrinsic Forest mana is projected");
    assert_eq!(forest.index, 0);
    assert_eq!(forest.colors, vec![ManaColor::Green]);
    let plains = projection(&view, "Plains").expect("intrinsic Plains mana is projected");
    assert_eq!(plains.index, 0);
    assert_eq!(plains.colors, vec![ManaColor::White]);
}

/// The second board-dependent source, folded into the same field so that
/// the two are never resolved on different sides of the wire. A Command
/// Tower with no commander at the table is the offline house duel, where
/// colourless is what the engine's own fallback leaves.
#[test]
fn a_command_tower_at_a_table_with_no_commander_is_colorless() {
    let (_, view) = board(&[command_tower()], &[]);
    let tower = projection(&view, "Command Tower").expect("the fallback is still an answer");
    assert_eq!(tower.colors, vec![ManaColor::Colorless]);
}

/// And at a Commander table it is the commanders' identity — seat 0 has
/// two, so this is also the case a projection reading one commander would
/// get wrong.
#[test]
fn a_command_tower_is_its_seats_commander_identity() {
    let mut preset = commander_preset();
    let tower = DeckEntry {
        card: command_tower(),
        print: PrintRef::new(0),
    };
    preset.seats[0].starting_battlefield.push(tower);
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    for _ in 0..2 {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan")
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    let view = player_view(
        engine.state(),
        PlayerId::new(0),
        1,
        None,
        &SeatContext::default(),
        &[],
    );

    let identity = |card: CardIndex| {
        baylee_cards::by_index(card)
            .expect("a card at that index")
            .color_identity
    };
    // Katara is `{G}{W}{U}` and Elesh Norn `{4}{W}`, so the union is
    // Katara's — which is worth saying out loud, because it means this
    // fixture cannot show a projection that reads only the first
    // commander. What it can show is the exact answer and its opposite:
    // three named colours, and not the five a constant would give.
    let both = identity(katara()).union(identity(elesh_norn()));
    let projected = projection(&view, "Command Tower").expect("a commander game");
    assert_eq!(
        projected.colors,
        vec![ManaColor::White, ManaColor::Blue, ManaColor::Green],
        "one mana colour per colour of the seat's commanders together"
    );
    assert_eq!(projected.colors.len(), both.iter().count());
    assert!(
        projected.colors.len() < 5,
        "not every commander is five colours, and a Tower that said so \
         would tap for a red the engine then refuses"
    );
}

/// The agreement test, and the one that makes the rest worth anything.
/// The projection is a promise about what the engine will offer, so a
/// colour list that disagreed with the engine's own `ChooseColor` would
/// be a land the planner taps and a payment that then fails — worse than
/// saying nothing. Both sides are the same function by construction;
/// this is what stops that staying true only by construction.
#[test]
fn the_projected_colors_are_the_ones_the_engine_then_offers() {
    let (mut engine, view) = board(&[reflecting_pool(), forest(), plains()], &[]);
    let pool = view
        .battlefield
        .iter()
        .find(|o| o.name == "Reflecting Pool")
        .expect("the Pool is on the battlefield");
    let projected = pool
        .board_mana
        .as_ref()
        .expect("a board to read")
        .colors
        .clone();

    for _ in 0..30 {
        let Pending::Priority { player, legal } = engine.pending().clone() else {
            panic!("expected priority")
        };
        if player != PlayerId::new(0) {
            engine.apply(player, PlayerAction::PassPriority).unwrap();
            continue;
        }
        assert!(
            legal.abilities.contains(&(pool.id, 0)),
            "the engine offers the ability the projection is about"
        );
        engine
            .apply(
                player,
                PlayerAction::ActivateAbility {
                    source: pool.id,
                    ability_index: 0,
                },
            )
            .unwrap();
        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!("a Pool with two colours beside it asks which one")
        };
        assert_eq!(
            options, projected,
            "the view promised what the engine then offered"
        );
        return;
    }
    panic!("seat 0 never got priority");
}
