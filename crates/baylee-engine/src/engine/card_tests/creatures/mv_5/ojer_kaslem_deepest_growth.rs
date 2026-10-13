//! `cards/creatures/mv_5/ojer_kaslem_deepest_growth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

/// "Trample": Ojer Kaslem is a 6/5 with trample and, attacking an opponent
/// who does not block, deals 6 combat damage to them. The reveal clause is
/// declined here (nothing is put onto the battlefield); it has its own tests
/// below.
#[test]
fn ojer_kaslem_has_trample_and_deals_combat_damage() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ojer_kaslem_deepest_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ojer = on_battlefield(&engine, p0, ojer_kaslem_deepest_growth())
        .expect("Ojer Kaslem is on the battlefield");
    assert_eq!(pt(&engine, ojer), (6, 5), "base stats are 6/5");
    assert!(
        keywords(&engine, ojer).contains(KeywordSet::TRAMPLE),
        "has trample"
    );

    kit::declare_attack(&mut engine, &[ojer], p1);
    kit::drive(
        &mut engine,
        |e| matches!(e.state().turn.phase, Phase::Ending),
        |_, pending| kit::choose_cards(pending, &[]),
    );

    assert_eq!(
        kit::life(&engine, p1),
        14,
        "deals 6 unblocked combat damage to opponent"
    );
}

/// What the library looks like when Ojer Kaslem is about to connect: player 0
/// has Ojer on the battlefield, and the top six cards of their library are, top
/// first, a Llanowar Elves, a Giant Growth, an Island, a Gray Ogre, a Forest
/// and a Swamp. Two creature cards, three land cards and an instant.
struct Library {
    /// The six cards, top first.
    cards: [ObjectId; 6],
}

impl Library {
    fn elves(&self) -> ObjectId {
        self.cards[0]
    }
    fn growth(&self) -> ObjectId {
        self.cards[1]
    }
    fn island(&self) -> ObjectId {
        self.cards[2]
    }
    fn ogre(&self) -> ObjectId {
        self.cards[3]
    }
    fn forest(&self) -> ObjectId {
        self.cards[4]
    }
    fn swamp(&self) -> ObjectId {
        self.cards[5]
    }
}

/// Ojer Kaslem on player 0's battlefield, `theirs` on player 1's, and the six
/// cards of [`Library`] stacked on top of player 0's library.
#[track_caller]
fn ojer_ready_to_connect(theirs: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId, Library) {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ojer_kaslem_deepest_growth()])
        .battlefield(1, theirs)
        .hand(
            0,
            &[
                llanowar_elves(),
                giant_growth(),
                island(),
                gray_ogre(),
                forest(),
                swamp(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ojer = on_battlefield(&engine, p0, ojer_kaslem_deepest_growth()).expect("Ojer");
    let stacked = kit::stack_library(
        &mut engine,
        p0,
        &[
            llanowar_elves(),
            giant_growth(),
            island(),
            gray_ogre(),
            forest(),
            swamp(),
        ],
    );
    let cards = [
        stacked[0], stacked[1], stacked[2], stacked[3], stacked[4], stacked[5],
    ];
    (engine, ojer, Library { cards })
}

/// Attacks player 1 with `ojer` and plays the combat through, taking every
/// question the reveal asks with `answer` and recording each `ChooseCards`
/// menu it is shown.
#[track_caller]
fn connect(
    engine: &mut Engine<RegistryLookup>,
    ojer: ObjectId,
    wanted: &[ObjectId],
) -> Vec<Vec<ObjectId>> {
    let mut menus = Vec::new();
    kit::declare_attack(engine, &[ojer], kit::P1);
    kit::drive(
        engine,
        |e| matches!(e.state().turn.phase, Phase::Ending),
        |_, pending| {
            if let Pending::ChooseCards { options, .. } = pending {
                menus.push(options.clone());
            }
            kit::choose_cards(pending, wanted)
        },
    );
    menus
}

/// "Whenever Ojer Kaslem deals combat damage to a player, reveal that many
/// cards from the top of your library. You may put a creature card and/or a
/// land card from among them onto the battlefield. Put the rest on the bottom
/// in a random order."
///
/// Ojer connects for 6: the six stacked cards are revealed. The Gray Ogre
/// (creature) and the Island (land) are put onto the battlefield; the Elves,
/// the Giant Growth, the Forest and the Swamp go to the bottom. Ruling
/// 2023-11-10: "you could put no cards, a creature card, a land card, or a
/// creature card and a land card".
#[test]
fn ojer_kaslem_puts_a_creature_and_a_land_from_the_revealed_cards_onto_the_battlefield() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, ojer, library) = ojer_ready_to_connect(&[]);
    let (ogre, island) = (library.ogre(), library.island());

    let menus = connect(&mut engine, ojer, &[ogre, island]);

    assert_eq!(kit::life(&engine, p1), 14, "Ojer connected for 6");
    assert!(
        !menus.is_empty(),
        "the controller was asked what to put onto the battlefield"
    );
    let mut offered: Vec<ObjectId> = menus.iter().flatten().copied().collect();
    offered.sort_unstable();
    offered.dedup();
    assert!(
        !offered.contains(&library.growth()),
        "an instant is neither a creature nor a land card: {offered:?}"
    );
    assert!(
        offered.contains(&ogre) && offered.contains(&island),
        "the revealed creature and land cards are on offer: {offered:?}"
    );
    assert_eq!(
        kit::zone_of(&engine, ogre),
        Zone::Battlefield,
        "the creature card"
    );
    assert_eq!(
        kit::zone_of(&engine, island),
        Zone::Battlefield,
        "and the land card"
    );
    assert_eq!(kit::controller_of(&engine, ogre), p0);
    assert_eq!(kit::controller_of(&engine, island), p0);

    let mut rest = vec![
        library.elves(),
        library.growth(),
        library.forest(),
        library.swamp(),
    ];
    rest.sort_unstable();
    let pile = kit::library_of(&engine, p0);
    let mut bottom = pile[..4].to_vec();
    bottom.sort_unstable();
    assert_eq!(
        bottom, rest,
        "the other four are on the bottom of the library, in some order"
    );
}

