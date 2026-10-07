//! `cards/lands/legendary/mines_of_moria.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mines of Moria enters tapped unless you control a legendary creature, taps
/// for {R}, and turns {3}{R}, its own tap and three cards out of *your*
/// graveyard into two Treasures. The first game reads the whole card in one go:
/// Thrun stands beside it, so the land is put onto the battlefield by a real
/// `PlayLand` — a `starting_battlefield` placement never runs an entry clause —
/// and arrives untapped, and the three exile questions are answered one at a
/// time off a graveyard seeded on *both* sides, so "three cards" is three
/// different cards and "your" keeps the opponent's pile where it is. The second
/// game is the other half of that clause: the same land with no legendary
/// creature anywhere arrives tapped and offers nothing until the untap step
/// stands it back up.
#[test]
#[allow(clippy::too_many_lines)] // two entry branches, a mana line and a three-part cost
fn mines_of_moria_enters_untapped_beside_a_legendary_creature_and_buys_two_treasures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut board = vec![mountain(); 4];
    board.push(thrun_the_last_troll());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[mines_of_moria()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A legendary creature is already on the table, so the printed "unless"
    // does not hold.
    let land = play_land(&mut engine, p0, mines_of_moria());
    assert!(
        on_battlefield(&engine, p0, mines_of_moria()).is_some(),
        "the land was played onto the battlefield"
    );
    assert!(
        !entered_tapped(&engine, land),
        "\"enters tapped unless you control a legendary creature\": Thrun is a \
         legendary creature this seat controls"
    );

    // Ability 0 is the printed "{T}: Add {R}"; the activated line is ability 1.
    activate(&mut engine, p0, mines_of_moria(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "{{T}}: Add {{R}}"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");

    // The {T} is spent and the line below needs it back, so the scenario walks
    // one turn cycle: the pool empties with the step that ends (CR 500.5) and
    // the untap step stands the land up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood the land up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended"
    );

    // Three cards in *my* graveyard and three in the opponent's: the cost
    // prints "your graveyard", so the pile across the table is the witness.
    seed_graveyard(&mut engine, p0, 3);
    seed_graveyard(&mut engine, p1, 3);
    let mine = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    let theirs_before = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();
    assert_eq!(mine.len(), 3, "three cards were buried on my side");

    // Four Mountains pay the {3}{R}, with the Mines named as the card kept
    // back: its own {T} is half of the price below, so `tap_all_mana` would
    // have spent it (#159).
    tap_all_mana_but(&mut engine, p0, Some(mines_of_moria()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Mountains, four red, and the Mines still standing"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        4,
        "and nothing of it came off the land"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "with the mana in the pool the activated line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, mines_of_moria(), 1);

    // The three ExileFromGraveyard parts are three questions, one card each,
    // and each offers what the graveyard still holds after the previous answer.
    let mut menus: Vec<Vec<ObjectId>> = Vec::new();
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat pays its own cost");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostExile,
                    "a cost and not a search, which is all a client has to tell apart"
                );
                assert_eq!((min, max), (1, 1), "one card per part");
                assert_eq!(
                    options.len(),
                    3 - menus.len(),
                    "each question offers what is left of my graveyard: {options:?}"
                );
                for id in &options {
                    assert!(
                        mine.contains(id),
                        "\"from *your* graveyard\": {id:?} is no card of mine: {options:?}"
                    );
                }
                menus.push(options.clone());
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .expect("a card the question offered pays the cost");
            }
            Pending::Priority { .. } if menus.len() == 3 => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Mines' ability is paid for: {other:?}"),
        }
    }
    assert_eq!(menus.len(), 3, "one question per ExileFromGraveyard part");

    assert!(is_tapped(&engine, land), "{{T}} is part of the price");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "making tokens is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Treasures arrive on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);
    let treasures = tokens_of(&engine, p0);
    assert_eq!(treasures.len(), 2, "\"Create two Treasure tokens\"");
    for token in &treasures {
        let object = engine
            .state()
            .object(*token)
            .expect("the token is an object");
        assert!(
            object.characteristics().types.contains(TypeSet::ARTIFACT),
            "a Treasure is an artifact token"
        );
        assert_eq!(
            object.token.expect("it knows which token it is").name,
            "Treasure",
            "and it is a Treasure and not some other token"
        );
    }
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .is_empty(),
        "all three cards were exiled, so no card answered twice"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        theirs_before,
        "and the graveyard across the table never moved: the cost says \"your\""
    );

    // The other half of the entry clause, on a board with no legendary creature
    // anywhere: the same land arrives tapped and offers nothing.
    let mut control = Duel::new(SEED, forest())
        .hand(0, &[mines_of_moria()])
        .start();
    keep_mulligans(&mut control);
    reach_main_phase(&mut control, p0);
    let alone = play_land(&mut control, p0, mines_of_moria());
    assert!(
        entered_tapped(&control, alone),
        "\"enters tapped unless you control a legendary creature\" — and this \
         seat controls no legendary creature"
    );
    let Pending::Priority { legal, .. } = control.pending().clone() else {
        panic!("expected priority, got {:?}", control.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == alone),
        "a land that arrived tapped has no {{T}} to pay with: {:?}",
        legal.abilities
    );

    // A turn later the untap step gives the same {T} back, so the missing line
    // above was the tap and not an ability the card does not have.
    reach_their_main_phase(&mut control, p1);
    reach_their_main_phase(&mut control, p0);
    assert!(!is_tapped(&control, alone), "the untap step stood it up");
    activate(&mut control, p0, mines_of_moria(), 0);
    assert_eq!(
        control.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "and the same {{T}} the tapped land withheld now makes its red"
    );
}