/// "You may put…": declining is legal, and then all six revealed cards go to
/// the bottom and nothing enters.
#[test]
fn ojer_kaslem_may_put_nothing_onto_the_battlefield() {
    let p0 = kit::P0;
    let (mut engine, ojer, library) = ojer_ready_to_connect(&[]);

    connect(&mut engine, ojer, &[]);

    for card in library.cards {
        assert_ne!(
            kit::zone_of(&engine, card),
            Zone::Battlefield,
            "{card:?} was declined"
        );
    }
    let mut all = library.cards.to_vec();
    all.sort_unstable();
    let pile = kit::library_of(&engine, p0);
    let mut bottom = pile[..6].to_vec();
    bottom.sort_unstable();
    assert_eq!(bottom, all, "all six revealed cards went to the bottom");
}

/// "…a creature card and/or a land card": one of each at most. Two creature
/// cards (the Elves and the Ogre) are refused, and so are two land cards (the
/// Island and the Forest); the answers that are refused change nothing, and a
/// creature and a land are then accepted.
#[test]
fn ojer_kaslem_cannot_put_two_creature_cards_or_two_land_cards_onto_the_battlefield() {
    let p0 = kit::P0;
    let (mut engine, ojer, library) = ojer_ready_to_connect(&[]);
    kit::declare_attack(&mut engine, &[ojer], kit::P1);
    kit::drive(
        &mut engine,
        |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
                || matches!(e.state().turn.phase, Phase::Ending)
        },
        |_, _| None,
    );
    assert!(
        matches!(engine.pending(), Pending::ChooseCards { .. }),
        "Ojer connected, so the controller is asked which revealed cards to \
         put onto the battlefield; got {:?}",
        engine.pending()
    );

    let two_creatures = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![library.elves(), library.ogre()],
        },
    );
    assert!(
        two_creatures.is_err(),
        "only one creature card may be put onto the battlefield"
    );
    let two_lands = engine.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![library.island(), library.forest()],
        },
    );
    assert!(
        two_lands.is_err(),
        "only one land card may be put onto the battlefield"
    );
    assert!(
        matches!(engine.pending(), Pending::ChooseCards { .. }),
        "the refused answers left the question open: {:?}",
        engine.pending()
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![library.ogre(), library.island()],
            },
        )
        .expect("one creature card and one land card is what the card allows");
    assert_eq!(kit::zone_of(&engine, library.ogre()), Zone::Battlefield);
    assert_eq!(kit::zone_of(&engine, library.island()), Zone::Battlefield);
    assert_ne!(
        kit::zone_of(&engine, library.elves()),
        Zone::Battlefield,
        "the second creature card stays off the battlefield"
    );
    assert_ne!(
        kit::zone_of(&engine, library.forest()),
        Zone::Battlefield,
        "and so does the second land card"
    );
}

/// "…reveal **that many** cards": the number is the combat damage dealt *to the
/// player*. Blocked by a Gray Ogre, the 6/5 trampler assigns it the 2 damage
/// that is lethal and tramples over for 4: only the top four cards are
/// revealed — the Elves, the Giant Growth, the Island and the Ogre. The Forest
/// and the Swamp are not revealed, so they are not on offer and stay on top of
/// the library, in order.
#[test]
fn ojer_kaslem_reveals_as_many_cards_as_the_combat_damage_it_dealt() {
    let (p0, p1) = (kit::P0, kit::P1);
    let (mut engine, ojer, library) = ojer_ready_to_connect(&[gray_ogre()]);
    let blocker = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    kit::declare_attack(&mut engine, &[ojer], p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, ojer)],
            },
        )
        .expect("an Ogre may block");
    let mut menus = Vec::new();
    kit::drive(
        &mut engine,
        |e| matches!(e.state().turn.phase, Phase::Ending),
        |_, pending| {
            if let Pending::ChooseCards { options, .. } = pending {
                menus.push(options.clone());
            }
            kit::choose_cards(pending, &[])
        },
    );

    assert_eq!(
        kit::life(&engine, p1),
        16,
        "6 trampling over 2 toughness: 4 to the player"
    );
    assert_eq!(
        kit::zone_of(&engine, blocker),
        Zone::Graveyard,
        "the blocker died"
    );
    for menu in &menus {
        assert!(
            !menu.contains(&library.forest()) && !menu.contains(&library.swamp()),
            "only four cards were revealed: {menu:?}"
        );
    }
    let pile = kit::library_of(&engine, p0);
    assert_eq!(
        &pile[pile.len() - 2..],
        &[library.swamp(), library.forest()],
        "the two unrevealed cards are still on top, the Forest first"
    );
    let mut revealed = vec![
        library.elves(),
        library.growth(),
        library.island(),
        library.ogre(),
    ];
    revealed.sort_unstable();
    let mut bottom = pile[..4].to_vec();
    bottom.sort_unstable();
    assert_eq!(
        bottom, revealed,
        "the four revealed cards went to the bottom"
    );
}

/// "When Ojer Kaslem dies, return it to the battlefield tapped and transformed
/// under its owner's control."
///
/// Destroyed, Ojer comes back as Temple of Cultivation: a tapped land, face 1
/// of the same card, no creature, under its owner's control.
#[test]
fn ojer_kaslem_dies_and_returns_tapped_and_transformed() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ojer_kaslem_deepest_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ojer = on_battlefield(&engine, p0, ojer_kaslem_deepest_growth()).expect("Ojer");

    kill(&mut engine, ojer);

    let temple = on_battlefield(&engine, p0, ojer_kaslem_deepest_growth())
        .expect("it came back to the battlefield");
    assert_eq!(
        kit::face_of(&engine, temple),
        1,
        "transformed: Temple of Cultivation"
    );
    let types = types(&engine, temple);
    assert!(types.contains(TypeSet::LAND), "a land");
    assert!(
        !types.contains(TypeSet::CREATURE),
        "and no creature: {types:?}"
    );
    assert!(is_tapped(&engine, temple), "it returns tapped");
    assert_eq!(kit::controller_of(&engine, temple), p0);
    assert!(
        in_graveyard(&engine, p0, ojer_kaslem_deepest_growth()).is_none(),
        "and it is no longer in the graveyard"
    );
}

/// "…under its **owner's** control": player 1 owns Ojer Kaslem, player 0 has
/// taken it with Control Magic. When it dies it comes back as Temple of
/// Cultivation under player 1's control, tapped, and player 0 gets nothing.
#[test]
fn ojer_kaslem_stolen_returns_under_its_owner_s_control() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[control_magic()])
        .battlefield(1, &[ojer_kaslem_deepest_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ojer = on_battlefield(&engine, p1, ojer_kaslem_deepest_growth()).expect("their Ojer");
    cast_from_hand(&mut engine, p0, control_magic());
    aim_at(&mut engine, p0, ojer);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(kit::controller_of(&engine, ojer), p0, "player 0 stole it");
    assert_eq!(
        kit::owner_of(&engine, ojer),
        p1,
        "and player 1 still owns it"
    );

    kill(&mut engine, ojer);

    let temple = on_battlefield(&engine, p1, ojer_kaslem_deepest_growth())
        .expect("it came back under its owner's control");
    assert_eq!(kit::face_of(&engine, temple), 1, "transformed");
    assert!(is_tapped(&engine, temple), "tapped");
    assert!(
        on_battlefield(&engine, p0, ojer_kaslem_deepest_growth()).is_none(),
        "and not under the control of the player who stole it"
    );
}

/// Temple of Cultivation: "{T}: Add {G}."
///
/// Turned over where it stands, it taps for exactly one green mana.
#[test]
fn temple_of_cultivation_taps_for_green() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ojer_kaslem_deepest_growth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let temple = on_battlefield(&engine, p0, ojer_kaslem_deepest_growth()).expect("the card");
    kit::turn_over(&mut engine, p0, temple, ojer_kaslem_deepest_growth());

    activate(&mut engine, p0, ojer_kaslem_deepest_growth(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one mana");
    assert_eq!(pool.available(ManaColor::Green), 1, "and it is green");
    assert!(is_tapped(&engine, temple), "{{T}} is the cost");
}

/// "{2}{G}, {T}: Transform this land. Activate only if you control ten or more
/// permanents": with nine permanents (the Temple and eight Forests) and `{2}{G}`
/// floating the ability is not offered; with ten it is.
#[test]
fn temple_of_cultivation_needs_ten_permanents_to_transform() {
    let p0 = kit::P0;
    for (forests, offered) in [(8_usize, false), (9, true)] {
        let mut board = vec![ojer_kaslem_deepest_growth()];
        board.extend(vec![forest(); forests]);
        let mut engine = Duel::new(SEED, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let temple = on_battlefield(&engine, p0, ojer_kaslem_deepest_growth()).expect("the card");
        kit::turn_over(&mut engine, p0, temple, ojer_kaslem_deepest_growth());
        tap_mana_except(&mut engine, p0, temple);

        let now = kit::offered(&engine, temple);

        assert_eq!(
            now.contains(&1),
            offered,
            "with {} permanents the transform is {}offered: {now:?}",
            forests + 1,
            if offered { "" } else { "not " }
        );
    }
}

/// Temple of Cultivation's "{2}{G}, {T}: Transform this land. Activate only if
/// you control ten or more permanents and only as a sorcery." The land is
/// placed on its back face (its dies trigger, the other way there, is not
/// written) beside nine Forests: ten permanents. It turns over into Ojer
/// Kaslem where it stands (CR 701.27a) and is the same permanent (CR 712.18):
/// still tapped from the activation, and no newcomer to its controller, so it
/// is not summoning sick (CR 302.6). The stand-in (#206) returned a new,
/// untapped, summoning-sick god.
#[test]
fn temple_of_cultivation_turns_back_into_ojer_kaslem_where_it_stands() {
    let p0 = PlayerId::new(0);
    let def = baylee_cards::by_index(ojer_kaslem_deepest_growth()).expect("in the pool");
    let mut board = vec![ojer_kaslem_deepest_growth()];
    board.extend([forest(); 9]);
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &board).start();
    let temple = on_battlefield(&engine, p0, ojer_kaslem_deepest_growth()).expect("the card");
    assert!(
        engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up")
            .transform(temple, def, 1),
        "placed on its back face"
    );
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    assert!(
        types(&engine, temple).contains(TypeSet::LAND),
        "Temple of Cultivation"
    );
    let temple_was = identity(&engine, temple);

    tap_mana_except(&mut engine, p0, temple);
    activate(&mut engine, p0, ojer_kaslem_deepest_growth(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        on_battlefield(&engine, p0, ojer_kaslem_deepest_growth()).map(|id| identity(&engine, id)),
        Some(temple_was),
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );
    assert_eq!(
        engine.state().object(temple).map(|o| o.face_index),
        Some(0),
        "Ojer Kaslem again"
    );
    assert_eq!(pt(&engine, temple), (6, 5), "the god's own 6/5");
    assert!(
        is_tapped(&engine, temple),
        "tapped for the activation: turning over is not entering"
    );
    let god = engine.state().object(temple).expect("the god");
    assert!(
        !crate::combat::summoning_sick(engine.state(), god),
        "under p0's control since the turn began"
    );
}
