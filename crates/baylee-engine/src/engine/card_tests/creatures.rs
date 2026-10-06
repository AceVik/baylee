//! Cards whose front face is a creature, the door `cards/creatures/`
//! puts them behind.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;
use crate::choice::ChoicePrompt;

mod artifacts;
mod enchantments;
mod legends_a;
mod legends_b;
mod mv_1;
mod mv_2;
mod mv_3;
mod mv_4;
mod mv_5;
mod mv_6;
mod mv_7;
mod mv_8;
mod mv_9;
mod nether_shadow;
mod sengir_vampire;

// oracle_id = "e3c85068-b4b6-40b9-a16c-5c3b2d059ec4"
fn academy_rector() -> baylee_core::ids::CardIndex {
    card_index("e3c85068-b4b6-40b9-a16c-5c3b2d059ec4")
}

/// An Academy Rector killed by the opponent's Vindicate, stopped at the one
/// question the card asks.
///
/// Two boards over one builder rather than two arms of one, for the reason
/// the Ondu Cleric pair has: the second answer wants the same open board the
/// first one spent.
///
/// The whole library is Luminarch Ascension, and that is what gives the
/// trigger somewhere to reach: "search your library for an enchantment card"
/// has an answer, so a library that stays whole below is a search that was
/// never made rather than a search that found nothing — failing to find in a
/// hidden zone is always legal, and the two outcomes would look the same.
fn a_rector_asking() -> (Engine<RegistryLookup>, PlayerId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, luminarch_ascension())
        .battlefield(0, &[academy_rector()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);

    let rector = on_battlefield(&engine, p0, academy_rector()).expect("the Rector is out");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![rector],
            },
        )
        .expect("their removal may point at an ordinary creature");

    // Stopping *at* the "may" rather than through it: `pass_until` checks the
    // predicate before it answers anything, so the question the card prints
    // is still unanswered when this hands the game back.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    (engine, p0)
}

// oracle_id = "f9b46a1a-474f-4fac-8d71-131c1720e4c0"
fn borg_queen_perfection_manifest() -> baylee_core::ids::CardIndex {
    card_index("f9b46a1a-474f-4fac-8d71-131c1720e4c0")
}

// oracle_id = "c7b044c3-3cfa-407e-bf20-2875e8e04b7b"
fn brazen_borrower() -> baylee_core::ids::CardIndex {
    card_index("c7b044c3-3cfa-407e-bf20-2875e8e04b7b")
}

/// Answers whatever the engine asks until `pred` holds, through the kit's
/// own `answer_one` — with the cleanup discard taken back off it.
///
/// [`pass_until`] would do for all of this but one question, and it is the
/// question this scenario is built to ask: a seat that deliberately holds
/// the card it was dealt across the *opponent's* turn sits on eight cards
/// at a cleanup and is asked to discard, which `pass_until` has no arm for.
/// `answer_one` does have one — and it answers with the first cards in the
/// hand, which may be the Borrower itself, so the card under test would be
/// thrown away before it was ever cast. Everything else is delegated.
#[track_caller]
fn walk_the_game_until(
    engine: &mut Engine<RegistryLookup>,
    pred: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..200 {
        if pred(engine) {
            return;
        }
        if let Pending::DiscardChoice { player, count } = engine.pending().clone() {
            let filler: Vec<ObjectId> = engine
                .state()
                .zones
                .list(crate::zone::ZoneLocation::Hand(player))
                .iter()
                .copied()
                .filter(|id| {
                    engine
                        .state()
                        .object(*id)
                        .is_some_and(|o| o.card.is_some_and(|c| c.index != brazen_borrower()))
                })
                .take(usize::from(count))
                .collect();
            engine
                .apply(player, PlayerAction::ChooseObjects { objects: filler })
                .expect("a seat discards down to seven");
            continue;
        }
        let (player, action) = match answer_one(engine) {
            Ok(pair) => pair,
            Err(rest) => panic!("the game stopped before the test did: {rest:?}"),
        };
        engine
            .apply(player, action)
            .expect("every answer came out of the question that enumerated it");
    }
    panic!("the walk never reached what it was waiting for");
}

// oracle_id = "22f1a4a4-c423-4d1c-8775-0ed604a9fa51"
fn deathrite_shaman() -> baylee_core::ids::CardIndex {
    card_index("22f1a4a4-c423-4d1c-8775-0ed604a9fa51")
}

// oracle_id = "f9d3b046-0b95-4103-a630-4b3fb88bb60b"
fn delighted_halfling() -> baylee_core::ids::CardIndex {
    card_index("f9d3b046-0b95-4103-a630-4b3fb88bb60b")
}
fn ravenous_chupacabra() -> baylee_core::ids::CardIndex {
    card_index("7b459306-149b-4f43-abc1-2dd70c748c0e")
}

// oracle_id = "afa49a09-146f-4439-850e-dd1938c93cef"
fn derevi_empyrial_tactician() -> baylee_core::ids::CardIndex {
    card_index("afa49a09-146f-4439-850e-dd1938c93cef")
}

/// Answers Derevi's trigger: takes mode `mode`, points it at `target`, and
/// lets it resolve.
///
/// Both of her trigger conditions run the same pair of modes — one printed
/// sentence, two abilities over one `SpellMode` list — so both readings of
/// the card go through this one door, and a difference between them would be
/// a difference in the engine rather than in the test.
///
/// Mode 0 is tap and mode 1 is untap, in the order the card writes them. They
/// are answered by *position* and not by number: a modal trigger drops the
/// modes it cannot legally choose (CR 603.3c), so the two are the same list
/// only while both are offered — which is asserted here rather than assumed.
#[track_caller]
fn tap_or_untap(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    mode: usize,
    target: ObjectId,
) {
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Derevi's trigger asked for no mode — got {:?}. \"You may tap or \
             untap target permanent\" is two modes, chosen by the controller \
             as the ability goes on the stack (CR 603.3c); a modal trigger \
             that is never collected is a card with no text at all.",
            engine.pending()
        )
    };
    assert_eq!(player, seat, "Derevi's controller answers her trigger");
    let modes: Vec<usize> = options
        .iter()
        .filter_map(|o| match o.kind {
            crate::choice::CastModeKind::Mode(m) => Some(m),
            _ => None,
        })
        .collect();
    assert_eq!(
        modes,
        vec![0, 1],
        "tap *or* untap, and both are always offered: \"target permanent\" is \
         `Filter::Any` over the battlefield, which is never empty while she \
         is standing on it",
    );
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(m) if m == mode))
        .expect("the mode asked for is on the list");
    engine.apply(seat, PlayerAction::ChooseMode(slot)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the chosen mode asked for no target — got {:?}. The `TargetReq` \
             is on the mode, not on the ability.",
            engine.pending()
        )
    };
    assert!(
        options.contains(&target),
        "\"target permanent\" reaches any permanent on the table, whoever \
         controls it",
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    // The printed "may" is asked on resolution; `pass_until` takes it.
    pass_until(engine, stack_is_empty);
}

// oracle_id = "c8625113-0ce4-4454-83a1-25c31b8bfb9a"
fn disciple_of_the_vault() -> baylee_core::ids::CardIndex {
    card_index("c8625113-0ce4-4454-83a1-25c31b8bfb9a")
}

// oracle_id = "8eb7c0a5-6190-40de-b473-2d1daa3bbe28"
fn dualcaster_mage() -> baylee_core::ids::CardIndex {
    card_index("8eb7c0a5-6190-40de-b473-2d1daa3bbe28")
}

// oracle_id = "b11c250c-f191-4c52-ba02-a9176f163447"
fn emiel_the_blessed() -> baylee_core::ids::CardIndex {
    card_index("b11c250c-f191-4c52-ba02-a9176f163447")
}

// oracle_id = "da3e7d3d-2ca0-40c3-9602-fca37c92f507"
fn emry_lurker_of_the_loch() -> baylee_core::ids::CardIndex {
    card_index("da3e7d3d-2ca0-40c3-9602-fca37c92f507")
}

// oracle_id = "30b24e8e-3b0e-4d8e-90f3-f66eb7c1858c"
fn eternal_witness() -> baylee_core::ids::CardIndex {
    card_index("30b24e8e-3b0e-4d8e-90f3-f66eb7c1858c")
}

/// A cast Eternal Witness whose enters-trigger is waiting to be pointed
/// somewhere, over a graveyard holding exactly one card.
///
/// The card in that graveyard comes back with the engine, because the answer
/// has to be the object the engine itself offered rather than a card looked
/// up by index — and because the assertions afterwards are counts: an object
/// takes a new id when it changes zone, and with a Forest filler deck "a
/// Forest is in hand" is as true of a draw step as of the return.
///
/// A whole game per half, the way `hagra_on_the_table` is used: a second
/// Witness would want three untapped Forests again in a main phase that has
/// already spent them, and what the two halves share is the board, not the
/// turn.
fn a_witness_asking() -> (Engine<RegistryLookup>, PlayerId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[eternal_witness()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    seed_graveyard(&mut engine, p0, 1);
    let buried = *engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(p0))
        .first()
        .expect("one card was put into the graveyard");

    cast_from_hand(&mut engine, p0, eternal_witness());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    (engine, p0, buried)
}

// oracle_id = "a426a258-fd8b-489c-8642-9868ee47de85"
fn golgari_thug() -> baylee_core::ids::CardIndex {
    card_index("a426a258-fd8b-489c-8642-9868ee47de85")
}

/// How many cards sit in one zone, for the two counts the dredge half reads.
fn cards_in(engine: &Engine<RegistryLookup>, loc: crate::zone::ZoneLocation) -> usize {
    engine.state().zones.list(loc).len()
}

// oracle_id = "5eb4403f-f199-4f75-a7c6-e76783f9b07d"
fn liliana_the_repentant() -> baylee_core::ids::CardIndex {
    card_index("5eb4403f-f199-4f75-a7c6-e76783f9b07d")
}

// oracle_id = "726d9d2c-736a-4852-9938-a0f50d8fd89f"
fn marionette_apprentice() -> baylee_core::ids::CardIndex {
    card_index("726d9d2c-736a-4852-9938-a0f50d8fd89f")
}

fn mind_stone() -> baylee_core::ids::CardIndex {
    card_index("c97361b5-af16-4a7b-af85-a429dbaf4ad2")
}

// oracle_id = "056b651e-e0e2-4333-9235-d1ffe8fcca29"
fn stingcaster_mage() -> baylee_core::ids::CardIndex {
    card_index("056b651e-e0e2-4333-9235-d1ffe8fcca29")
}

// oracle_id = "7fd61a18-6e4f-40c5-aa00-3d101ec1ec82"
fn stitcher_s_supplier() -> baylee_core::ids::CardIndex {
    card_index("7fd61a18-6e4f-40c5-aa00-3d101ec1ec82")
}

/// Stitcher's Supplier ({B}, 1/1): "When this creature enters **or dies**,
/// mill three cards."
///
/// One printed sentence firing on two events, and the card is only itself
/// when both of them fire: read as an enter-trigger alone it is an ordinary
/// 1/1 that mills three once, which is a different card — and the half that
/// goes missing is the one no board shows, because a Zombie that entered and
/// milled looks right until somebody kills it. So the Zombie is cast off a
/// Swamp, and then killed by the opponent's Vindicate, and the same
/// three-card step is struck twice.
///
/// Both halves are read off the *library*, which is what makes the word
/// "mill": three cards leave the top of the deck, and the graveyard's growth
/// is struck beside the library's loss so that a mill which exiled or drew
/// instead could not pass. After the death that growth is **four** — three
/// milled cards and the Zombie's own corpse, which is lying in the same
/// graveyard by the time the trigger resolves.
///
/// The opponent's library is the counter-half. `PlayerRel::You` is what makes
/// this "mill three cards" rather than "target player mills three", and a
/// card that milled the wrong seat would satisfy every other assertion here.
#[test]
fn a_stitchers_supplier_mills_three_entering_and_three_more_dying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let graveyard = |e: &Engine<RegistryLookup>| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .len()
    };
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[stitcher_s_supplier()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let library_before = library_size(&engine, p0);
    let graveyard_before = graveyard(&engine);
    let their_library = library_size(&engine, p1);

    cast_from_hand(&mut engine, p0, stitcher_s_supplier());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, stitcher_s_supplier()).is_some(),
        "one Swamp pays {{B}} and the Zombie arrives"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 3,
        "entering took three cards off the top of its controller's library"
    );
    assert_eq!(
        graveyard(&engine),
        graveyard_before + 3,
        "and put all three of them into the graveyard"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library,
        "\"mill three cards\" mills its controller: the opponent's library is untouched"
    );

    // The other half of the same sentence, on the opponent's turn: Vindicate
    // destroys the Zombie, and dying has to mill again.
    reach_their_main_phase(&mut engine, p1);
    let zombie =
        on_battlefield(&engine, p0, stitcher_s_supplier()).expect("the Zombie is still out");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![zombie],
            },
        )
        .expect("their removal may point at an ordinary creature");

    // Snapshotted with the Zombie still alive, so the four cards counted
    // below are the three it mills plus itself and nothing else.
    let library_alive = library_size(&engine, p0);
    let graveyard_alive = graveyard(&engine);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, stitcher_s_supplier()).is_none(),
        "Vindicate destroyed it"
    );
    assert!(
        in_graveyard(&engine, p0, stitcher_s_supplier()).is_some(),
        "and it fell into the graveyard it had been milling into"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_alive - 3,
        "dying is the second event of the one printed sentence: three more \
         cards leave the library"
    );
    assert_eq!(
        graveyard(&engine),
        graveyard_alive + 4,
        "three milled cards and the Zombie's own corpse"
    );
}

// oracle_id = "a145ff8c-5812-4bcb-bd16-9839dc25121d"
fn storm_kiln_artist() -> baylee_core::ids::CardIndex {
    card_index("a145ff8c-5812-4bcb-bd16-9839dc25121d")
}

/// How many Dark Rituals `seat` controls on the stack: one while it is the
/// spell that was cast, two the moment a copy of it stands beside it.
fn rituals_on_the_stack(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .iter()
        .filter(|id| {
            engine.state().object(**id).is_some_and(|o| {
                o.controller == seat && o.card.is_some_and(|c| c.index == dark_ritual())
            })
        })
        .count()
}

// oracle_id = "9deded8b-cec4-4ede-a50b-131404d456d4"
fn thought_monitor() -> baylee_core::ids::CardIndex {
    card_index("9deded8b-cec4-4ede-a50b-131404d456d4")
}

// oracle_id = "f82a4e85-526d-4456-b700-7760043a31be"
fn viscera_seer() -> baylee_core::ids::CardIndex {
    card_index("f82a4e85-526d-4456-b700-7760043a31be")
}

// oracle_id = "afedce7b-0e18-40ad-a26a-1933fddb560d"

/// Akoum Warrior // Akoum Teeth: a {5}{R} 4/5 Minotaur Warrior with trample
/// on the front, a land on the back.
fn akoum_warrior() -> CardIndex {
    card_index("afedce7b-0e18-40ad-a26a-1933fddb560d")
}

// oracle_id = "34320ebf-da97-44a4-bbeb-a9da06548289"

/// Blackbloom Rogue // Blackbloom Bog (ZNR #91), the modal double-faced card
/// the three tests below play from both sides.
fn blackbloom_rogue() -> CardIndex {
    card_index("34320ebf-da97-44a4-bbeb-a9da06548289")
}

// oracle_id = "727f3201-1cfc-4ab2-9dfe-be4f7251f42f"

/// Boggart Trawler // Boggart Bog — a modal double-faced card (CR 712.3)
/// whose front is a {2}{B} 3/1 Goblin and whose back is a land.
fn boggart_trawler() -> CardIndex {
    card_index("727f3201-1cfc-4ab2-9dfe-be4f7251f42f")
}

// oracle_id = "573151f0-00d4-4a8a-8a09-745c5f376532"
fn hydroelectric_specimen() -> CardIndex {
    card_index("573151f0-00d4-4a8a-8a09-745c5f376532")
}

/// Curse of the Swine, which this file wants for one printed property alone:
/// "Exile X target creatures" is a spell that genuinely stands on the stack
/// holding **more than one** target, which is the case the Weird's printed
/// line excludes and the engine cannot.
///
/// Named after the Weird rather than after the card, because the handles in
/// `card_tests` share one namespace and a sibling module may want the card's
/// own name for itself.
fn the_specimens_two_target_spell() -> CardIndex {
    card_index("5669ea7c-c4fc-494c-896b-4bce9b494817")
}

/// Flashes the Weird in on top of `their_spell` and answers with the options
/// its enters-trigger is offered.
///
/// The `castable` assertion is the first printed word doing its work: a
/// creature spell is a noninstant, so CR 117.1a offers it only in its own
/// controller's main phase with an **empty** stack, and the only thing that
/// reaches a stack with somebody else's spell standing on it — or, below,
/// somebody else's turn — is flash (CR 702.8a). The mana is tapped first
/// because `castable` is an affordability answer too, and an untapped board
/// would hide the timing question behind a price.
#[track_caller]
fn flash_the_specimen_in(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    their_spell: ObjectId,
) -> Vec<ObjectId> {
    pass_until(engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana_but(engine, p0, None);
    let weird = in_hand(engine, p0, hydroelectric_specimen()).expect("the Weird is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p0's priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&weird),
        "\"Flash\": a creature spell is offered onto a stack that is not empty"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: weird })
        .expect("three Islands pay {2}{U}");
    options_offered_including(engine, their_spell)
}

/// Answers the trigger's "you may" and points the redirect at the Weird.
///
/// The offer is answered here rather than walked past, because "you may" is a
/// clause under test on both boards. What follows it is the rest of the same
/// sentence: the new target is chosen as the trigger resolves (CR 115.7), and
/// "to this creature" leaves `Filter::This` exactly one thing to offer.
#[track_caller]
fn the_weird_takes_the_aim(engine: &mut Engine<RegistryLookup>, p0: PlayerId, weird: ObjectId) {
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the offer")
    };
    assert_eq!(
        (player, prompt),
        (p0, YesNoPrompt::MayDo),
        "\"you may change the target\" is asked of the Weird's controller"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected the redirect's choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![weird],
        "\"to this creature\": the Weird itself, and nothing else on the board"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![weird],
            },
        )
        .unwrap();
}

/// Ancestral Recall, which this file wants for "target player": a spell the
/// Weird can never be a target for.
///
/// Named after the Weird for the reason [`the_specimens_two_target_spell`]
/// gives.
fn the_specimens_player_spell() -> CardIndex {
    card_index("550c74d4-1fcb-406a-b02a-639a760a4380")
}

/// Plays the back face and answers its entry question with `pay`.
///
/// There is no face choice on the way, and that is worth saying out loud: the
/// card file calls the back "an MDFC land reached by the face choice"
/// (CR 712.12), but the engine only asks when *both* faces are lands, the way
/// a pathway prints them. Here the front is a creature, so exactly one face
/// answers `TypeSet::LAND` and the engine switches to it itself.
#[track_caller]
fn a_played_laboratory(pay: bool) -> (Engine<RegistryLookup>, PlayerId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[hydroelectric_specimen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = play_land(&mut engine, p0, hydroelectric_specimen());
    assert!(
        !matches!(engine.pending(), Pending::ChooseCastMode { .. }),
        "only one face is a land, so nothing is asked about which one"
    );

    // "As this land enters, you may pay 3 life. If you don't, it enters
    // tapped." — the entry stops here, mid-way, and the land is already on
    // the battlefield while the question stands.
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!("the laboratory never asked: {:?}", engine.pending())
    };
    assert_eq!(
        (player, prompt),
        (p0, YesNoPrompt::PayLifeOrEnterTapped { amount: 3 }),
        "three life, which is what this card prints"
    );
    engine.apply(p0, PlayerAction::YesNo(pay)).unwrap();
    (engine, p0, land)
}

// oracle_id = "2ac1c95c-2a9d-40bc-9cad-9cadfa3f19f7"
fn kazandu_mammoth() -> CardIndex {
    card_index("2ac1c95c-2a9d-40bc-9cad-9cadfa3f19f7")
}

/// A game stopped on Mystic Peak's own entry question, with the object the
/// land play made.
///
/// A whole game per half, the way the Witness pair is used: "you may pay 3
/// life" is answered once and for good on the way in, so the two answers are
/// two land drops and cannot share a turn.
///
/// The graveyard is stocked with one Dark Ritual for a reason that has
/// nothing to do with mana: it is the target the *Djinn's* enters-trigger
/// would want. `CardDef::abilities_for_face` gives a back face only what that
/// face prints, so the land carries no trigger at all — and with an empty
/// graveyard that could never have been seen: a mandatory target with nothing
/// legal to point at leaves nothing on the stack either way, so a land that
/// wrongly carried the Djinn's trigger would have looked exactly like one
/// that carries nothing.
fn a_mystic_peak_asking(seed: u64) -> (Engine<RegistryLookup>, PlayerId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, dark_ritual())
        .hand(0, &[pinnacle_monk()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    seed_graveyard(&mut engine, p0, 1);
    let land = play_land(&mut engine, p0, pinnacle_monk());
    (engine, p0, land)
}

// oracle_id = "da9e3910-9a1c-43a9-9138-ca971b2bccae"

/// Skyclave Cleric // Skyclave Basilica, the modal double-faced card whose
/// two faces are a creature and a land (CR 712.3).
fn skyclave_cleric() -> CardIndex {
    card_index("da9e3910-9a1c-43a9-9138-ca971b2bccae")
}

// oracle_id = "53542c79-a62a-4d6a-97db-5296e9c68302"
fn tangled_florahedron() -> CardIndex {
    card_index("53542c79-a62a-4d6a-97db-5296e9c68302")
}

/// Walks to `seat`'s **next** first main phase, through the kit's own
/// [`answer_one`].
///
/// [`reach_their_main_phase`] cannot serve here: it is [`pass_until`] with
/// the predicate "a first main phase whose active player is `seat`", and
/// both tests below already stand in exactly that, so it would return
/// without moving the game at all.
///
/// Crossing the turn boundary is then where `pass_until` itself gives out.
/// A seeded `starting_hand` *replaces* the opening draw, so the seat under
/// test holds one card and never overflows — but the **opponent** keeps the
/// seven it was dealt and draws an eighth on its own turn, CR 103.8a
/// skipping the first draw for the starting seat alone, and is asked to
/// discard at cleanup (CR 514.1). `pass_until` has no arm for that question
/// and `answer_one` does.
///
/// Answering it off the top of the hand is safe here, where
/// `walk_the_game_until` above had to intercept it: the cards handed back
/// are the opponent's filler Forests, and the card under test is on the
/// battlefield before any of it is asked.
#[track_caller]
fn florahedron_to_next_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let from = engine.state().turn.number;
    for _ in 0..200 {
        if engine.state().turn.number > from
            && matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return;
        }
        let (player, action) = match answer_one(engine) {
            Ok(pair) => pair,
            Err(rest) => panic!("the game stopped before {seat:?}'s next main: {rest:?}"),
        };
        engine
            .apply(player, action)
            .expect("every answer came out of the question that enumerated it");
    }
    panic!("never reached {seat:?}'s next main phase");
}

// oracle_id = "6bc668f4-8fc7-4aaf-891b-277d8328b376"
fn umara_wizard() -> CardIndex {
    card_index("6bc668f4-8fc7-4aaf-891b-277d8328b376")
}

// oracle_id = "0355249a-8e4e-41db-9cea-1b901faffbe6"
fn witch_enchanter() -> CardIndex {
    card_index("0355249a-8e4e-41db-9cea-1b901faffbe6")
}

// oracle_id = "cb814e16-acf7-41d5-a357-1323dcc369f3"
fn devoted_druid() -> CardIndex {
    card_index("cb814e16-acf7-41d5-a357-1323dcc369f3")
}

// oracle_id = "01546b7d-a233-4176-8843-d732074dc5b6"
fn doubling_season() -> CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}

// oracle_id = "3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f"
fn wall_of_roots() -> CardIndex {
    card_index("3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f")
}

// oracle_id = "9575d7ce-f26d-4b90-87a3-6329e9799572"
fn abandoned_air_temple() -> CardIndex {
    card_index("9575d7ce-f26d-4b90-87a3-6329e9799572")
}

// oracle_id = "3ecaefc8-ead2-47a3-a7ea-b030faab65a7"
fn quirion_ranger() -> CardIndex {
    card_index("3ecaefc8-ead2-47a3-a7ea-b030faab65a7")
}

// oracle_id = "62e7e7b1-9887-4d15-b0e5-a8ddc711bd88"
fn arcbound_ravager() -> CardIndex {
    card_index("62e7e7b1-9887-4d15-b0e5-a8ddc711bd88")
}

fn archon_of_emeria() -> CardIndex {
    card_index("ceef2d5a-77ea-4e56-9806-fd1a2d5be400")
}

fn aven_mindcensor() -> CardIndex {
    card_index("d9517c5d-66d0-4178-96fb-a8c04f311ad8")
}

// oracle_id = "10af9cd9-1700-48f9-97e1-61e239536fef"
fn brad_boimler_eager_ensign() -> CardIndex {
    card_index("10af9cd9-1700-48f9-97e1-61e239536fef")
}

// oracle_id = "a1cc5e37-b09a-4b7f-afd5-77c1c35aa425"
fn carrion_feeder() -> CardIndex {
    card_index("a1cc5e37-b09a-4b7f-afd5-77c1c35aa425")
}

// oracle_id = "cee583b7-7cc3-40ea-a227-b760839ec291"
fn dwalin_weaponmaster() -> CardIndex {
    card_index("cee583b7-7cc3-40ea-a227-b760839ec291")
}

fn endurance() -> CardIndex {
    card_index("c85d824b-c190-4d04-ab99-918ad0e6516c")
}

// oracle_id = "70a6f08e-854d-4e2f-9d8c-c45ec3231157"
fn faeburrow_elder() -> CardIndex {
    card_index("70a6f08e-854d-4e2f-9d8c-c45ec3231157")
}

fn geist_of_saint_thalia() -> CardIndex {
    card_index("ef32a4a9-14e2-4738-b4c2-53ce5e1d2a53")
}

fn gemrazer() -> CardIndex {
    card_index("3dfb0c0a-b68f-43b9-8475-28d0192fc4ed")
}

// oracle_id = "c1d6cce8-085f-42cb-8b0c-b6fbbf88b16a"
fn goblin_engineer() -> CardIndex {
    card_index("c1d6cce8-085f-42cb-8b0c-b6fbbf88b16a")
}

// oracle_id = "a8ddea1c-8d80-49c1-a5b4-630d5e51d66e"
fn hapatra_vizier_of_poisons() -> CardIndex {
    card_index("a8ddea1c-8d80-49c1-a5b4-630d5e51d66e")
}

fn kenrith_the_returned_king() -> CardIndex {
    card_index("d209b948-9afb-4fd1-a961-72c87282878c")
}

fn kess_dissident_mage() -> CardIndex {
    card_index("f5092c14-eec4-472c-999c-ba96c36b2fbb")
}

// oracle_id = "5470dcfa-4eff-43da-abf7-19922841f719"
fn kitchen_finks() -> CardIndex {
    card_index("5470dcfa-4eff-43da-abf7-19922841f719")
}

// oracle_id = "e9117015-1050-44dd-a46b-e7ffe2085fae"
fn ledger_shredder() -> CardIndex {
    card_index("e9117015-1050-44dd-a46b-e7ffe2085fae")
}

fn luminous_broodmoth() -> CardIndex {
    card_index("28c7c816-07e7-42fb-923c-bf149ba28b38")
}

// oracle_id = "51233ade-70cd-4539-9f41-5ffab761da54"
fn malevolent_hermit() -> CardIndex {
    card_index("51233ade-70cd-4539-9f41-5ffab761da54")
}

// oracle_id = "5d27c63e-d1ef-48af-b51d-01ebc6daeac9"
fn mikaeus_the_unhallowed() -> CardIndex {
    card_index("5d27c63e-d1ef-48af-b51d-01ebc6daeac9")
}

fn phyrexian_fleshgorger() -> CardIndex {
    card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
}

// oracle_id = "c739e180-2f14-41ed-8e7e-50b7df985f35"
fn rabbit_battery() -> CardIndex {
    card_index("c739e180-2f14-41ed-8e7e-50b7df985f35")
}

// oracle_id = "37108cd4-bbab-4ce3-9ed6-f60e8422e703"
fn ragavan_nimble_pilferer() -> CardIndex {
    card_index("37108cd4-bbab-4ce3-9ed6-f60e8422e703")
}

fn ranger_captain_of_eos() -> CardIndex {
    card_index("cada3481-cc2b-4412-b9b5-0436af53aad2")
}

fn renegade_rallier() -> CardIndex {
    card_index("6fa07b6c-f01a-4416-b0fc-986b0fc4e412")
}

// oracle_id = "68ca91ba-31fb-47e0-9b32-e4f3504cbbca"
fn safehold_elite() -> CardIndex {
    card_index("68ca91ba-31fb-47e0-9b32-e4f3504cbbca")
}

fn scavenging_ooze() -> CardIndex {
    card_index("1ff25f67-36a7-4cfa-a2b1-2135b5b6fb67")
}

// oracle_id = "d1961110-575b-4a1b-9cee-db0e1f0fdbc1"
fn scryb_ranger() -> CardIndex {
    card_index("d1961110-575b-4a1b-9cee-db0e1f0fdbc1")
}

fn strangleroot_geist() -> CardIndex {
    card_index("af12758f-4a7b-4156-8942-de4716aa0623")
}

// oracle_id = "e87906d2-db1a-4e19-b910-adb4eb339945"
fn urza_lord_high_artificer() -> CardIndex {
    card_index("e87906d2-db1a-4e19-b910-adb4eb339945")
}

fn yawgmoth_thran_physician() -> CardIndex {
    card_index("a1e232c0-dc38-47be-a5a0-f68bc1d86a29")
}

/// Seat 1's god dies and comes back as its Temple under seat 1, its owner;
/// seat 0 steals the Temple for `duration` and turns it over with its
/// "{2}{U}, {T}: Transform this land". Hands back the table and the Temple.
fn a_temple_stolen_and_turned_over(
    duration: baylee_cards_dsl::Duration,
) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), island(), island(), island()],
        )
        .hand(0, &[heroes_downfall()])
        .battlefield(1, &[ojer_pakpatiq_deepest_epoch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let god = on_battlefield(&engine, p1, ojer_pakpatiq_deepest_epoch()).expect("the god is out");
    let lands = |engine: &Engine<RegistryLookup>, card: CardIndex| -> Vec<ObjectId> {
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
            })
            .collect()
    };
    let swamps = lands(&engine, swamp());
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    cast_with_floating(&mut engine, p0, heroes_downfall());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![god],
                players: vec![],
            },
        )
        .expect("a target the spell offered");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let temple =
        on_battlefield(&engine, p1, ojer_pakpatiq_deepest_epoch()).expect("it came back at once");
    assert_eq!(
        engine
            .state()
            .object(temple)
            .map(|o| (o.face_index, o.owner, o.controller)),
        Some((1, p1, p1)),
        "\"…under its owner's control\": the Temple, seat 1's"
    );
    let temple_was = identity(&engine, temple);

    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        let filter = crate::effects::EffectFilter::object(state, temple);
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: p0,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Control,
            timestamp,
            duration,
            filter,
            modifier: baylee_cards_dsl::Modifier::GainControl,
        });
        state.refresh_characteristics();
    }
    engine.refresh_offer();
    let islands = lands(&engine, island());
    tap_mana_where(&mut engine, p0, |id| islands.contains(&id));
    activate(&mut engine, p0, ojer_pakpatiq_deepest_epoch(), 1);
    pass_until(&mut engine, stack_is_empty);

    let back = engine
        .state()
        .object(temple)
        .expect("the same arena handle");
    assert_eq!(
        (back.zone, back.face_index),
        (crate::zone::Zone::Battlefield, 0),
        "turned over into the god"
    );
    assert_eq!(
        identity(&engine, temple),
        temple_was,
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );
    (engine, temple)
}

/// Temple of Cyclical Time's "{2}{U}, {T}: Transform this land" turns the
/// same permanent over (CR 701.27a), and a permanent that transforms is not
/// a new object: every effect that applied to it goes on applying
/// (CR 712.18). So the god seat 0 turned its stolen Temple into is still
/// seat 0's by the steal that held the Temple, still seat 1's by default and
/// still seat 1's card (CR 108.3), and still tapped from the activation.
///
/// The stand-in that exiled the land and returned the god (#206) gave back a
/// new object whose own default was the activator, which kept it with the
/// thief after the steal had ended (the next test).
#[test]
fn a_stolen_temple_of_cyclical_time_turns_back_into_the_god_under_the_thief() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (engine, temple) =
        a_temple_stolen_and_turned_over(baylee_cards_dsl::Duration::Indefinitely);
    let god = engine
        .state()
        .object(temple)
        .expect("the same arena handle");
    assert_eq!(
        (god.owner, god.controller, god.base_controller),
        (p1, p0, p1),
        "stolen by the same effect, seat 1's by default, and seat 1's card"
    );
    assert!(
        is_tapped(&engine, temple),
        "tapped for the activation, and turning over is not entering"
    );
}

/// A steal "until end of turn" ends in the cleanup step (CR 514.2) whatever
/// face the stolen permanent shows by then. Seat 0 turns the Temple it stole
/// for the turn into the god, and seat 1 has its god back once the turn is
/// over. The stand-in's god was a new object that the steal no longer held,
/// under seat 0 by default, so it stayed with the thief.
#[test]
fn a_temple_stolen_until_end_of_turn_goes_home_as_the_god_it_became() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, temple) =
        a_temple_stolen_and_turned_over(baylee_cards_dsl::Duration::UntilEndOfTurn);
    assert_eq!(
        engine.state().object(temple).map(|o| o.controller),
        Some(p0),
        "seat 0's for the rest of the turn"
    );
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine
            .state()
            .object(temple)
            .map(|o| (o.zone, o.face_index, o.controller)),
        Some((crate::zone::Zone::Battlefield, 0, p1)),
        "the steal ended with the turn, and the god went home"
    );
}

fn courser_of_kruphix() -> CardIndex {
    card_index("46779609-4fa7-4fd2-b5b4-7d4d749339e6")
}

fn dragon_s_rage_channeler() -> CardIndex {
    card_index("0c016ccc-a341-4b76-87ba-69c639d2746d")
}

fn dryad_of_the_ilysian_grove() -> CardIndex {
    card_index("bdbde5d0-f5e4-44da-b27c-b4ad6f374cc9")
}

fn enduring_vitality() -> CardIndex {
    card_index("3577c47e-76d3-4659-b922-31c4b74be3a0")
}

fn flamekin_harbinger() -> CardIndex {
    card_index("d6585e30-4ca0-4701-b274-b24f3508dd97")
}

fn golden_guardian() -> CardIndex {
    card_index("58afb897-4d57-4b53-a5c3-b532cb3d5180")
}

fn hexdrinker() -> CardIndex {
    card_index("69bc2afd-9f53-47f2-b9c8-f12732784e10")
}

fn hexdrinker_s_bonesplitter() -> CardIndex {
    card_index("452e3f5f-ce17-4682-966b-5cc100210aee")
}

fn ignoble_hierarch() -> CardIndex {
    card_index("c8de43a3-ebd3-4000-b343-a6ffed11d34d")
}

fn altered_ego() -> CardIndex {
    card_index("7c35f3fd-c64e-4944-a4d5-37ce916d23c3")
}

/// Casts Badgermole Cub off two of p0's Forests and earthbends `land`,
/// answering the enters trigger's target question as a player would.
#[track_caller]
fn earthbend_with_the_cub(engine: &mut Engine<RegistryLookup>, pay: [ObjectId; 2], land: ObjectId) {
    let p0 = PlayerId::new(0);
    for forest in pay {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: forest })
            .expect("a Forest taps for {G}");
    }
    cast_with_floating(engine, p0, badgermole_cub());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched");
    };
    assert!(options.contains(&land), "\"target land you control\"");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: vec![],
            },
        )
        .expect("the land");
    pass_until(engine, stack_is_empty);
}

/// How many `ZoneChanged` entries moved `object` from `from` to `to`.
fn moves_of(engine: &Engine<RegistryLookup>, object: ObjectId, from: Zone, to: Zone) -> usize {
    engine
        .journal()
        .entries()
        .iter()
        .filter(|e| {
            matches!(e.event, crate::event::GameEvent::ZoneChanged { object: o, from: f, to: t, .. }
                if o == object && f == from && t == to)
        })
        .count()
}

/// Whether an earthbend watch is still registered.
fn earthbend_watches(engine: &Engine<RegistryLookup>) -> usize {
    engine
        .state()
        .delayed
        .iter()
        .filter(|d| matches!(d.when, crate::state::DelayedWhen::DiesOrIsExiled { .. }))
        .count()
}

fn birgi_god_of_storytelling() -> CardIndex {
    card_index("fb81e4d3-1d8c-4779-be62-87cf49277e51")
}

fn bristly_bill_spine_sower() -> CardIndex {
    card_index("d3b2d8a2-d3bc-448c-9cf6-6bead6010c28")
}

fn lake_town_lookout() -> CardIndex {
    card_index("cf765efe-884c-48e2-9edb-9d45cf2756dd")
}

fn murderous_rider() -> CardIndex {
    card_index("1080c5b5-6651-4c6a-93e6-099fbe389e26")
}

/// Bolts `victim` from `seat`'s hand and walks until it is in a graveyard, so
/// its dies trigger is the one waiting on the stack.
fn bolt_to_death(engine: &mut Engine<RegistryLookup>, seat: PlayerId, victim: ObjectId) {
    let bolt = in_hand(engine, seat, lightning_bolt()).expect("the Bolt is in hand");
    let mountains = all_of(engine, seat, mountain());
    tap_mana_where(engine, seat, |id| mountains.contains(&id));
    engine
        .apply(seat, PlayerAction::CastSpell { card: bolt })
        .expect("the Bolt is castable");
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![victim],
                players: vec![],
            },
        )
        .expect("the creature is a legal target");
    pass_until(engine, |e| {
        e.state()
            .object(victim)
            .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard)
    });
}

fn engine_card_is(engine: &Engine<RegistryLookup>, id: ObjectId, card: CardIndex) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
}

fn nantuko_mentor() -> CardIndex {
    card_index("b79378e7-99db-403f-8f63-4d71ebdb3f6c")
}

fn noble_hierarch() -> CardIndex {
    card_index("98aa9424-5912-4bd6-9300-b3972a31d8af")
}

fn rishkar_peema_renegade() -> CardIndex {
    card_index("761021ce-4559-464e-aa03-85c2fe78e267")
}

fn tireless_provisioner() -> CardIndex {
    card_index("ab8d5f5c-1976-4f77-8ed2-8d28ee666741")
}

fn wayward_swordtooth() -> CardIndex {
    card_index("3875aef0-3102-4fbf-be90-e4139f7a2348")
}

fn spirit_of_the_labyrinth() -> CardIndex {
    card_index("1463795b-ec0c-44d6-ae1a-55f78d9843ec")
}

fn counsel_of_the_soratami() -> CardIndex {
    card_index("62ddc5ae-ced9-4319-854c-1a114c6afc3f")
}

fn supreme_verdict() -> CardIndex {
    card_index("0230de18-8d15-4cfa-9d42-7ccddd9f9570")
}

fn leovold_emissary_of_trest() -> CardIndex {
    card_index("d5d91377-fd66-4dbe-a092-07f2ea379ca7")
}

/// Aims an opponent's Wintermoon Mesa ("tap two target lands") at two lands.
fn mesa_at(engine: &mut Engine<RegistryLookup>, seat: PlayerId, lands: [ObjectId; 2]) {
    activate(engine, seat, wintermoon_mesa(), 1);
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!(
            "the Mesa asks for its two lands, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, seat);
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: lands.to_vec(),
                players: vec![],
            },
        )
        .expect("two lands are what the Mesa targets");
}

/// Every battlefield object of `card` a seat controls, in zone order.
fn all_of(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// The opponent's answer to a spell on the stack: pass to them, tap two of
/// their Islands, and point a Counterspell at it.
fn counter_with_two(engine: &mut Engine<RegistryLookup>, spell: ObjectId, islands: &[ObjectId]) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_mana_where(engine, p1, |id| islands.contains(&id));
    let answer = in_hand(engine, p1, counterspell()).expect("a Counterspell in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: answer })
        .expect("two Islands pay {U}{U}");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("a counterspell may point at what it cannot counter");
    pass_until(engine, stack_is_empty);
}

/// Attacks `p1` with `p0`'s Tyrranax Rex, has `p1` answer the blockers
/// question with `blocks`, and walks to the end of combat.
fn rex_attacks(
    engine: &mut Engine<RegistryLookup>,
    rex: ObjectId,
    blocks: Vec<(ObjectId, ObjectId)>,
) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let offered = attack_and_collect_blocks(engine, rex, p1);
    for (blocker, attacker) in &blocks {
        assert!(
            offered
                .iter()
                .any(|o| o.blocker == *blocker && o.attackers.contains(attacker)),
            "the block is on the offer: {offered:?}"
        );
    }
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: blocks })
        .expect("the blocks came off the offer");
    pass_until(engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd || e.state().turn.active != p0
    });
}

/// Passes to cascade's offer and names the card it offers.
fn cascade_offer(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo {
        prompt: crate::choice::YesNoPrompt::CastWithoutPaying { card },
        ..
    } = engine.pending().clone()
    else {
        panic!("cascade's offer, got {:?}", engine.pending())
    };
    card
}

/// Plays a land and answers Nissa's colour, then lets the trigger finish.
fn landfall_for_nissa(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    play_land(engine, seat, forest());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(seat, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    pass_until(engine, stack_is_empty);
}

fn bottle_gnomes() -> CardIndex {
    card_index("54b5e429-7a44-480d-bea4-4f8eeb7449b5")
}

fn brass_secretary() -> CardIndex {
    card_index("0d8c2d7b-7cce-4115-a3f8-18c23731544e")
}

fn cathodion() -> CardIndex {
    card_index("fcd4f816-2de1-4b30-82fb-cb87f45747ea")
}

fn cobalt_golem() -> CardIndex {
    card_index("1e50af57-e49d-4b94-a370-44846d9f6b33")
}

fn coiled_tinviper() -> CardIndex {
    card_index("b0bdc033-29f0-4dcb-87ff-ccd56126f7e5")
}

fn copper_myr() -> CardIndex {
    card_index("8b52f30c-5e38-4333-88ab-901b37105b36")
}

fn coretapper() -> CardIndex {
    card_index("66e37011-f3d2-41ed-8e09-7bba9464e8c3")
}

fn crenellated_wall() -> CardIndex {
    card_index("ca9e41e8-7830-4721-b131-07d0b60297ad")
}

fn dancing_scimitar() -> CardIndex {
    card_index("82b13601-d460-45c5-94a1-07e146d463a9")
}

fn dragon_engine() -> CardIndex {
    card_index("727a6474-d5b5-42ab-ace3-352d28f499eb")
}

fn elf_replica() -> CardIndex {
    card_index("2dda5011-aaa1-48d0-afa8-76998278ab75")
}

fn energizer() -> CardIndex {
    card_index("708b0321-7de9-4ffd-9d7a-9f4813dd542e")
}

fn goblin_replica() -> CardIndex {
    card_index("0c5a7772-8358-4549-a583-f54246869e20")
}

fn gold_myr() -> CardIndex {
    card_index("bd6af7b3-b30f-4a65-a18f-8655f778e76a")
}

fn hematite_golem() -> CardIndex {
    card_index("f2e1bb32-de4c-4abc-ac15-6e6deaa8fe5f")
}

fn hopping_automaton() -> CardIndex {
    card_index("7848e040-e1b4-4f81-922e-b1eca8fe4398")
}

fn iron_myr() -> CardIndex {
    card_index("6c5cbab6-ee27-46f5-97a7-df85698d1e9f")
}

fn leaden_myr() -> CardIndex {
    card_index("f62cabf0-df0d-4c4f-a93a-9340967d1775")
}

fn manakin() -> CardIndex {
    card_index("d2343af0-468b-42bc-8a0c-347c10f7e2f3")
}

fn myr_moonvessel() -> CardIndex {
    card_index("3e922661-80df-4e84-a12a-524bc74e6c9d")
}

fn nim_replica() -> CardIndex {
    card_index("68c3aa66-85f2-494c-8fb2-ad0348814506")
}

fn patagia_golem() -> CardIndex {
    card_index("85308fb8-e5a1-4ff1-8606-283cf2056895")
}

fn rustspore_ram() -> CardIndex {
    card_index("a45b3934-1c9b-4cff-98b1-c9ac2f7759ea")
}

fn shifting_wall() -> CardIndex {
    card_index("7c852dfe-8238-461e-814e-9667807f2cf5")
}

fn silver_myr() -> CardIndex {
    card_index("66e8f7f8-3a6d-46ba-837c-b9713ddf7f40")
}

fn steel_wall() -> CardIndex {
    card_index("5ccb57e1-ca94-4b5a-8e5f-b8b5e692cfb9")
}

fn straw_golem() -> CardIndex {
    card_index("6e48981b-8971-4b7c-ae05-29c7e4091a05")
}

fn thermal_navigator() -> CardIndex {
    card_index("355c2840-2cb8-431a-bcdc-f4b8824a1f5d")
}

fn wall_of_spears() -> CardIndex {
    card_index("ed836d84-ff1e-4af8-b4b8-314569b3faec")
}

fn yotian_soldier() -> CardIndex {
    card_index("645571f1-a775-4834-8b79-8852d38c9587")
}

fn adarkar_sentinel() -> CardIndex {
    card_index("bd372a53-48f9-4e9a-ab00-0f9c4606d452")
}

fn agent_of_stromgald() -> CardIndex {
    card_index("099fd4b3-92fd-4b5f-bb4f-eefa6f09700c")
}

fn akki_avalanchers() -> CardIndex {
    card_index("5a98decb-30ba-4c34-a88a-424b7f4e03bc")
}

fn ali_baba() -> CardIndex {
    card_index("ef3fdfea-330d-4948-94ec-67e344b48086")
}

fn ana_disciple() -> CardIndex {
    card_index("f543dfcd-015e-48bc-851d-08002d0241fa")
}

fn arachnoid() -> CardIndex {
    card_index("7116db6d-82ea-4675-a0cd-73aaf94be43f")
}

fn armorer_guildmage() -> CardIndex {
    card_index("7b75b74c-6003-4b34-bae3-2c9e2f880042")
}

fn auriok_transfixer() -> CardIndex {
    card_index("0ad5f4c5-f6d2-4e96-af65-5488d9711113")
}

fn avacyn_s_pilgrim() -> CardIndex {
    card_index("069f6530-e65c-4d52-85f3-e0a2acd148c5")
}

fn aven_envoy() -> CardIndex {
    card_index("73cd8461-7930-4e2d-9a14-4d10db91e93e")
}

fn coal_golem() -> CardIndex {
    card_index("64b63847-27dd-469b-aad3-58e061f92817")
}

fn composite_golem() -> CardIndex {
    card_index("784970de-ef8c-4672-b5d0-24a4b0a978a3")
}

fn crosis_s_attendant() -> CardIndex {
    card_index("6223b1c0-bfe0-490c-b2d4-28537b05f571")
}

fn darigaaz_s_attendant() -> CardIndex {
    card_index("c0ad16d7-2550-47e0-8581-0e22b27cb0d0")
}

fn darksteel_gargoyle() -> CardIndex {
    card_index("73010421-374f-458e-aa88-248ef8ae4f8b")
}

fn dromar_s_attendant() -> CardIndex {
    card_index("91d36a9d-3a3f-4cb7-837d-b1e8f6730512")
}

fn ebony_rhino() -> CardIndex {
    card_index("3d2b5201-c2bf-4753-89ad-543795740216")
}

fn flowstone_thopter() -> CardIndex {
    card_index("a3169d1a-fd8d-474e-9e9f-1bd7db510e8b")
}

fn henge_guardian() -> CardIndex {
    card_index("71744428-65ae-42ac-893d-0802fd41e9b4")
}

fn igneous_golem() -> CardIndex {
    card_index("7c4408eb-b3e2-407f-9018-197787edeae7")
}

fn limestone_golem() -> CardIndex {
    card_index("0dddf5a9-fdf3-49e5-8103-a8cfd62f55c6")
}

fn lotus_guardian() -> CardIndex {
    card_index("143d7a32-c15d-4a1f-ad81-cf1e6aa92bf3")
}

fn malachite_golem() -> CardIndex {
    card_index("54e71732-6575-48f4-838b-35d44eba51cf")
}

fn mantis_engine() -> CardIndex {
    card_index("57bea272-5b38-444e-acc2-6104af2ece22")
}

fn razormane_masticore() -> CardIndex {
    card_index("ee45b588-b424-49d3-9e37-270f2ae9490d")
}

fn rith_s_attendant() -> CardIndex {
    card_index("781542b7-155b-4a66-aa52-eec0ebb29bb2")
}

fn su_chi() -> CardIndex {
    card_index("9a28d53e-c789-47de-8f3e-a251843ac596")
}

fn telethopter() -> CardIndex {
    card_index("8202ff77-9afc-4e5b-895b-471bbfa67cce")
}

fn titanium_golem() -> CardIndex {
    card_index("84fa0c22-4e11-4d57-9a45-633f0e1686de")
}

fn treva_s_attendant() -> CardIndex {
    card_index("ff7d927f-0a01-4243-ad00-c65686ee86bb")
}

fn bile_urchin() -> CardIndex {
    card_index("7d406aa7-636a-45c1-903f-11c2ce2ef3e3")
}

fn birds_of_paradise() -> CardIndex {
    card_index("d3a0b660-358c-41bd-9cd2-41fbf3491b1a")
}

fn blood_celebrant() -> CardIndex {
    card_index("695ce90b-a5a2-4d8c-ac05-07f04010603e")
}

fn blood_pet() -> CardIndex {
    card_index("e05c6c80-a91a-45e0-b991-0014fd5a6472")
}

fn boros_recruit() -> CardIndex {
    card_index("00abb43b-550f-42d2-9049-61ac56ff5d8d")
}

fn cabal_trainee() -> CardIndex {
    card_index("fce68545-c31d-4dc2-9f1b-aadf924a07d1")
}

fn ceta_disciple() -> CardIndex {
    card_index("1f085c2d-bd1b-4889-882f-367e0e970f44")
}

fn child_of_thorns() -> CardIndex {
    card_index("f94002a1-582f-4b9d-a4f7-dae816f21c1d")
}

fn dedicated_martyr() -> CardIndex {
    card_index("ca39af65-985d-4ab1-874b-9e8d3455c349")
}

fn defiant_elf() -> CardIndex {
    card_index("6faa29c4-582a-42d5-b80c-4dc0bd738493")
}

fn dega_disciple() -> CardIndex {
    card_index("c6c53277-daad-4aa3-b29a-11378d550651")
}

fn devout_monk() -> CardIndex {
    card_index("8a0f4e8e-b541-4327-80fe-4fecc23e0df3")
}

fn druid_lyrist() -> CardIndex {
    card_index("b32a10ba-528c-4dc1-9828-b33f5d5a3091")
}

fn electric_eel() -> CardIndex {
    card_index("ad8213db-074d-475c-b94a-9f1e81c8f4fd")
}

fn elves_of_deep_shadow() -> CardIndex {
    card_index("20347559-95a9-4689-bb79-c5bb3809b719")
}

fn elvish_herder() -> CardIndex {
    card_index("7832b90a-23b9-448c-9019-8ee2e8dfab31")
}

fn elvish_lookout() -> CardIndex {
    card_index("9e9aea99-1eee-4bb6-9043-a1ac2884d071")
}

fn elvish_lyrist() -> CardIndex {
    card_index("e0de1c9b-f71f-4498-a968-d3c635cb4c5c")
}

fn elvish_mystic() -> CardIndex {
    card_index("3f3b2c10-21f8-4e13-be83-4ef3fa36e123")
}

fn elvish_scrapper() -> CardIndex {
    card_index("81b2242b-40c9-475a-a198-28dd348235ec")
}

fn flying_men() -> CardIndex {
    card_index("b5666c68-059d-4e32-9b04-548b3058430d")
}

fn frostling() -> CardIndex {
    card_index("6f6ac768-4c97-44db-933a-f2d442e7f665")
}

fn fyndhorn_elves() -> CardIndex {
    card_index("df317532-7d36-40fd-938f-e972749c8792")
}

fn goblin_balloon_brigade() -> CardIndex {
    card_index("10bc98b0-3fdc-46d1-8d3b-6d160e9dd62f")
}

fn goblin_digging_team() -> CardIndex {
    card_index("8408f2a1-e321-43f5-a7d1-1911eba9d706")
}

fn goblin_sledder() -> CardIndex {
    card_index("ae93292a-458a-441b-a7de-4b10f0d20cc0")
}

fn granger_guildmage() -> CardIndex {
    card_index("0230fdda-d916-457e-9dfd-61783fcf4652")
}

fn honor_guard() -> CardIndex {
    card_index("4ba82d8e-1857-40a9-8b42-5d8348c68859")
}

fn icatian_priest() -> CardIndex {
    card_index("3a9d29ce-e8e2-402f-b0c2-873f70418234")
}

fn icatian_scout() -> CardIndex {
    card_index("b1a9f49a-b0e2-481f-806d-0bc321517a4f")
}

fn infantry_veteran() -> CardIndex {
    card_index("42798e2b-9a5c-4f93-8f77-b7bb7a916d07")
}

fn ivy_elemental() -> CardIndex {
    card_index("38517711-e570-4337-b269-addcf8bfdd74")
}

fn kitsune_diviner() -> CardIndex {
    card_index("2281ab3f-fad7-4b42-9621-5c35fec4c0f6")
}

fn kris_mage() -> CardIndex {
    card_index("d53623b0-f7d8-401f-a8f1-80034f6f20df")
}

fn lantern_kami() -> CardIndex {
    card_index("bddb6b9b-5120-48d2-a57f-eb5d46d38cec")
}

fn leonin_elder() -> CardIndex {
    card_index("f85a1752-c6fa-481b-997d-6b5dd4c6b54b")
}

fn maggot_carrier() -> CardIndex {
    card_index("e4ad4b81-3685-4f95-84c0-755263b9d3b1")
}

fn manta_riders() -> CardIndex {
    card_index("d04f1f62-5363-4bf2-bef2-fb3387fc5f48")
}

fn mogg_fanatic() -> CardIndex {
    card_index("7efd5d62-6da2-428a-98e3-5b842f668656")
}

fn mogg_raider() -> CardIndex {
    card_index("240746f5-caab-46b9-914f-c2c7ecce6ca9")
}

fn mogg_sentry() -> CardIndex {
    card_index("ac6f490b-6cf7-4afc-910e-4c8b51fcb2de")
}

fn molting_harpy() -> CardIndex {
    card_index("c8dcb8d8-ae27-43ac-88e7-e59f2656e0ce")
}

fn mountain_bandit() -> CardIndex {
    card_index("e3638c54-e494-47d2-b614-94c9d9add85a")
}

fn nezumi_shadow_watcher() -> CardIndex {
    card_index("3eae5419-b686-4321-a89d-cb602c487bfc")
}

fn mutavault() -> CardIndex {
    card_index("6b3cc59a-7ea5-4eb5-9bf9-5a9c07f80e2b")
}

fn orcish_lumberjack() -> CardIndex {
    card_index("383e3fe4-8558-4561-8632-6eadb5d5963c")
}

fn orochi_leafcaller() -> CardIndex {
    card_index("081db161-1ad0-4ee7-9d34-c415984080f0")
}

fn phyrexian_battleflies() -> CardIndex {
    card_index("58ed1294-ba7d-43a1-b988-8d78d9fa0e3e")
}

fn plagued_rusalka() -> CardIndex {
    card_index("d6912f52-e478-4cc6-a34c-67e372abc4fa")
}

fn plated_sliver() -> CardIndex {
    card_index("89d9641c-84a4-41df-af1f-5b552305e895")
}

fn raging_goblin() -> CardIndex {
    card_index("30997b43-fc13-41d3-8064-1ccc2cb6fd2b")
}

fn rogue_elephant() -> CardIndex {
    card_index("bed7ac55-fe40-46d0-bc22-1c8d11f41459")
}

fn sandbar_merfolk() -> CardIndex {
    card_index("7d6e5a88-a7fd-466c-a9f8-5015e3400176")
}

fn scavenger_folk() -> CardIndex {
    card_index("74f361f8-a0d8-4824-9380-61469d6a19cf")
}

fn serra_zealot() -> CardIndex {
    card_index("989a7353-8d3d-4ea2-ab5e-8535d95dddae")
}

fn sewer_rats() -> CardIndex {
    card_index("bf526a0d-65cc-445f-b2b8-19a2cfdd836b")
}

fn shaper_guildmage() -> CardIndex {
    card_index("dfbce321-d3c1-4b76-a3fa-27f875015085")
}

fn shield_mate() -> CardIndex {
    card_index("98e7bfd4-fcb6-47b1-ad0b-c54b85dc2a79")
}

fn skirk_prospector() -> CardIndex {
    card_index("c18013e4-0b99-44e3-a2b2-027ace68723a")
}

fn soul_warden() -> CardIndex {
    card_index("f3fad295-1af2-4ecc-8546-b121ad6be27b")
}

fn stone_throwing_devils() -> CardIndex {
    card_index("124c8663-21f3-4cd8-a060-9d04be35c43f")
}

fn advance_scout() -> CardIndex {
    card_index("831e2185-0bb2-45bb-99cd-d2091bad86f0")
}

fn agent_of_shauku() -> CardIndex {
    card_index("d2e0d739-b441-4173-a8f1-b831a19ea98e")
}

fn akki_raider() -> CardIndex {
    card_index("bf9c5c0d-86a5-467a-863c-653279f2e6e7")
}

fn akki_rockspeaker() -> CardIndex {
    card_index("01cd52f9-7fac-4396-bec7-d06bf683e011")
}

fn alaborn_grenadier() -> CardIndex {
    card_index("69c4429e-a8a4-49c3-bb43-9dce4cc6b90a")
}

// oracle_id = "61daa88d-fca4-4019-97f3-568346157ba6"
fn alaborn_musketeer() -> CardIndex {
    card_index("61daa88d-fca4-4019-97f3-568346157ba6")
}

fn anaba_ancestor() -> CardIndex {
    card_index("9d2e7422-397a-47d4-9a1e-1957c7db9a18")
}

fn angelic_wall() -> CardIndex {
    card_index("4502b24f-604b-4e36-9168-31c1a1ab4dab")
}

fn argothian_enchantress() -> CardIndex {
    card_index("11f17f85-ca97-4551-838f-7cb32f0e5f10")
}

fn armored_pegasus() -> CardIndex {
    card_index("f097a059-5505-4c3c-b879-7853ab6972ed")
}

fn atog() -> CardIndex {
    card_index("3095f2cf-05c3-4381-929f-c526b6fc30a7")
}

fn auratog() -> CardIndex {
    card_index("fe47535e-e50c-49ad-beda-46750d33e6f4")
}

fn fastbond() -> CardIndex {
    card_index("e27193b7-1a47-4555-865d-b1fd4c6d597f")
}

fn basal_thrull() -> CardIndex {
    card_index("9da50130-3f83-4968-983c-7dcba257cf1b")
}

fn bay_falcon() -> CardIndex {
    card_index("709e99f1-288b-49e0-80a1-f817b457a2cf")
}

fn benalish_trapper() -> CardIndex {
    card_index("d45c6c3f-6079-40c1-9083-06f2f2431bcc")
}

fn blighted_shaman() -> CardIndex {
    card_index("2f14e92d-307f-4854-b7ec-a2f6ce03dced")
}

fn blood_artist() -> CardIndex {
    card_index("310f141c-7f37-4729-aed6-dd9c09db448d")
}

fn bog_imp() -> CardIndex {
    card_index("45b94e3c-a905-435b-aee5-bec9239fd24c")
}

fn bog_initiate() -> CardIndex {
    card_index("23f93411-f83c-4ed2-abed-99cf905f7d7f")
}

fn stormscape_apprentice() -> CardIndex {
    card_index("cf2bacd3-b1ac-4863-b9c7-1e0a8d4854f3")
}

fn suntail_hawk() -> CardIndex {
    card_index("a8d31e2f-7b2e-4135-8074-9e6ef778bd80")
}

fn sylvan_safekeeper() -> CardIndex {
    card_index("bddf8f4a-3149-4dd6-a9e5-7747e7e45a1c")
}

fn thornscape_apprentice() -> CardIndex {
    card_index("ebb68e03-b2a5-4c5c-8e3b-968cd241d1f0")
}

fn thunderscape_apprentice() -> CardIndex {
    card_index("34e325f0-b66f-44b9-a25c-da88357dbac4")
}

fn tireless_tribe() -> CardIndex {
    card_index("5dfbcdb4-d2ad-477d-b37d-db4725410b27")
}

fn tree_monkey() -> CardIndex {
    card_index("d7318464-e400-424f-ae6d-a9070c64df04")
}

fn tundra_wolves() -> CardIndex {
    card_index("2155864e-5788-4e72-a800-e2cf25cf59a7")
}

fn viridian_acolyte() -> CardIndex {
    card_index("9db7d8f0-0712-4e32-825c-1339e7a8d768")
}

fn wall_of_wood() -> CardIndex {
    card_index("aa5e6894-8c97-46f8-a0fd-07c6db51e9a7")
}

/// Wirewood Symbiote prints one line: "Return an Elf you control to its
/// owner's hand: Untap target creature. Activate only once each turn." One
/// activation reads both halves of it, because the two questions are
/// different questions: the target menu must offer every creature on the
/// table — the sentence says nothing about who controls the thing being
/// untapped — while the cost menu must offer exactly the Elves this seat
/// controls and neither the Elf across the table nor the Symbiote, which is
/// an Insect. Both Elves are tapped when the cost is paid, which is the
/// other half of the printed price: CR 118.3's "untapped" belongs to a tap
/// cost, and a return takes a permanent in either state. And the Elf still
/// standing afterwards is what makes the second activation's absence a
/// reading of "only once each turn" rather than of an empty board.
fn wirewood_symbiote() -> CardIndex {
    card_index("67a52a74-9474-4f4e-8785-6ff54078a8ca")
}

fn boros_guildmage() -> CardIndex {
    card_index("266c2f49-82eb-48e9-a7ea-e0da787fa40e")
}

fn brine_shaman() -> CardIndex {
    card_index("3e3e2f71-2159-4f83-a9c1-a67ecac8a711")
}

fn canopy_spider() -> CardIndex {
    card_index("37f3733e-cc4e-4d84-b29b-d474f6e254a2")
}

fn capashen_knight() -> CardIndex {
    card_index("e295d207-128f-419a-866a-c9ac6248c199")
}

fn capashen_unicorn() -> CardIndex {
    card_index("52793400-bc83-402a-9609-928a4d3ac812")
}

fn cephalid_scout() -> CardIndex {
    card_index("0ee042ce-7cb2-47a8-9a48-6ffc07ba07b3")
}

fn citanul_druid() -> CardIndex {
    card_index("0d292575-c4ff-427a-953a-abc89e40caa6")
}

fn consumptive_goo() -> CardIndex {
    card_index("4bbfa49e-4460-439e-8f31-2e54316fbb5f")
}

fn crystalline_sliver() -> CardIndex {
    card_index("ba3aa1eb-722a-47d3-83be-96daddb50265")
}

fn dakmor_bat() -> CardIndex {
    card_index("724d8247-33b1-42bf-a827-7b66e45504c1")
}

fn deepwood_drummer() -> CardIndex {
    card_index("b2bbb4e5-05b1-49e5-ad02-7384e115e06b")
}

fn dwarven_lieutenant() -> CardIndex {
    card_index("06e18c42-d158-494d-979c-c2ac074a8ee3")
}

fn dwarven_miner() -> CardIndex {
    card_index("0935377b-384f-4e4c-9fbb-8d2a7d5dc280")
}

fn earthblighter() -> CardIndex {
    card_index("86b8fdad-0025-4b6f-8527-23686820a5cd")
}

fn elvish_archers() -> CardIndex {
    card_index("534ce8e2-53a1-407c-9881-d3b5347d43c3")
}

fn elvish_vanguard() -> CardIndex {
    card_index("4163aa30-7e3d-424f-b003-f4a300f0071e")
}

fn emerald_dragonfly() -> CardIndex {
    card_index("b067aa04-5977-4371-85d1-78d2884df03f")
}

fn femeref_enchantress() -> CardIndex {
    card_index("8b983a46-580f-4a94-8ef3-5253e503148e")
}

fn fire_sprites() -> CardIndex {
    card_index("fc5e42b5-4da2-4777-828b-138c0a5d234f")
}

fn fledgling_djinn() -> CardIndex {
    card_index("2c73ef77-ae58-401f-8747-538c4cd075d0")
}

fn floodbringer() -> CardIndex {
    card_index("6c8bba8b-44de-4bbf-bf6a-3045e7313995")
}

fn foul_imp() -> CardIndex {
    card_index("25c04be3-f2e0-4f45-a1ca-9e75ffffa9b5")
}

fn gaea_s_skyfolk() -> CardIndex {
    card_index("dd0da475-55d0-462e-bfac-6eeb5d39a099")
}

fn giant_tortoise() -> CardIndex {
    card_index("2dd50d7f-941f-4deb-a15c-ee2357844c35")
}

fn goblin_masons() -> CardIndex {
    card_index("cfec9d17-2d11-4b89-8b30-84bd25a38466")
}

fn goblin_striker() -> CardIndex {
    card_index("c991d1b8-3adb-4854-9fd3-83f06aeb3941")
}

fn grimclaw_bats() -> CardIndex {
    card_index("161ebbc8-b8f9-4ac4-80f3-e290070a1e30")
}

fn guardian_of_solitude() -> CardIndex {
    card_index("712ec339-ffa8-43bb-a826-a7b0f0120b1f")
}

fn heart_sliver() -> CardIndex {
    card_index("6dd1f375-e8f2-4725-8fbd-9750c4860820")
}

fn heart_warden() -> CardIndex {
    card_index("babcc551-39bb-4b1c-92a0-e60b236a59b2")
}

fn humble_budoka() -> CardIndex {
    card_index("9508efe6-0528-4a13-9961-c2a1b05085a6")
}

fn icatian_lieutenant() -> CardIndex {
    card_index("51e9af95-745b-45b9-b870-7f768578fa4f")
}

fn ironshell_beetle() -> CardIndex {
    card_index("693ca077-2b91-4de1-8136-8fe45f8ea5a7")
}

fn kami_of_ancient_law() -> CardIndex {
    card_index("e937e0f5-ca35-4dce-9f05-8f2075731628")
}

fn king_suleiman() -> CardIndex {
    card_index("97f548b1-6398-4dc7-b5eb-da5a3f24ddb7")
}

fn kobold_drill_sergeant() -> CardIndex {
    card_index("a6409aa6-035c-4859-8135-90d34d67f72c")
}

fn kobold_overlord() -> CardIndex {
    card_index("5839bc83-868e-402a-9233-ad3d2871ac6d")
}

fn kobold_taskmaster() -> CardIndex {
    card_index("372f2534-25dc-4ff3-9891-31ee42b33345")
}

fn leonin_skyhunter() -> CardIndex {
    card_index("517c7295-8ba2-47a6-a1c6-aee722f8ca88")
}

fn llanowar_dead() -> CardIndex {
    card_index("3a48541a-240b-4406-945c-1d0d032eca25")
}

fn longbow_archer() -> CardIndex {
    card_index("51531db1-ba59-4ccc-9a91-54de2887fc8b")
}

fn lotus_cobra() -> CardIndex {
    card_index("8ad91f64-ccab-4edc-bd54-b2ee9267d614")
}

fn lurking_nightstalker() -> CardIndex {
    card_index("8c17f48b-467e-4953-85d0-221cef5b3bff")
}

fn master_decoy() -> CardIndex {
    card_index("94a16609-8529-4301-8a3d-47ced194f7cd")
}

// oracle_id = "10736ea3-6253-44c3-8a1e-3b9a7f38f3cf"
fn misshapen_fiend() -> CardIndex {
    card_index("10736ea3-6253-44c3-8a1e-3b9a7f38f3cf")
}

fn monk_realist() -> CardIndex {
    card_index("8df64d89-64ee-4388-826d-dd091db4510a")
}

fn moon_sprite() -> CardIndex {
    card_index("1e41136b-afa7-45ee-8d04-d0c7514b5387")
}

fn nantuko_shade() -> CardIndex {
    card_index("40d02b8c-fb0e-4607-855f-49fbcec2e209")
}

fn nomadic_elf() -> CardIndex {
    card_index("025bbfc8-bccd-4062-b758-7f9e1f8ad420")
}

fn oboro_breezecaller() -> CardIndex {
    card_index("257ee9a0-5ee0-49c4-8619-e75e1134d7ac")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

fn orochi_sustainer() -> CardIndex {
    card_index("d3039b42-25d5-4fe2-955a-0d910d7395bd")
}

fn patrol_hound() -> CardIndex {
    card_index("115dabbd-1a09-4641-a428-92fcbfd82ae9")
}

fn phyrexian_denouncer() -> CardIndex {
    card_index("b331c5fa-cd97-4fd8-b1df-d2c9e6a5a854")
}

fn plague_witch() -> CardIndex {
    card_index("532ab335-4873-405b-84b3-c447207172b5")
}

fn plant_elemental() -> CardIndex {
    card_index("e822bf3d-3a29-4a02-9ae8-e2830ce70f15")
}

fn priest_of_titania() -> CardIndex {
    card_index("3a198a16-17b9-481e-b516-5bc945c7e247")
}

fn pygmy_razorback() -> CardIndex {
    card_index("2bdd82e7-cbbf-4f12-bcb7-b276df021351")
}

fn scryb_sprites() -> CardIndex {
    card_index("a1f20695-6f08-4d5c-9fba-b0018bee298e")
}

fn quirion_elves() -> CardIndex {
    card_index("e2975a88-5a5a-462a-90b9-d8c00150b6d3")
}

fn quiron_sentinel() -> CardIndex {
    card_index("70bb2690-653b-4787-bdd1-d1739a8ae545")
}

fn rats_of_rath() -> CardIndex {
    card_index("75458175-eeb1-4a6b-93d9-c744c93b3759")
}

fn razorfin_hunter() -> CardIndex {
    card_index("45baa5d1-79aa-4583-b938-0d5feaa5aa0c")
}

fn rofellos_llanowar_emissary() -> CardIndex {
    card_index("3015882b-4897-4d2c-8e33-6731c85a0d03")
}

fn royal_falcon() -> CardIndex {
    card_index("a247a7e7-83bb-46f8-9a31-85b0d8ede919")
}

fn sage_of_lat_nam() -> CardIndex {
    card_index("f34be3cc-ff47-4415-a6a8-ed142891dc0c")
}

fn sakura_tribe_elder() -> CardIndex {
    card_index("e3afc704-220f-498f-9eaa-0821b17dc24c")
}

fn sea_eagle() -> CardIndex {
    card_index("acb57162-7093-4a3c-9818-d3b61ce757c6")
}

fn sea_scryer() -> CardIndex {
    card_index("f17954dd-c8e0-4983-b81e-5440cca41afc")
}

fn seeker_of_skybreak() -> CardIndex {
    card_index("704723f3-0490-4d96-930a-aacc6d19ba8d")
}

fn selesnya_evangel() -> CardIndex {
    card_index("4a2fea39-99d6-4766-9f2c-0a63925349cb")
}

fn shimmering_barrier() -> CardIndex {
    card_index("94674840-c73c-4659-80b9-11140d1deaa3")
}

fn silverglade_pathfinder() -> CardIndex {
    card_index("ac00d3db-d9e4-4fc3-a4ea-dd1187f406bb")
}

fn skittering_skirge() -> CardIndex {
    card_index("fae1d5d9-a50e-423e-8692-3e41f39df081")
}

fn skyshroud_elf() -> CardIndex {
    card_index("84a666a0-e453-4b34-815c-48dbfc9b4f4b")
}

fn skyshroud_falcon() -> CardIndex {
    card_index("877ab494-da90-4c26-a184-c6ba64ec30b5")
}

/// Slobad, Goblin Tinkerer — {1}{R} legendary 1/2 Goblin Artificer:
/// "Sacrifice an artifact: Target artifact gains indestructible until end of
/// turn."
///
/// The one printed line carries two different artifact readings, and the two
/// menus the engine publishes separate them: the *target* is any artifact on
/// either side of the table, while the *sacrifice* is one this seat controls
/// (CR 701.21a). The grant is then put to work instead of read off the layers —
/// a Vindicate aimed at the artifact is a destruction effect, and indestructible
/// is the one thing it cannot do (CR 702.12b) — so the permanent still standing
/// with a spent Vindicate in the graveyard is what says the keyword was really
/// there, and not merely that a projection said so.
fn slobad_goblin_tinkerer() -> CardIndex {
    card_index("1f2d6cf4-b8f4-440f-ac08-ca47ada868ac")
}

fn spiteful_bully() -> CardIndex {
    card_index("410acdbc-3a00-49db-a712-ad970d24b363")
}

fn starlight_invoker() -> CardIndex {
    card_index("4d27a90b-dae4-48a0-9eb2-e199ff548867")
}

fn steadfast_guard() -> CardIndex {
    card_index("0c3b3c04-017d-4741-90c8-4b6c0bd46e49")
}

fn stern_proctor() -> CardIndex {
    card_index("5120bfdc-7201-420d-85a1-eeb4aba6632b")
}

fn storm_crow() -> CardIndex {
    card_index("000d5588-5a4c-434e-988d-396632ade42c")
}

fn sword_dancer() -> CardIndex {
    card_index("9ae49b45-a38f-4bd6-aa08-b5a5da6e75a1")
}

fn talas_scout() -> CardIndex {
    card_index("71b7d5c4-c16c-4c6b-8e2e-e9b5cc128e1f")
}

fn talon_sliver() -> CardIndex {
    card_index("74910702-e385-48f7-a6b3-064ebed08be7")
}

fn temple_acolyte() -> CardIndex {
    card_index("17e4085e-af44-4ba8-af05-ccfe4b35f96a")
}

fn tonic_peddler() -> CardIndex {
    card_index("9039f59a-915b-4b16-a5c4-1a5fbfb0b639")
}

fn uktabi_faerie() -> CardIndex {
    card_index("447f1cce-bbc5-473b-a838-34070f8fd2dc")
}

fn urborg_elf() -> CardIndex {
    card_index("9c382817-5c62-44ff-8f29-d283e65712a1")
}

fn abbey_matron() -> CardIndex {
    card_index("62e3f285-886c-414e-b4ff-403a7c01c23a")
}

fn akki_drillmaster() -> CardIndex {
    card_index("aa228a01-7de3-4539-9ebb-b3c546f72b41")
}

fn alert_shu_infantry() -> CardIndex {
    card_index("0f019387-4684-493f-80c4-de6023096306")
}

fn alpha_kavu() -> CardIndex {
    card_index("f1a76cca-829a-4576-a560-f1af34b80b87")
}

fn ambassador_laquatus() -> CardIndex {
    card_index("2df8f253-1d3c-4b1f-8549-93fb90623474")
}

fn apprentice_wizard() -> CardIndex {
    card_index("d16d2451-1c21-4a6b-8a4a-40ac8cd9fe08")
}

fn armor_thrull() -> CardIndex {
    card_index("35453c9e-e1ae-4fe9-926d-75724deb0555")
}

// oracle_id = "8ebc4198-7317-4ffb-b8b8-14733c2077ff"
fn arms_dealer() -> CardIndex {
    card_index("8ebc4198-7317-4ffb-b8b8-14733c2077ff")
}

fn utopia_tree() -> CardIndex {
    card_index("73a9122c-60d2-4fdc-bc7b-3576bcbca241")
}

fn vedalken_mastermind() -> CardIndex {
    card_index("1ba85537-6dc2-459e-8850-619b41c0a881")
}

fn veteran_cavalier() -> CardIndex {
    card_index("2d02cfce-a142-4b3b-a01f-4e156e75164e")
}

fn vine_trellis() -> CardIndex {
    card_index("57de8fe7-3d1b-41cd-8354-38aff3d2d052")
}

fn viridian_zealot() -> CardIndex {
    card_index("9410ae41-b6db-47c6-ae0a-4e33aad1d3ce")
}

fn wall_of_blossoms() -> CardIndex {
    card_index("ef4d5fb3-70a3-433d-a9d3-18b2beb8d79f")
}

fn wall_of_earth() -> CardIndex {
    card_index("1e3e1916-bb82-481e-9b10-225504ef06b3")
}

fn wall_of_kelp() -> CardIndex {
    card_index("64722c32-975b-43b9-9b4a-9511c75740d9")
}

fn wall_of_mulch() -> CardIndex {
    card_index("afd2141f-1b0f-46b5-b1ac-aa28982d16c0")
}

fn wall_of_razors() -> CardIndex {
    card_index("19ed5429-5631-4a36-a69c-ab35860160b6")
}

fn waterfront_bouncer() -> CardIndex {
    card_index("b53a8cde-494c-43d0-8d08-7bdfe2e84064")
}

fn wei_ambush_force() -> CardIndex {
    card_index("2d1446f4-cf71-4798-b921-8f6824350319")
}

fn willow_faerie() -> CardIndex {
    card_index("5390bb44-985e-473b-883e-c9562c06f210")
}

fn wind_dancer() -> CardIndex {
    card_index("e9903d75-f71d-45d0-ac5f-955c0c227e02")
}

// oracle_id = "24e96682-14ca-4a62-a920-2d686c8a5dbf"
fn winged_sliver() -> CardIndex {
    card_index("24e96682-14ca-4a62-a920-2d686c8a5dbf")
}

fn wirewood_elf() -> CardIndex {
    card_index("4aab6794-506f-413c-91e0-d886e0019dce")
}

fn wirewood_hivemaster() -> CardIndex {
    card_index("20339b68-b617-45a6-b569-cc6231641e88")
}

fn wyluli_wolf() -> CardIndex {
    card_index("00185f3b-2777-4417-a8e0-4691f41c0ec1")
}

fn youthful_knight() -> CardIndex {
    card_index("ef2a24f5-ce5e-4054-843a-2cae0c66318a")
}

fn zephyr_falcon() -> CardIndex {
    card_index("b5e44bd6-0b67-4770-868d-c46af5b6a42f")
}

fn zulaport_cutthroat() -> CardIndex {
    card_index("76b003e0-15af-4f22-bdf2-1ade5430964a")
}

fn army_ants() -> CardIndex {
    card_index("c39112ab-ee1f-4f00-b18b-692b5fe32b80")
}

fn azimaet_drake() -> CardIndex {
    card_index("a82d9c9b-ec17-4f51-a6bf-5984ca4433e5")
}

fn balloon_peddler() -> CardIndex {
    card_index("10f182a7-ffa3-4128-8b2e-e05b97f92e00")
}

fn balthor_the_stout() -> CardIndex {
    card_index("23669721-fe9e-49d7-9504-ae6164de723a")
}

fn barbarian_lunatic() -> CardIndex {
    card_index("12b5fbe5-a426-4b7e-bef0-abea71170e49")
}

fn barrin_master_wizard() -> CardIndex {
    card_index("8c6d6684-1943-4af3-98da-951604193911")
}

// oracle_id = "d52b72ff-a82d-430e-94c7-675c83b43e50"
fn braidwood_cup() -> CardIndex {
    card_index("d52b72ff-a82d-430e-94c7-675c83b43e50")
}

fn battle_rampart() -> CardIndex {
    card_index("ec187304-4c05-44da-8ad5-7c56f6b05212")
}

fn bird_maiden() -> CardIndex {
    card_index("bddd6b93-7c69-427a-97ee-0c1f9f950f09")
}

fn blade_sliver() -> CardIndex {
    card_index("110e5695-a9df-4201-9230-99f6abe9978f")
}

fn blaster_mage() -> CardIndex {
    card_index("e5b35b54-77d9-4cb0-86a1-efc8e49aea09")
}

fn blistering_barrier() -> CardIndex {
    card_index("8783533d-dd98-4b80-b348-29051e1b5c28")
}

fn blood_vassal() -> CardIndex {
    card_index("f792f40d-1d37-4854-98da-e4020c6a44d4")
}

fn bog_witch() -> CardIndex {
    card_index("85573cba-07ae-4421-a167-a8569f85c0f7")
}

fn bogardan_firefiend() -> CardIndex {
    card_index("1d61f3a7-5861-479b-8312-f2d308c35b68")
}

fn breathstealer() -> CardIndex {
    card_index("17d18e2f-2d8d-449e-9356-a7351bf0f8c2")
}

fn cabal_archon() -> CardIndex {
    card_index("7d80f016-5a4a-4843-9aec-0fe64f7f7ae8")
}

fn capashen_templar() -> CardIndex {
    card_index("ab9ad5e2-2bf3-450d-9d5c-c7280d368e10")
}

fn charging_paladin() -> CardIndex {
    card_index("6cccf44e-c05e-4239-b643-54b559c98552")
}

fn coastal_drake() -> CardIndex {
    card_index("d4220231-e9ac-4390-8c7b-979549b26d33")
}

fn council_of_advisors() -> CardIndex {
    card_index("ff427657-173a-4845-89af-6f0a93467130")
}

fn devout_witness() -> CardIndex {
    card_index("83647f5b-2b0f-4f9d-83c5-75c52460f35b")
}

fn diving_griffin() -> CardIndex {
    card_index("8785f9b4-eea2-4b45-9ae4-194b9a715702")
}

fn dogged_hunter() -> CardIndex {
    card_index("cf5b3249-619f-48f5-8885-00fd41d24825")
}

fn drake_hatchling() -> CardIndex {
    card_index("bd55bcc4-f0c5-4c09-8b3b-2e25e9fbd9ee")
}

fn dusk_imp() -> CardIndex {
    card_index("1389ae4a-c3a5-4678-9012-937a3cbaf7f7")
}

fn dwarven_bloodboiler() -> CardIndex {
    card_index("cbbca098-0b33-4693-8ea7-a2005b79af3d")
}

// oracle_id = "caf3c6ec-d17e-497c-8fe7-6f818ce93f96"
fn dwarven_demolition_team() -> CardIndex {
    card_index("caf3c6ec-d17e-497c-8fe7-6f818ce93f96")
}

fn ertai_wizard_adept() -> CardIndex {
    card_index("f2902dfd-888f-4b02-b8ad-1056f0059fc3")
}

fn fallow_wurm() -> CardIndex {
    card_index("7b414f70-a9b1-42c3-9655-6aa520d3b19b")
}

fn fault_riders() -> CardIndex {
    card_index("df516046-7f83-4d64-aa0d-d687631d919a")
}

fn feral_shadow() -> CardIndex {
    card_index("cf26dcb4-e181-4ba5-bc03-b56f57032b85")
}

fn fire_drake() -> CardIndex {
    card_index("fd3bcc9b-7d84-478e-aef5-2e44610107c7")
}

fn fire_imp() -> CardIndex {
    card_index("5bd806e7-3f9b-4bb4-9708-f87a578f531e")
}

fn fledgling_imp() -> CardIndex {
    card_index("e7eaffd5-bdaf-4f72-9c88-8196b3411894")
}

fn fleshgrafter() -> CardIndex {
    card_index("04b1fb9e-4aef-46d9-b163-f9beff198bad")
}

fn flowstone_shambler() -> CardIndex {
    card_index("e72ae9ab-7405-4ea7-b075-42d0ddc352db")
}

fn flowstone_wall() -> CardIndex {
    card_index("5fa86b45-1d5b-42df-8811-6a10903f3039")
}

fn foratog() -> CardIndex {
    card_index("77f257b9-2047-40a3-85c2-00ad17deeb08")
}

// oracle_id = "f75f9006-217d-4ec8-9c35-ceefe1c5ae4e"
fn frozen_shade() -> CardIndex {
    card_index("f75f9006-217d-4ec8-9c35-ceefe1c5ae4e")
}

fn furnace_spirit() -> CardIndex {
    card_index("86b33d25-6123-439c-9026-a4e835b48562")
}

fn fyndhorn_brownie() -> CardIndex {
    card_index("6d1f8073-4da2-4abc-abb4-2053c4a40bbf")
}

fn fyndhorn_elder() -> CardIndex {
    card_index("507bdb1a-90b4-4fd3-a1a3-e8be316e97f3")
}

fn ghosts_of_the_damned() -> CardIndex {
    card_index("1b3bbd3c-f5ef-4940-82a2-d36f4d148b9a")
}

fn glacial_wall() -> CardIndex {
    card_index("7fcf8faa-2c93-4dc8-aa56-6fc40340e212")
}

fn goblin_chariot() -> CardIndex {
    card_index("6ef0dd51-31df-4fd3-8153-49a427a7fdcf")
}

fn goblin_medics() -> CardIndex {
    card_index("457bee4c-2332-440c-9336-36f6a69614f9")
}

fn goblin_sky_raider() -> CardIndex {
    card_index("9e0ebf3b-9295-43a9-9b5c-4ffac6a6b630")
}

fn goliath_beetle() -> CardIndex {
    card_index("86ab4400-fbdd-4c18-a893-441286a9d7d0")
}

fn granite_gargoyle() -> CardIndex {
    card_index("ef5248d4-4cd1-447c-bdcc-78a09a67d923")
}

fn helionaut() -> CardIndex {
    card_index("8d4de785-20e9-430e-8df1-f39dac0ef07d")
}

fn hidden_horror() -> CardIndex {
    card_index("db7c5a2f-9aea-4b14-86cf-7af79d6484af")
}

// oracle_id = "64553181-9852-459f-a6e0-54ce188dd937"
fn horned_sliver() -> CardIndex {
    card_index("64553181-9852-459f-a6e0-54ce188dd937")
}

// oracle_id = "4958baf3-2f5a-4087-bc14-ffee9887db1d"
fn hornet_cobra() -> CardIndex {
    card_index("4958baf3-2f5a-4087-bc14-ffee9887db1d")
}

fn horseshoe_crab() -> CardIndex {
    card_index("793f29ac-8a5d-49bf-91e8-a32f771268f3")
}

fn juniper_order_druid() -> CardIndex {
    card_index("943ab696-470b-488f-9567-c8cd89ec06e3")
}

fn kabuto_moth() -> CardIndex {
    card_index("537a255c-a77b-4691-9461-68efb1aba7d4")
}

fn kami_of_the_hunt() -> CardIndex {
    card_index("96e68c7a-f187-4aa6-998c-238e7c833809")
}

fn kavu_glider() -> CardIndex {
    card_index("425050a7-41d2-4850-bb45-3a83275d3c50")
}

fn keen_eyed_archers() -> CardIndex {
    card_index("0ace32d6-7261-447c-9ee2-e03febaab91b")
}

// Killer Bees — {1}{G}{G} — Creature — Insect, printed 0/1.

fn killer_bees() -> CardIndex {
    card_index("5da1f1af-d2e5-4e14-914b-d93f15626636")
}

fn krark_clan_grunt() -> CardIndex {
    card_index("e7be5ab3-a11f-4786-9b0b-1daa94f59440")
}

// oracle_id = "756b4bc9-6f3f-4eae-b6b0-63c94ed8482f"
fn krark_clan_stoker() -> CardIndex {
    card_index("756b4bc9-6f3f-4eae-b6b0-63c94ed8482f")
}

// oracle_id = "228f0afa-146b-4564-a380-71595cdf1ef4"
fn land_leeches() -> CardIndex {
    card_index("228f0afa-146b-4564-a380-71595cdf1ef4")
}

// oracle_id = "23d42beb-5293-4f12-a732-3531a0fa0ca0"
fn leaping_lizard() -> CardIndex {
    card_index("23d42beb-5293-4f12-a732-3531a0fa0ca0")
}

fn ley_druid() -> CardIndex {
    card_index("a4ae41c7-9631-407d-8fd3-04f403d940a8")
}

fn lieutenant_kirtar() -> CardIndex {
    card_index("ad39e859-e4a6-49cc-b738-4578a5250538")
}

fn llanowar_cavalry() -> CardIndex {
    card_index("3e6e70eb-6638-4aa6-997b-90f5810e12a0")
}

fn llanowar_vanguard() -> CardIndex {
    card_index("9123086e-ae67-46c5-aed5-6366e2fe5a8c")
}

fn lumengrid_sentinel() -> CardIndex {
    card_index("ede92fb6-dfd6-40b4-b58f-af1f7a155efc")
}

fn man_o_war() -> CardIndex {
    card_index("67a3541c-8408-40c8-b44f-90035b860f57")
}

fn marker_beetles() -> CardIndex {
    card_index("4b1614fd-2810-4d4d-a0d6-934f27adbe65")
}

fn mercenary_knight() -> CardIndex {
    card_index("ed5429bb-233a-4528-bf7d-df5f6b192b1c")
}

fn merchant_of_secrets() -> CardIndex {
    card_index("f6aebd42-0150-4741-84c2-4c85893640e9")
}

fn moaning_spirit() -> CardIndex {
    card_index("78962897-e2e3-4cb2-ae97-eca5dbe74e49")
}

fn moonwing_moth() -> CardIndex {
    card_index("6501b3c7-fd21-4668-b647-db12f0cc23f7")
}

fn morgue_thrull() -> CardIndex {
    card_index("fc759e09-34b2-4004-8474-a6500dd1280c")
}

fn morgue_toad() -> CardIndex {
    card_index("c615540b-bf85-4e16-8ac2-9d541cba732f")
}

fn moriok_rigger() -> CardIndex {
    card_index("9364b5bc-3e78-4b9d-9993-0081bbe2eef6")
}

fn mournful_zombie() -> CardIndex {
    card_index("f0ed2cf1-7bd4-4586-a682-fd588632cb3d")
}

fn nantuko_elder() -> CardIndex {
    card_index("001c233f-2959-479b-a82a-64a25ac60830")
}

fn nantuko_husk() -> CardIndex {
    card_index("0dcefc00-9425-43e3-bd47-1e34fff8b0e2")
}

fn nightguard_patrol() -> CardIndex {
    card_index("bb482f90-5624-4e2c-9916-85fb09a3639a")
}

fn nim_abomination() -> CardIndex {
    card_index("b0713b1e-544d-4549-9371-61ac7f6c2a6a")
}

fn noble_panther() -> CardIndex {
    card_index("bfb799d2-b2fa-4efb-91d7-a457230b370f")
}

fn orcish_mechanics() -> CardIndex {
    card_index("0ec58835-de2d-4064-89a9-f92db80bc276")
}

fn overeager_apprentice() -> CardIndex {
    card_index("3ea71eed-e1ae-4e28-a4f5-44ff115949f4")
}

fn pegasus_charger() -> CardIndex {
    card_index("81a5ac8d-b904-4755-a5bc-650e95f4138f")
}

fn phyrexian_broodlings() -> CardIndex {
    card_index("7d5e4033-bd58-4854-b9d3-a440240995ee")
}

fn phyrexian_rager() -> CardIndex {
    card_index("e409c9be-0c9a-43c3-adb4-8c47afc1d551")
}

fn pincher_beetles() -> CardIndex {
    card_index("0fae1c73-fd34-433e-8989-fbd80e3b1c70")
}

fn pradesh_gypsies() -> CardIndex {
    card_index("37c49483-bef6-47c6-9354-ead8560d48da")
}

fn priest_of_gix() -> CardIndex {
    card_index("d93f82ce-0eed-45cc-a7b1-50fd4cbb6152")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

fn quagmire_druid() -> CardIndex {
    card_index("560e1e81-6675-4d08-8e82-cdf1abd4b3d0")
}

fn raging_cougar() -> CardIndex {
    card_index("695ebb35-55fd-4ed1-8cbe-e3fc1223115b")
}

fn raging_kavu() -> CardIndex {
    card_index("d126bb08-be60-4046-91f1-65672ef42c63")
}

// oracle_id = "b8ae71b3-8ee7-49a0-87ea-7e938166ccb3"
fn ravenous_skirge() -> CardIndex {
    card_index("b8ae71b3-8ee7-49a0-87ea-7e938166ccb3")
}

fn reliquary_monk() -> CardIndex {
    card_index("a149ac31-b792-4fbe-903a-788bbdfd5e97")
}

fn rib_cage_spider() -> CardIndex {
    card_index("906cba93-3dac-4720-a482-987cf1b4e786")
}

fn rootwalla() -> CardIndex {
    card_index("4dda5f21-11d6-40ce-be24-7c9529272c25")
}

fn rootwater_hunter() -> CardIndex {
    card_index("8e42ed82-8b56-46ad-a1ca-e29689a9d030")
}

// oracle_id = "de7f1725-833c-4784-94b1-7a33e36aec94"
fn rotlung_reanimator() -> CardIndex {
    card_index("de7f1725-833c-4784-94b1-7a33e36aec94")
}

fn royal_assassin() -> CardIndex {
    card_index("9ed6f28f-a3db-48c5-9ab0-b90a7fba5f57")
}

fn sabretooth_tiger() -> CardIndex {
    card_index("8ea35158-1c1a-46da-b466-0eafb376e464")
}

// oracle_id = "dd520d83-297a-487f-b8f3-4997bbc056e0"
fn serendib_efreet() -> CardIndex {
    card_index("dd520d83-297a-487f-b8f3-4997bbc056e0")
}

fn serpent_warrior() -> CardIndex {
    card_index("fe249f27-db6c-4826-836e-a04efb1a3eaa")
}

fn seton_krosan_protector() -> CardIndex {
    card_index("48d9747f-f9eb-4f71-93c9-e5151a7ff3ea")
}

fn shu_grain_caravan() -> CardIndex {
    card_index("587e9f15-1d79-458f-a073-229b828b2393")
}

fn silent_attendant() -> CardIndex {
    card_index("4fa30e29-94c1-4537-bebd-dc7124e4ef71")
}

fn sisters_of_the_flame() -> CardIndex {
    card_index("389a9d46-d3fb-47f0-91cd-f6d487636916")
}

fn skittering_horror() -> CardIndex {
    card_index("4344abeb-47ab-4638-b50f-dd502f14c1af")
}

fn sky_spirit() -> CardIndex {
    card_index("fc7e288f-ade2-4997-b3fe-f873c26990b0")
}

fn skyhunter_prowler() -> CardIndex {
    card_index("5e0d13b0-d5d0-4c5f-945e-5cda7d980670")
}

/// Skyhunter Skirmisher — {1}{W}{W}, a 1/1 Cat Knight printing flying and
/// double strike. Neither keyword is a number a card file can be checked
/// against: flying and double strike only mean anything once the layer
/// system has projected them onto a permanent that is actually in a game,
/// and double strike is only itself in a combat where a printed 1/1 takes
/// **two** life rather than one. So the Skirmisher is cast for its printed
/// cost, read off its projected characteristics, and then walked around a
/// whole turn — it is summoning sick the turn it arrives (CR 302.6) — to
/// attack past a ground 1/1 that could block any other 1/1 and may not block
/// this one.
fn skyhunter_skirmisher() -> CardIndex {
    card_index("0c07d09e-e127-4573-b827-6c50246f7a31")
}

fn skyknight_legionnaire() -> CardIndex {
    card_index("61178a6c-70b7-447d-87ee-a8d9369c3d15")
}

fn smokespew_invoker() -> CardIndex {
    card_index("a6df9679-1e4c-4791-855b-28bc2fa35b49")
}

fn soratami_rainshaper() -> CardIndex {
    card_index("63c5f42b-13d0-4391-b033-4ab2f4e3152b")
}

fn soulsworn_jury() -> CardIndex {
    card_index("06424141-2c26-4ee8-83c2-dac62e3d3540")
}

fn stalking_assassin() -> CardIndex {
    card_index("12117089-67f7-4d0e-88db-82b27a4c7cdc")
}

fn standing_troops() -> CardIndex {
    card_index("9e9d5242-424d-4f61-b625-806295dbb0c7")
}

fn storm_shaman() -> CardIndex {
    card_index("65e07c0f-4eff-4b38-9785-63684d67c0d8")
}

fn stronghold_machinist() -> CardIndex {
    card_index("50bbbb39-4709-478f-bb51-716c7e4f8323")
}

fn taoist_hermit() -> CardIndex {
    card_index("960b1251-12af-4766-8df1-bb98e7e967f5")
}

fn tempest_drake() -> CardIndex {
    card_index("ff0086e0-706f-4474-9b5e-a1591235bf9b")
}

fn temporal_adept() -> CardIndex {
    card_index("fd9b5462-6578-4bf6-8026-23eaf5af3eee")
}

fn thornwind_faeries() -> CardIndex {
    card_index("941a0e8d-c407-476b-9d0e-cffc1f8c6003")
}

fn thundering_wurm() -> CardIndex {
    card_index("cdab50a8-1e02-4ba5-8c09-e2837e7652f7")
}

fn timberwatch_elf() -> CardIndex {
    card_index("50cee3ac-cba0-4abb-babf-de1928b1590e")
}

fn uktabi_orangutan() -> CardIndex {
    card_index("6cc07a7e-e70c-45c5-83c5-a2704e57dbc5")
}

fn venerable_monk() -> CardIndex {
    card_index("5c5cfa6d-857f-44a9-80f7-46f016ef71e4")
}

fn verduran_enchantress() -> CardIndex {
    card_index("cd98a31b-cc7e-43f9-982e-109ad9850908")
}

fn vesper_ghoul() -> CardIndex {
    card_index("c45562ce-5d1a-4ad4-82f8-92199520ae87")
}

// oracle_id = "236fbf12-f908-4918-babb-1558e767cbe3"
fn vicious_kavu() -> CardIndex {
    card_index("236fbf12-f908-4918-babb-1558e767cbe3")
}

fn viridian_shaman() -> CardIndex {
    card_index("5aa6d553-a144-4c5b-83fa-65e931903899")
}

fn vulshok_sorcerer() -> CardIndex {
    card_index("6f858405-dfe4-4ca2-b2e3-80ce8cda5522")
}

fn wall_of_air() -> CardIndex {
    card_index("b2d3da40-e2f7-4480-9da9-33019d6f4071")
}

fn wall_of_blood() -> CardIndex {
    card_index("b26c3563-d289-4ca3-8736-9d89e191a840")
}

fn wall_of_granite() -> CardIndex {
    card_index("8445094f-008b-491a-977c-e8582d5ab72c")
}

fn wall_of_heat() -> CardIndex {
    card_index("c7bbfd32-2a6f-44e9-b51e-9b272f48d458")
}

fn wall_of_ice() -> CardIndex {
    card_index("1945e165-7636-4727-89ba-3811251ef175")
}

fn wall_of_stone() -> CardIndex {
    card_index("cd4cadb4-3156-49bd-b36e-12ba5c85938b")
}

fn whip_sergeant() -> CardIndex {
    card_index("1e25c9bd-2a6a-4856-80e6-775847a2c89f")
}

fn whiptongue_frog() -> CardIndex {
    card_index("edc961bd-5ec9-47ed-9d34-e26a48c13702")
}

fn wild_aesthir() -> CardIndex {
    card_index("dbdf076c-18ce-45e3-8f70-0a32391ea4b6")
}

fn wild_colos() -> CardIndex {
    card_index("cb6b8ce3-9f9d-418c-94b0-c4469a254938")
}

fn wild_griffin() -> CardIndex {
    card_index("c643cfe1-5844-4eb1-b1f5-028382411773")
}

fn wind_drake() -> CardIndex {
    card_index("d6ffdaf0-ac08-4de9-bbce-2eab2f86bcca")
}

fn advanced_hoverguard() -> CardIndex {
    card_index("65dc61a4-9052-49d4-bce8-9c16f0339382")
}

fn akroma_s_devoted() -> CardIndex {
    card_index("d264903b-d23e-48c8-95f1-cb1606a705a2")
}

// Alaborn Cavalier — the card the knight deck's whole attack plan turns on.

fn alaborn_cavalier() -> CardIndex {
    card_index("f8273146-f2e5-40ec-ba96-c0a4192caec8")
}

fn anaba_bodyguard() -> CardIndex {
    card_index("8c64db66-879e-46ad-9d11-8a95b6cf5d19")
}

fn anaba_shaman() -> CardIndex {
    card_index("bc416c17-e1f0-4344-8c21-6184cac10205")
}

fn anaba_spirit_crafter() -> CardIndex {
    card_index("e9bee79c-144f-41e4-a53d-17df8381f691")
}

fn ancient_spider() -> CardIndex {
    card_index("28537019-3061-4efc-8e87-9bb805bf520f")
}

fn angelfire_crusader() -> CardIndex {
    card_index("1a587b8d-fcb4-47ca-9689-610897117166")
}

fn archivist() -> CardIndex {
    card_index("d137586f-83b0-40af-8100-443460b07ac0")
}

// oracle_id = "80e08ee2-4272-4783-9a0d-90079600b594"
fn argothian_swine() -> CardIndex {
    card_index("80e08ee2-4272-4783-9a0d-90079600b594")
}

fn armored_griffin() -> CardIndex {
    card_index("ad3d6003-66d3-486d-aa54-ddce1adb5ff1")
}

fn aven_cloudchaser() -> CardIndex {
    card_index("48bda7dd-d023-41e8-8c28-e0cfda0d07ca")
}

fn aven_fogbringer() -> CardIndex {
    card_index("3fb3d4b3-1034-449c-864b-633e7f39d73f")
}

fn aven_trooper() -> CardIndex {
    card_index("258073aa-6500-46a2-a5e5-22378b14260b")
}

fn azure_drake() -> CardIndex {
    card_index("15cc068c-f424-433e-b165-8457c62a4b35")
}

fn balshan_collaborator() -> CardIndex {
    card_index("7cdbc115-cb15-4ada-8229-d40f0b22dd27")
}

fn benalish_heralds() -> CardIndex {
    card_index("09ee1332-741f-4c55-abc3-8bcff9031cd9")
}

fn blessed_orator() -> CardIndex {
    card_index("c0856fea-cbdc-427b-ae00-bfd9311f284f")
}

fn cackling_imp() -> CardIndex {
    card_index("adc8c55c-8fbe-4e07-b1be-8cb796db3d8a")
}

fn carnivorous_plant() -> CardIndex {
    card_index("3a7478d7-b33e-4227-8cc3-b30643624d56")
}

fn carrion_ants() -> CardIndex {
    card_index("87bc6be7-4734-4a11-806e-e9e4f5616fe0")
}

fn carrion_howler() -> CardIndex {
    card_index("e139176f-483f-4f05-af3d-0176d0048b62")
}

fn clickslither() -> CardIndex {
    card_index("436f9a5d-ec26-4c88-9851-b88fe742fa67")
}

fn cloudchaser_eagle() -> CardIndex {
    card_index("add18787-2329-4355-a524-66fefda8d06e")
}

fn corrupt_eunuchs() -> CardIndex {
    card_index("77e604fe-d6ca-495f-ba22-a7d8ba88c8b5")
}

fn windseeker_centaur() -> CardIndex {
    card_index("b3d40cf7-a529-4bd0-8df7-231630a49bb8")
}

fn wirewood_savage() -> CardIndex {
    card_index("45579312-499f-41de-b16e-a231d65a2053")
}

/// Shaleskin Bruiser — the Beast the Savage's trigger is about.
fn shaleskin_bruiser() -> CardIndex {
    card_index("b90e370a-5080-485e-a957-93d5f60e6cdb")
}

fn wood_elves() -> CardIndex {
    card_index("8973bd99-20f8-4867-90ef-50392147ee1b")
}

fn xira_arien() -> CardIndex {
    card_index("4a6e367c-7bc9-44a3-8ede-f2d0651abad3")
}

fn zuran_spellcaster() -> CardIndex {
    card_index("bc7b90b1-3517-4e5d-9bd8-68b4d8a259fd")
}

fn visara_the_dreadful() -> CardIndex {
    card_index("79b999bc-2d4b-41e1-b64e-f0c080a9a2c5")
}

// oracle_id = "b2e950fb-cb7e-40a0-a311-5bbdd0477b29"
fn sun_titan() -> CardIndex {
    card_index("b2e950fb-cb7e-40a0-a311-5bbdd0477b29")
}

// oracle_id = "1be13ede-98f8-497e-800c-03e5802932b3"
fn reveillark() -> CardIndex {
    card_index("1be13ede-98f8-497e-800c-03e5802932b3")
}

// oracle_id = "017aa9b3-a8ea-4588-9c50-e914a7d8e4ee"
fn metamorphosis_fanatic() -> CardIndex {
    card_index("017aa9b3-a8ea-4588-9c50-e914a7d8e4ee")
}

/// Recruiter of the Guard's search menu over a library holding a Llanowar
/// Elves, an Ashaya and a Pyrogoyf, with three Plains on p0's side. With
/// `two_types`, the graveyards hold a land card and a creature card first.
/// Returns what the menu offered, as cards.
fn recruiter_menu_over_defined_bodies(two_types: bool) -> Vec<CardIndex> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(
            0,
            &[
                recruiter_of_the_guard(),
                llanowar_elves(),
                ashaya_soul_of_the_wild(),
                pyrogoyf(),
                juzam_djinn(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // In hand already: "equal to the number of lands you control" counts
    // the three Plains wherever the card is (CR 604.3).
    let ashaya = in_hand(&engine, p0, ashaya_soul_of_the_wild()).expect("Ashaya starts in hand");
    assert_eq!(
        pt(&engine, ashaya),
        (3, 3),
        "Ashaya in hand is as big as p0's lands"
    );

    let moves: Vec<(ObjectId, ZoneLocation)> =
        [llanowar_elves(), ashaya_soul_of_the_wild(), pyrogoyf()]
            .into_iter()
            .map(|card| {
                (
                    in_hand(&engine, p0, card).expect("dealt into the hand"),
                    ZoneLocation::Library(p0),
                )
            })
            .chain(two_types.then(|| {
                (
                    in_hand(&engine, p0, juzam_djinn()).expect("dealt into the hand"),
                    ZoneLocation::Graveyard(p0),
                )
            }))
            .collect();
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        for (card, to) in moves {
            state
                .move_object(
                    card,
                    to,
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .expect("the harness moves a card");
        }
    }
    if two_types {
        // A Forest off the top of p1's library: the land type beside the
        // Djinn's creature type.
        seed_graveyard(&mut engine, p1, 1);
    }
    engine.refresh_offer();

    cast_from_hand(&mut engine, p0, recruiter_of_the_guard());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched");
    };
    options
        .iter()
        .filter_map(|id| engine.state().object(*id).and_then(|o| o.card))
        .map(|card| card.index)
        .collect()
}

fn sokka_tenacious_tactician() -> CardIndex {
    card_index("6b68acc2-b9d5-495b-8054-c04bae1349f1")
}

fn mirrorhall_mimic() -> CardIndex {
    card_index("5768fe50-a134-492c-a725-5ed02610c39f")
}

/// The card printed as `card` in `zone`, if it is there.
fn card_in(
    engine: &Engine<RegistryLookup>,
    zone: ZoneLocation,
    card: CardIndex,
) -> Option<ObjectId> {
    engine.state().zones.list(zone).iter().copied().find(|id| {
        engine
            .state()
            .object(*id)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
    })
}

/// Puts Mirrorhall Mimic in `seat`'s graveyard and casts it with disturb as
/// Ghastly Mimicry, enchanting `creature`. The harness moves the card: how
/// it got to the graveyard is not what these tests read.
fn disturb_the_mimic(engine: &mut Engine<RegistryLookup>, seat: PlayerId, creature: ObjectId) {
    let card = in_hand(engine, seat, mirrorhall_mimic()).expect("the Mimic is in hand");
    engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up")
        .move_object(
            card,
            ZoneLocation::Graveyard(seat),
            ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("into the graveyard");
    tap_all_mana(engine, seat);
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("disturb is offered from the graveyard");
    aim_at(engine, seat, creature);
}

fn abrupt_decay() -> CardIndex {
    card_index("1c747fe2-289e-492a-a846-aa77707e2dc3")
}

fn dauthi_voidwalker() -> CardIndex {
    card_index("f1c2dbe2-fbe0-4058-bdf1-91d1b1832786")
}

/// The card an opponent's `card` became in their exile, with the void
/// counter Dauthi Voidwalker's replacement put on it.
fn voided(engine: &Engine<RegistryLookup>, owner: PlayerId, card: CardIndex) -> ObjectId {
    let id = engine
        .state()
        .zones
        .list(ZoneLocation::Exile(owner))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .expect("the card was exiled instead of put into the graveyard");
    assert_eq!(
        counters_on(engine, id, baylee_cards_dsl::counters::VOID),
        1,
        "with a void counter on it"
    );
    id
}

/// Sacrifices the Voidwalker and answers its choice with `card`.
fn void_walk(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: ObjectId) {
    activate(engine, seat, dauthi_voidwalker(), 1);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = pass_to_card_choice(engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, seat);
    assert_eq!(prompt, ChoicePrompt::PlayFromExile);
    assert_eq!((min, max), (1, 1), "\"choose an exiled card\"");
    assert_eq!(options, vec![card], "only the opponent's voided card");
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

fn risen_reef() -> CardIndex {
    card_index("2ae71e86-4400-4a30-9077-4d57a43e7395")
}

/// Answers Risen Reef's question about the top card, after checking it is
/// the one-card, may-decline question about that card, and returns the card.
#[track_caller]
fn reef_question(engine: &mut Engine<RegistryLookup>, seat: PlayerId, put: bool) -> ObjectId {
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .last()
        .expect("a card on top");
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = pass_to_card_choice(engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, seat);
    assert_eq!(options, vec![top], "the looked-at card is the question");
    assert_eq!((min, max), (0, 1), "\"you may\": naming nothing declines");
    assert_eq!(prompt, ChoicePrompt::PutOntoBattlefield);
    let objects = if put { vec![top] } else { vec![] };
    engine
        .apply(seat, PlayerAction::ChooseObjects { objects })
        .unwrap();
    pass_until(engine, stack_is_empty);
    top
}

// ---------------------------------------------------------------------------
// Maik's European Highlander (28.09.2026): Thragtusk, Massacre Wurm.
// ---------------------------------------------------------------------------

fn thragtusk() -> CardIndex {
    card_index("0dd0e91a-d16b-4718-8d11-1a3fcf8e0753")
}

fn massacre_wurm() -> CardIndex {
    card_index("93cf50cf-0ecc-4d3e-abea-778c1ebacec4")
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: the two transforming cards, Archangel Avacyn
// and Huntmaster of the Fells.
// ---------------------------------------------------------------------------

fn archangel_avacyn() -> CardIndex {
    card_index("432b37a5-d32a-4b78-91ab-860aa026b7cc")
}

fn huntmaster_of_the_fells() -> CardIndex {
    card_index("582328cd-660d-47a4-bb23-e91e80b9a907")
}

fn face_shown(engine: &Engine<RegistryLookup>, id: ObjectId) -> u8 {
    engine.state().object(id).expect("still there").face_index
}

/// Passes until the upkeep of `seat`'s next turn has begun and everything
/// it put on the stack has resolved.
///
/// Ravager of the Fells' transform trigger is answered on the way past, the
/// plain way: its first target is the first opponent offered, and its "up to
/// one target creature" is declined. A test about that trigger answers it
/// itself, after [`upkeep_until_ravager_asks`].
fn through_upkeep_of(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    // First into that upkeep, then out of it.
    for past in [false, true] {
        let stage = |e: &Engine<RegistryLookup>| {
            e.state().turn.active == seat
                && matches!(e.state().turn.step, crate::turn::Step::Upkeep) != past
        };
        loop {
            pass_until(engine, |e| {
                stage(e) || matches!(e.pending(), Pending::ChooseTargets { .. })
            });
            let Pending::ChooseTargets {
                player,
                player_options,
                ..
            } = engine.pending().clone()
            else {
                break;
            };
            let players = player_options.into_iter().take(1).collect();
            engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players,
                    },
                )
                .expect("the first opponent, or no creature");
        }
    }
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Kiki-Jiki, Mirror Breaker.
// ---------------------------------------------------------------------------

fn kiki_jiki_mirror_breaker() -> CardIndex {
    card_index("a34b7416-cfe3-4a1e-a8c1-a3056b747519")
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Coiling Oracle.
// ---------------------------------------------------------------------------

fn coiling_oracle() -> CardIndex {
    card_index("69fd4ddf-9ed8-4c56-bef3-9944daf05e4f")
}

/// Casts Coiling Oracle off a Forest and an Island and lets it resolve with
/// its enter trigger; answers the top card it revealed.
fn coil(filler: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, filler)
        .battlefield(0, &[forest(), island()])
        .hand(0, &[coiling_oracle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let top = *engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .last()
        .expect("a library under the Oracle");
    cast_from_hand(&mut engine, p0, coiling_oracle());
    pass_until(&mut engine, stack_is_empty);
    (engine, top)
}

fn revealed(engine: &Engine<RegistryLookup>, card: ObjectId) -> bool {
    engine.journal().entries().iter().any(|e| {
        matches!(&e.event, crate::event::GameEvent::Revealed { player, cards }
            if *player == PlayerId::new(0) && *cards == vec![card])
    })
}

/// "If it's a land card, put it onto the battlefield." The top card is a
/// Forest: shown to everyone, then on the battlefield, untapped.
#[test]
fn coiling_oracle_puts_a_revealed_land_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let (engine, top) = coil(forest());
    assert!(revealed(&engine, top), "revealed before it moved");
    let land = engine.state().object(top).expect("the revealed Forest");
    assert_eq!(land.zone, crate::zone::Zone::Battlefield);
    assert!(
        !land.status.contains(crate::object::Status::TAPPED),
        "put onto the battlefield, not tapped"
    );
    let lands = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.controller == p0 && o.card.is_some_and(|c| c.index == forest()))
        })
        .count();
    assert_eq!(lands, 2, "the Forest it paid with and the one it revealed");
    let hand = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    assert_eq!(hand, 0, "nothing went to the hand");
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: three cards that were already written, played.
// ---------------------------------------------------------------------------

fn acidic_slime() -> CardIndex {
    card_index("21f45043-5419-4019-8b6c-e5294bd5f549")
}

fn fulminator_mage() -> CardIndex {
    card_index("bd4c46a3-b723-4b35-9061-9a1dee7cc9d8")
}

fn karmic_guide_card() -> CardIndex {
    card_index("8c31fec9-e4b3-4761-990e-7be38eb05604")
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Pyrogoyf.
// ---------------------------------------------------------------------------

fn pyrogoyf() -> CardIndex {
    card_index("7fd7457a-388d-4cca-a7cf-86b4ea922037")
}

/// Casts Pyrogoyf with a creature card and a land card in the graveyards,
/// so it enters a 2/3, and aims its own enter trigger at the opponent.
fn a_pyrogoyf_aimed_at_the_opponent() -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, steadfast_guard())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), forest()],
        )
        .hand(0, &[pyrogoyf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    seed_graveyard(&mut engine, p1, 1);
    let spare = on_battlefield(&engine, p0, forest()).unwrap();
    bury(&mut engine, &[spare]);
    cast_from_hand(&mut engine, p0, pyrogoyf());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("any target takes a player");
    let goyf = on_battlefield(&engine, p0, pyrogoyf()).unwrap();
    (engine, goyf)
}

/// "…that creature deals damage equal to its power to any target." It
/// enters a 2/3 and deals 2 to the opponent.
#[test]
fn pyrogoyf_deals_its_power_to_any_target_as_it_enters() {
    let (mut engine, goyf) = a_pyrogoyf_aimed_at_the_opponent();
    assert_eq!(pt(&engine, goyf), (2, 3));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 18);
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Questing Beast.
// ---------------------------------------------------------------------------

fn questing_beast() -> CardIndex {
    card_index("b685757b-521e-4353-a233-97052359723d")
}

/// Sends Questing Beast at seat 1 on seat 0's first turn.
fn questing_beast_attacks(opponent: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[questing_beast()])
        .battlefield(1, opponent)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let beast = on_battlefield(&engine, p0, questing_beast()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { attackers, .. } if attackers.contains(&beast)),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beast, Defender::Player(p1))],
            },
        )
        .expect("haste: it attacks the turn it arrives");
    (engine, beast)
}

/// Both players cast an Oko, Thief of Crowns (4 loyalty); the opponent's
/// ticks up to 6. Questing Beast then attacks the opponent on its
/// controller's second turn. Returns the engine at the declared attack,
/// its controller's Oko and the opponent's.
fn questing_beast_against_an_oko() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[questing_beast(), forest(), island(), forest()])
        .hand(0, &[oko_thief_of_crowns()])
        .battlefield(1, &[forest(), island(), forest()])
        .hand(1, &[oko_thief_of_crowns()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, oko_thief_of_crowns());
    pass_until(&mut engine, stack_is_empty);
    let mine = on_battlefield(&engine, p0, oko_thief_of_crowns()).expect("its own Oko");

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, oko_thief_of_crowns());
    pass_until(&mut engine, stack_is_empty);
    let theirs = on_battlefield(&engine, p1, oko_thief_of_crowns()).expect("their Oko");
    activate(&mut engine, p1, oko_thief_of_crowns(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, theirs, CounterKind::Loyalty), 6, "+2");

    let beast = on_battlefield(&engine, p0, questing_beast()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { attackers, .. } if attackers.contains(&beast)),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beast, Defender::Player(p1))],
            },
        )
        .expect("at the opponent");
    (engine, mine, theirs)
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Delney, Streetwise Lookout.
// ---------------------------------------------------------------------------

fn delney_streetwise_lookout() -> CardIndex {
    card_index("245d0ccf-87b6-460a-8b99-9e2079f2d375")
}

/// Every Plains `seat` controls, in battlefield order, so a test can tap a
/// few of them for one spell and keep the rest for the next.
fn plains_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == seat && o.card.is_some_and(|c| c.index == plains())
            })
        })
        .collect()
}

/// Werefox Bodyguard on seat 0's side, cast off three of its five Plains,
/// with its enters trigger aimed at seat 1's Elves and still on the stack.
/// The Elves and the Plains left over.
fn a_bodyguard_aimed_at_their_elves(
    seed: u64,
) -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[werefox_bodyguard()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("p1's Elves");
    let plains = plains_of(&engine, p0);
    assert_eq!(plains.len(), 5);
    tap_mana_where(&mut engine, p0, |id| plains[..3].contains(&id));
    cast_with_floating(&mut engine, p0, werefox_bodyguard());
    let options = pass_until_targets(&mut engine, p0);
    assert!(
        options.contains(&elves),
        "a non-Fox creature is on the offer: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .expect("the Elves are targeted");
    (engine, elves, plains[3..].to_vec())
}

// ---------------------------------------------------------------------------
// Maik's European Highlander, round 2: Voice of Resurgence.
// ---------------------------------------------------------------------------

fn voice_of_resurgence() -> CardIndex {
    card_index("5cbbb3f3-63a4-4983-81ea-8c405b10e63f")
}

// ---------------------------------------------------------------------------
// Maik's European Highlander, round 2: Ravager of the Fells' damage.
// ---------------------------------------------------------------------------

/// Passes into `seat`'s upkeep until Ravager of the Fells' transform trigger
/// asks for its first target, and returns what it offers.
fn upkeep_until_ravager_asks(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    pass_until(engine, |e| {
        e.state().turn.active == seat && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    (options, player_options)
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Garna, the Bloodflame.
// ---------------------------------------------------------------------------

fn garna_the_bloodflame() -> CardIndex {
    card_index("97cb993c-90ae-49ff-8ccb-b4fe317a43ef")
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Extraction Specialist.
// ---------------------------------------------------------------------------

fn extraction_specialist() -> CardIndex {
    card_index("4164034a-5e59-4e40-a150-2c1000b0bd0d")
}

/// Whether `object` can neither attack nor block.
fn held_back(engine: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    keywords(engine, object).contains(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK))
}

/// Seat 0 with Llanowar Elves (mana value 1), Steadfast Guard (2) and
/// Thundering Giant (7) in its graveyard, Extraction Specialist cast and its
/// entering trigger asking for a target. Returns the offer.
fn extraction_specialist_asks(
    extra_hand: &[CardIndex],
    their_hand: &[CardIndex],
) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let p0 = PlayerId::new(0);
    let mut hand = vec![extraction_specialist()];
    hand.extend_from_slice(extra_hand);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                llanowar_elves(),
                steadfast_guard(),
                thundering_giant(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[island(), island(), island(), island(), island()])
        .hand(0, &hand)
        .hand(1, their_hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let dead: Vec<ObjectId> = [llanowar_elves(), steadfast_guard(), thundering_giant()]
        .into_iter()
        .map(|card| on_battlefield(&engine, p0, card).unwrap())
        .collect();
    bury(&mut engine, &dead);
    cast_from_hand(&mut engine, p0, extraction_specialist());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    (engine, options)
}

/// "~ phases out", resolved with the Specialist as its own source: the
/// resolver's `Effect::PhaseOut`, not a status written by hand.
fn phase_out_specialist(engine: &mut Engine<RegistryLookup>, specialist: ObjectId) {
    let p0 = PlayerId::new(0);
    let state = engine.dev_state_mut(p0).expect("the harness trusts itself");
    let mut res = crate::resolve::Resolution {
        source: specialist,
        on_stack: specialist,
        controller: p0,
        effects: vec![baylee_cards_dsl::Effect::PhaseOut { target: None }],
        pc: 0,
        targets: smallvec::SmallVec::new(),
        second_targets: smallvec::SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
    };
    assert!(matches!(
        crate::resolve::run(state, &mut res),
        crate::resolve::Flow::Complete
    ));
    engine.refresh_offer();
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Fiend Artisan.
// ---------------------------------------------------------------------------

fn fiend_artisan() -> CardIndex {
    card_index("43b8456a-3333-4936-a09c-324327619c36")
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: The Reaper, King No More.
// ---------------------------------------------------------------------------

fn the_reaper() -> CardIndex {
    card_index("39b67a4d-6a87-41f0-a86f-b66671ccc20d")
}

/// Destroys `id`, passes once, and stops at the first optional question or
/// at an empty stack, whichever comes first.
#[track_caller]
fn reaper_sees_die(engine: &mut Engine<RegistryLookup>, id: ObjectId) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    crate::sba::destroy(state, id);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
}

/// Whether the engine is asking seat 0 "you may …".
fn asked_may(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::MayDo,
            ..
        }
    )
}

/// Seat 0 casts The Reaper with the opponent's Steadfast Guard and
/// Thundering Giant as the two targets of its entering trigger, and the
/// opponent's Llanowar Elves left alone.
fn the_reaper_marks_two() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[swamp(), mountain(), forest()])
        .battlefield(
            1,
            &[steadfast_guard(), thundering_giant(), llanowar_elves()],
        )
        .hand(0, &[the_reaper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p1, steadfast_guard()).unwrap();
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    let (guard_pt, giant_pt) = (pt(&engine, guard), pt(&engine, giant));
    cast_from_hand(&mut engine, p0, the_reaper());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (0, 2), "up to two target creatures");
    assert!(options.contains(&elves));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![guard, giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    for (id, was) in [(guard, guard_pt), (giant, giant_pt)] {
        assert_eq!(counters_on(&engine, id, CounterKind::M1M1), 1);
        assert_eq!(pt(&engine, id), (was.0 - 1, was.1 - 1));
    }
    assert_eq!(counters_on(&engine, elves, CounterKind::M1M1), 0);
    engine
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Subtlety.
// ---------------------------------------------------------------------------

fn subtlety() -> CardIndex {
    card_index("377179d5-ac83-4d34-b5b1-f3d8caa60f79")
}

/// The opponent casts Steadfast Guard in their main phase and passes; seat 0
/// answers with Subtlety (`hand`), paying with `pay`. Returns the engine at
/// Subtlety's target question, with the Guard spell.
fn subtlety_answers_a_guard(
    battlefield: &[CardIndex],
    hand: &[CardIndex],
    evoke: bool,
) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, battlefield)
        .battlefield(1, &[plains(), plains()])
        .hand(0, hand)
        .hand(1, &[steadfast_guard()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, steadfast_guard());
    let guard = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .expect("the Guard is on the stack");
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let incarnation = in_hand(&engine, p0, subtlety()).unwrap();
    if !evoke {
        tap_all_mana_but(&mut engine, p0, None);
    }
    engine
        .apply(p0, PlayerAction::CastSpell { card: incarnation })
        .expect("flash: on the opponent's turn, with a spell on the stack");
    if evoke {
        // No land: the printed {2}{U}{U} is out of reach, and the evoke
        // cost is the one left.
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!(
                "the evoke cost asks for a blue card, got {:?}",
                engine.pending()
            )
        };
        let blue = in_hand(&engine, p0, brainstorm()).unwrap();
        assert_eq!(options, vec![blue]);
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![blue],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    (engine, guard)
}

/// Aims Subtlety's trigger at `spell`, resolves it, and returns the question
/// its owner is asked.
fn subtlety_sends(engine: &mut Engine<RegistryLookup>, spell: ObjectId) -> Pending {
    let p0 = PlayerId::new(0);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((min, max), (0, 1), "up to one target");
    assert_eq!(options, vec![spell], "the one creature spell on the stack");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![spell],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    engine.pending().clone()
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Omnath, Locus of Creation.
// ---------------------------------------------------------------------------

fn omnath_locus_of_creation() -> CardIndex {
    card_index("cd133d30-51ff-4114-a7d7-029345f0f0d7")
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Mawloc.
// ---------------------------------------------------------------------------

fn mawloc() -> CardIndex {
    card_index("e5d928dc-b465-4cf7-ab11-d5bd3328f8e7")
}

/// Seat 0 casts Mawloc for X = `x` with `lands` Mountains and Forests out,
/// against the opponent's Steadfast Guard and Thundering Giant. Stops at the
/// first question after the spell resolves.
fn mawloc_for(x: u32, lands: usize) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut battlefield = vec![mountain()];
    battlefield.extend(std::iter::repeat_n(forest(), lands - 1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &battlefield)
        .battlefield(1, &[steadfast_guard(), thundering_giant()])
        .hand(0, &[mawloc()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, mawloc());
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("X is announced, got {:?}", engine.pending())
    };
    assert!(min <= x && x <= max, "{min}..={max}");
    engine.apply(p0, PlayerAction::ChooseNumber(x)).unwrap();
    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. })
            || on_battlefield(e, p0, mawloc()).is_some() && stack_is_empty(e)
    });
    engine
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Trumpeting Carnosaur.
// ---------------------------------------------------------------------------

fn trumpeting_carnosaur() -> CardIndex {
    card_index("f2ef8bda-373d-4387-900d-0f1b6ccf72e9")
}

fn damn() -> CardIndex {
    card_index("b01d61cc-9844-4191-86a0-f2db6d42d6e5")
}

/// Seat 0 casts Trumpeting Carnosaur off six Mountains with `stack` on top
/// of its library, the last card on top, and walks until the first
/// question after the Carnosaur entered or an empty stack. Returns the ids
/// of the stacked cards, in `stack`'s order.
fn carnosaur_discovers(
    stack: &[CardIndex],
    theirs: &[CardIndex],
) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    carnosaur_discovers_beside(&[], stack, theirs)
}

/// [`carnosaur_discovers`] with `lands` beside the six Mountains, tapped
/// with them: what they make still floats as the discovered card is cast.
fn carnosaur_discovers_beside(
    lands: &[CardIndex],
    stack: &[CardIndex],
    theirs: &[CardIndex],
) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let p0 = PlayerId::new(0);
    let mut hand = vec![trumpeting_carnosaur()];
    hand.extend_from_slice(stack);
    let mut mine = vec![mountain(); 6];
    mine.extend_from_slice(lands);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &mine)
        .battlefield(1, theirs)
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mut ids = Vec::new();
    for &card in stack {
        let id = engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .iter()
            .copied()
            .rev()
            .find(|&id| {
                engine
                    .state()
                    .object(id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
            })
            .unwrap();
        engine
            .dev_state_mut(p0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Library(p0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        ids.push(id);
    }
    engine.refresh_offer();
    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, trumpeting_carnosaur());
    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. })
            || on_battlefield(e, p0, trumpeting_carnosaur()).is_some() && stack_is_empty(e)
    });
    (engine, ids)
}

/// A Counterspell discovered after the trigger has left the stack has
/// nothing to counter, so it cannot be cast (CR 601.2c): nobody is asked,
/// and it goes to the hand.
#[test]
fn trumpeting_carnosaur_asks_nothing_about_a_spell_with_no_target() {
    let p0 = PlayerId::new(0);
    let (engine, ids) = carnosaur_discovers(&[counterspell()], &[]);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }) && stack_is_empty(&engine),
        "{:?}",
        engine.pending()
    );
    assert_eq!(in_hand(&engine, p0, counterspell()), Some(ids[0]));
}

/// Casting without paying the mana cost is an alternative cost, and a
/// spree mode's cost is an additional one, which is still paid (CR 118.9d,
/// 702.172a). Three Steps Ahead discovered with three Islands' mana still
/// floating: the draw, "+ {2}", is two of it, and the {U} the card prints
/// is not paid. The copy, "+ {3}", is offered too, since the Carnosaur is a
/// creature to copy; both at once, {5}, is more than floats.
#[test]
fn trumpeting_carnosaur_discovers_three_steps_ahead_and_pays_its_spree() {
    let p0 = PlayerId::new(0);
    let (mut engine, _) = carnosaur_discovers_beside(&[island(); 3], &[three_steps_ahead()], &[]);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let offered = choose_modes(&mut engine, p0, 0b100);
    let cost = baylee_core::mana::ManaCost::parse;
    assert_eq!(
        offered.iter().map(|o| (o.kind, o.cost)).collect::<Vec<_>>(),
        [
            (CastModeKind::Modes(0b010), cost("{3}")),
            (CastModeKind::Modes(0b100), cost("{2}")),
        ]
    );
    assert!(on_stack(&engine, three_steps_ahead()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Atraxa, Grand Unifier.
// ---------------------------------------------------------------------------

fn atraxa_grand_unifier() -> CardIndex {
    card_index("abbcb153-0763-44c6-964f-b4ff0eb64257")
}

/// Seat 0 casts Atraxa with `stack` on top of its library (the last card
/// named on top) and lets it enter; its trigger resolves up to the first
/// question. Returns the engine and the stacked cards' ids, in `stack`'s
/// order.
fn atraxa_reveals(stack: &[CardIndex]) -> (Engine<RegistryLookup>, Vec<ObjectId>) {
    let p0 = PlayerId::new(0);
    let mut hand = vec![atraxa_grand_unifier()];
    hand.extend_from_slice(stack);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                forest(),
                plains(),
                island(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mut ids = Vec::new();
    for &card in stack {
        let id = engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .iter()
            .copied()
            .rev()
            .find(|&id| {
                !ids.contains(&id)
                    && engine
                        .state()
                        .object(id)
                        .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
            })
            .unwrap();
        engine
            .dev_state_mut(p0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Library(p0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        ids.push(id);
    }
    engine.refresh_offer();
    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, atraxa_grand_unifier());
    pass_until(&mut engine, |e| {
        !matches!(e.pending(), Pending::Priority { .. })
    });
    (engine, ids)
}

/// The open question, as (card type, options): it must be Atraxa's, asked of
/// seat 0, and "may" — zero or one card.
fn atraxa_question(engine: &Engine<RegistryLookup>) -> Option<(TypeSet, Vec<ObjectId>)> {
    match engine.pending().clone() {
        Pending::ChooseCards {
            player,
            options,
            min: 0,
            max: 1,
            prompt: ChoicePrompt::OneOfType { card_type },
            ..
        } if player == PlayerId::new(0) => Some((card_type, options)),
        _ => None,
    }
}

fn sorted(mut ids: Vec<ObjectId>) -> Vec<ObjectId> {
    ids.sort_unstable();
    ids
}

/// Ten lands revealed (two Mountains on the Plains the deck is made of) are
/// one question, about lands, and nothing is asked about a type no revealed
/// card has.
#[test]
fn atraxa_grand_unifier_asks_only_about_types_revealed() {
    let p0 = PlayerId::new(0);
    let (mut engine, ids) = atraxa_reveals(&[mountain(), mountain()]);
    let (asked, offered) = atraxa_question(&engine).expect("the land question");
    assert_eq!(asked, TypeSet::LAND);
    // Two Mountains and eight of the Plains the deck is made of.
    assert_eq!(offered.len(), 10);
    assert!(ids.iter().all(|id| offered.contains(id)));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ids[1]],
            },
        )
        .unwrap();
    assert!(atraxa_question(&engine).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&ids[1])
    );
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: The Balrog of Moria.
// ---------------------------------------------------------------------------

fn the_balrog_of_moria() -> CardIndex {
    card_index("d73191d8-6f94-4fba-acd2-2d0490e3ac00")
}

/// Seat 0's Balrog is destroyed at a table of `boards.len() + 1` seats,
/// seat `i + 1` holding `boards[i]`, and its dies trigger resolves up to
/// "you may exile it". Returns the engine and the Balrog's id.
fn balrog_dies(boards: &[&[CardIndex]]) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut table =
        Duel::table(SEED, plains(), boards.len() + 1).battlefield(0, &[the_balrog_of_moria()]);
    for (i, board) in boards.iter().enumerate() {
        table = table.battlefield(i + 1, board);
    }
    let mut engine = table.start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let balrog = on_battlefield(&engine, p0, the_balrog_of_moria()).unwrap();
    crate::sba::destroy(engine.dev_state_mut(p0).unwrap(), balrog);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    assert!(in_graveyard(&engine, p0, the_balrog_of_moria()).is_some());
    (engine, balrog)
}

fn balrog_in_exile(engine: &Engine<RegistryLookup>) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(PlayerId::new(0)))
        .iter()
        .any(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == the_balrog_of_moria()))
        })
}

/// An opponent with no creature is passed over, and the Balrog is exiled all
/// the same: "up to one" asks nothing of a player with nothing to name.
#[test]
fn the_balrog_of_moria_is_exiled_with_no_creature_to_name() {
    let p0 = PlayerId::new(0);
    let (mut engine, _) = balrog_dies(&[&[]]);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(balrog_in_exile(&engine));
}

// ---------------------------------------------------------------------------
// Maik's European Highlander: Fury.
// ---------------------------------------------------------------------------

fn fury() -> CardIndex {
    card_index("fbf9f8c5-849f-45d5-8129-5fc683c21a04")
}

/// Fury enters on seat 0's main phase against the opponent's `theirs`:
/// cast off five Mountains, or evoked by exiling a Lightning Bolt with no
/// land at all. Returns the engine at the enters trigger's target question.
fn fury_enters(theirs: &[CardIndex], evoke: bool) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let lands = if evoke { vec![] } else { vec![mountain(); 5] };
    let hand = if evoke {
        vec![fury(), lightning_bolt()]
    } else {
        vec![fury()]
    };
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &lands)
        .battlefield(1, theirs)
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    if evoke {
        let card = in_hand(&engine, p0, fury()).unwrap();
        engine
            .apply(p0, PlayerAction::CastSpell { card })
            .expect("the evoke cost needs no mana");
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!(
                "the evoke cost asks for a red card, got {:?}",
                engine.pending()
            )
        };
        let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
        assert_eq!(options, vec![bolt], "Fury itself is on the stack");
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bolt],
                },
            )
            .unwrap();
    } else {
        cast_from_hand(&mut engine, p0, fury());
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!(
        (min, usize::try_from(max).unwrap()),
        (0, options.len().min(4)),
        "any number, and a fifth target could not be dealt 1"
    );
    engine
}

/// Names `targets` for Fury's trigger.
fn fury_aims(engine: &mut Engine<RegistryLookup>, targets: Vec<ObjectId>) {
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ChooseTargets {
                objects: targets,
                players: vec![],
            },
        )
        .unwrap();
}

/// The share question for one target, checked against what it must say.
fn fury_share(
    engine: &Engine<RegistryLookup>,
    target: ObjectId,
    index: u8,
    of: u8,
    left: u32,
) -> (u32, u32) {
    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason,
    } = engine.pending().clone()
    else {
        panic!("a share is asked, got {:?}", engine.pending())
    };
    assert_eq!(player, PlayerId::new(0));
    assert_eq!(
        reason,
        crate::choice::NumberPrompt::DivideDamage {
            target,
            index,
            of,
            left
        }
    );
    (min, max)
}

fn marked(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine.state().object(id).unwrap().damage
}

// ---------------------------------------------------------------------------
// Maik: Ragavan, Nimble Pilferer (impulse from "that player", dash).
// ---------------------------------------------------------------------------

/// Sends seat 0's Ragavan at `defender` and lets the game run to seat 0's
/// second main phase, the combat damage trigger resolved.
fn ragavan_hits(engine: &mut Engine<RegistryLookup>, ragavan: ObjectId, defender: PlayerId) {
    let p0 = PlayerId::new(0);
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { attackers, .. } if attackers.contains(&ragavan)),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ragavan, Defender::Player(defender))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain && stack_is_empty(e)
    });
}

/// The top card of `seat`'s library.
fn top_of_library(engine: &Engine<RegistryLookup>, seat: PlayerId) -> ObjectId {
    *engine
        .state()
        .zones
        .list(ZoneLocation::Library(seat))
        .last()
        .expect("a library to take from")
}

fn uro_titan_of_nature_s_wrath() -> CardIndex {
    card_index("ee302659-59ed-4eef-babe-451b9ccf7f14")
}

/// Lets Uro's triggers resolve, answering "you may put a land card from your
/// hand onto the battlefield" with `land` (or with nothing) and every other
/// question the way [`answer_one`] does, until the stack is empty again.
fn resolve_uro(engine: &mut Engine<RegistryLookup>, land: Option<ObjectId>) {
    for _ in 0..60 {
        if stack_is_empty(engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            return;
        }
        if let Pending::ChooseCards {
            player,
            options,
            min: 0,
            max: 1,
            prompt: ChoicePrompt::Generic,
            ..
        } = engine.pending().clone()
        {
            let objects = land.filter(|l| options.contains(l)).into_iter().collect();
            engine
                .apply(player, PlayerAction::ChooseObjects { objects })
                .unwrap();
            continue;
        }
        let (player, action) = answer_one(engine).expect("a question to answer");
        engine.apply(player, action).unwrap();
    }
    panic!("Uro's triggers never finished: {:?}", engine.pending());
}

/// Casts Uro from `seat`'s graveyard with escape off the mana floating,
/// exiling every other card offered for the cost, and answers with the
/// cards the question offered.
fn escape_uro(engine: &mut Engine<RegistryLookup>, seat: PlayerId, uro: ObjectId) -> Vec<ObjectId> {
    engine
        .apply(seat, PlayerAction::CastSpell { card: uro })
        .expect("Uro is castable from the graveyard");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let escape = choose_cast_kind(engine, CastModeKind::Escape);
        engine
            .apply(seat, PlayerAction::ChooseMode(escape))
            .unwrap();
    }
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt: ChoicePrompt::CostExile,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected escape's exile, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (5, 5), "exactly five other cards");
    assert!(!options.contains(&uro), "other cards: not Uro itself");
    let chosen = options[..5].to_vec();
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: chosen.clone(),
            },
        )
        .unwrap();
    chosen
}

/// Passes until `seat` is asked to pick something or holds priority in its
/// own first main phase, and hands back what the question offered.
fn picks_before_main(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
) -> Option<(PlayerId, Vec<ObjectId>)> {
    for _ in 0..200 {
        if engine.state().turn.phase == Phase::FirstMain && engine.state().turn.active == seat {
            return None;
        }
        match engine.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } => return Some((player, options)),
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to the main phase: {other:?}"),
        }
    }
    panic!("never reached {seat:?}'s main phase");
}

// ---------------------------------------------------------------------------
// Alpha-era cards (set LEA and its reprints): played by their printed Oracle
// text, one function per card handle, tests grouped by colour in the order
// they were assigned.
// ---------------------------------------------------------------------------

fn northern_paladin() -> CardIndex {
    card_index("f5975294-508a-453e-893a-2fbea2487d17")
}

fn pearled_unicorn() -> CardIndex {
    card_index("c071be90-0531-40cc-af46-0cbe80c4ddd4")
}

fn samite_healer() -> CardIndex {
    card_index("95a0ca48-d924-47f4-86ed-42c673ee778c")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn white_knight() -> CardIndex {
    card_index("ddb021df-ae4a-4ac1-8353-d0b375761714")
}

fn clone() -> CardIndex {
    card_index("42226b87-0746-4ebf-9fd0-108d508462af")
}

fn lord_of_atlantis() -> CardIndex {
    card_index("cc7f290f-ca00-4285-9bdb-4b4402444f30")
}

fn merfolk_of_the_pearl_trident() -> CardIndex {
    card_index("218d9277-c179-4de3-9c7f-79b5a6d4fa38")
}

fn wall_of_water() -> CardIndex {
    card_index("608cc65c-f99a-4ca3-be24-190d2556b411")
}

fn water_elemental() -> CardIndex {
    card_index("c470fbe1-0538-46ec-8741-3143f5e78af8")
}

fn black_knight() -> CardIndex {
    card_index("9456c5b6-946d-403a-8ed0-dff9f921d98c")
}

fn bog_wraith() -> CardIndex {
    card_index("508248d1-09a4-4e41-a4c9-286618e5061e")
}

fn drudge_skeletons() -> CardIndex {
    card_index("c180ed02-07f4-4538-9bea-8c249234a8e2")
}

fn hypnotic_specter() -> CardIndex {
    card_index("759af941-f6a3-4726-91f2-9b1e4e55ea71")
}

fn nightmare() -> CardIndex {
    card_index("375932e6-1b3e-48dc-8154-9b664c3add34")
}

fn plague_rats() -> CardIndex {
    card_index("16cf1cf9-6900-406c-a5b4-0e5750f530e0")
}

fn scathe_zombies() -> CardIndex {
    card_index("e0fefaf0-da20-4d58-8db7-019dba16c780")
}

fn scavenging_ghoul() -> CardIndex {
    card_index("68c0c04e-b0d5-4721-83bf-bf18e8b7e680")
}

fn wall_of_bone() -> CardIndex {
    card_index("8ffe4986-9e09-4421-ad26-296a4c0df9e4")
}

fn will_o_the_wisp() -> CardIndex {
    card_index("8b60fcfe-fb90-4a00-a708-25b59bfc9b5a")
}

fn zombie_master() -> CardIndex {
    card_index("5446c92f-ff22-4e9b-a2f6-e64c8560c1e0")
}

fn dwarven_warriors() -> CardIndex {
    card_index("cfc553cd-3b4c-47a9-bffb-e5790befb32c")
}

fn fire_elemental() -> CardIndex {
    card_index("3912d21e-1ebc-4a81-9dc9-f404248d564a")
}

fn goblin_king() -> CardIndex {
    card_index("d236b3fc-0d3f-4d99-875d-e32a33fe5767")
}

fn hurloon_minotaur() -> CardIndex {
    card_index("8f1dae40-b307-446e-bbd2-86aa35813871")
}

fn ironclaw_orcs() -> CardIndex {
    card_index("c645a616-0d7d-416c-b5f3-057b3a1666a0")
}

fn keldon_warlord() -> CardIndex {
    card_index("acc869f8-dcbe-4d57-baa0-3eef4aceb251")
}

fn mons_s_goblin_raiders() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}

fn sedge_troll() -> CardIndex {
    card_index("a6a43190-cca2-4f07-afe9-8af681d777da")
}

fn shivan_dragon() -> CardIndex {
    card_index("711eea87-0fa3-46e0-a42b-fa5a86455f04")
}

fn uthden_troll() -> CardIndex {
    card_index("d6329dcc-b450-482a-8ee3-45449f7a4b3d")
}

fn wall_of_fire() -> CardIndex {
    card_index("f38c8b47-e8e0-4d2f-b1da-d8d986805a48")
}

fn cockatrice() -> CardIndex {
    card_index("af354337-424c-4c7e-8ca5-6149261368d2")
}

fn craw_wurm() -> CardIndex {
    card_index("6a462a69-3e42-41de-a3aa-a488d9f38d69")
}

fn force_of_nature() -> CardIndex {
    card_index("e3c4c27d-f263-4c69-a4fe-2928136ff68b")
}

fn fungusaur() -> CardIndex {
    card_index("3e771f5b-2de3-4a1b-8281-ab1f7491e5a1")
}

fn ironroot_treefolk() -> CardIndex {
    card_index("b7c0bb85-fb87-4c73-bc1b-7b4dc763c7e8")
}

fn shanodin_dryads() -> CardIndex {
    card_index("998484cc-fefc-4da5-9987-5d6e89599c34")
}

fn thicket_basilisk() -> CardIndex {
    card_index("c4822813-cd81-465d-9fe8-3a4c2dcd31ef")
}

fn wall_of_brambles() -> CardIndex {
    card_index("f8d82a00-c10e-4b9e-9642-d05706900a97")
}

// ---------------------------------------------------------------------
// Alpha batch B: Pirate Ship, Demonic Hordes, Nether Shadow, Sengir
// Vampire, Two-Headed Giant of Foriys, Gaea's Liege.
// ---------------------------------------------------------------------

fn pirate_ship() -> CardIndex {
    card_index("c6b3f924-806d-47d3-b044-72b48470196c")
}

fn sea_serpent() -> CardIndex {
    card_index("c16495fc-784d-4bac-9a68-ed437008df73")
}

fn demonic_hordes() -> CardIndex {
    card_index("2847c8a0-f6aa-4e4a-a7b8-fc116436a264")
}

fn nether_shadow() -> CardIndex {
    card_index("c358b9e2-524c-434b-b3fa-74d2aa6d1df7")
}

fn sengir_vampire() -> CardIndex {
    card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b")
}

fn two_headed_giant_of_foriys() -> CardIndex {
    card_index("38aa31bd-7145-43b9-9409-463d9ad6cd69")
}

fn gaea_s_liege() -> CardIndex {
    card_index("8d134a60-e1e5-4163-8bdc-36af91567185")
}

// ---------------------------------------------------------------------
// Alpha batch D: banding — Benalish Hero, Timber Wolves, Mesa Pegasus
// (CR 702.22).
// ---------------------------------------------------------------------

fn benalish_hero() -> CardIndex {
    card_index("4c81cfb7-8765-4e28-ae33-4287fa9a86cc")
}

fn timber_wolves() -> CardIndex {
    card_index("35d07ac9-b184-4b5f-8192-34b1db042f69")
}

fn mesa_pegasus() -> CardIndex {
    card_index("8161f5b8-6aab-4133-ba2c-2e7b5774153e")
}

/// Declares `attackers`, each aimed at `p1`, once the engine asks for them.
#[track_caller]
fn declare_band_attack(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    p1: PlayerId,
    attackers: &[ObjectId],
) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(p1)))
                    .collect(),
            },
        )
        .expect("the attack is legal");
}

/// Answers the one banding question a single leader with banding raises
/// (CR 508.1e) with `band`, and passes on to the declare-blockers question.
#[track_caller]
fn answer_band_and_reach_blockers(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    band: Vec<ObjectId>,
) {
    assert!(
        matches!(
            engine.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Band { .. },
                ..
            }
        ),
        "expected the band question, got {:?}",
        engine.pending()
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: band })
        .expect("the band answer is legal");
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
}

/// Declares `pairs` as blocks.
#[track_caller]
fn declare_band_blocks(
    engine: &mut Engine<RegistryLookup>,
    p1: PlayerId,
    pairs: &[(ObjectId, ObjectId)],
) {
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: pairs.to_vec(),
            },
        )
        .expect("the block is legal");
}

/// Passes priority until something other than priority is asked, or combat
/// ends — the division question banding raises stands here, before the
/// damage step deals anything (CR 510.1, then 510.2).
#[track_caller]
fn next_combat_question(engine: &mut Engine<RegistryLookup>) -> Pending {
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::Priority { player, .. }
                if engine.state().turn.step != crate::turn::Step::CombatEnd =>
            {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => return other,
        }
    }
    panic!("combat never reached a division or its end")
}

fn nettling_imp() -> CardIndex {
    card_index("c58dfcbf-49e6-4ef0-bd31-ebd81b0cfa41")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

// ---- Abilities no test had fired (L4 sweep, 2026-10-01) ----

/// Declares `attackers` against the first defender the engine offers.
#[track_caller]
fn unf_attack(engine: &mut Engine<RegistryLookup>, seat: PlayerId, attackers: &[ObjectId]) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the attack question")
    };
    let defender = defenders.into_iter().next().expect("a defender");
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: attackers.iter().map(|a| (*a, defender)).collect(),
            },
        )
        .unwrap();
}

// ---- Abilities no test had fired, second sweep (L4, 2026-10-01) ----

fn sheoldred_praetor() -> CardIndex {
    card_index("97652492-7906-4d79-983c-fa1dc1239eba")
}

/// Passes until `done`, answering the questions the walk meets with the
/// smallest legal answer (the first `min` cards offered, no targets).
#[track_caller]
fn run_answering(
    engine: &mut Engine<RegistryLookup>,
    done: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..40 {
        pass_until(engine, |e| {
            done(e)
                || matches!(
                    e.pending(),
                    Pending::ChooseCards { .. } | Pending::ChooseTargets { .. }
                )
        });
        if done(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } => {
                let objects = options.into_iter().take(usize::from(min)).collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects })
                    .unwrap();
            }
            Pending::ChooseTargets { player, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
    panic!("condition never reached");
}

fn their_library(engine: &Engine<RegistryLookup>) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Library(PlayerId::new(1)))
        .len()
}

fn solemn_simulacrum() -> CardIndex {
    card_index("00c0543c-2a1f-4425-8283-4062d74a1637")
}

fn cards_of_in(engine: &Engine<RegistryLookup>, location: ZoneLocation, card: CardIndex) -> usize {
    engine
        .state()
        .zones
        .list(location)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

fn engine_object_is(engine: &Engine<RegistryLookup>, id: ObjectId, card: CardIndex) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
}

// The reanimated permanent must not inherit the earlier spell's evoke cost.
fn assert_reanimated_solitude_stays(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    reanimate: CardIndex,
) {
    reach_main_phase(engine, seat);
    let target = in_graveyard(engine, seat, solitude()).unwrap();
    cast_from_hand(engine, seat, reanimate);
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
    assert!(
        on_battlefield(engine, seat, solitude()).is_some(),
        "a reanimated Solitude was not evoked and stays on the battlefield"
    );
}

fn kazandu_blademaster() -> CardIndex {
    card_index("133f5d30-d883-493e-93a1-cf9583db460b")
}

fn general_tazri() -> CardIndex {
    card_index("b0f19cba-1339-4518-8320-d7b1dcaf2eb0")
}

fn charming_prince() -> CardIndex {
    card_index("c48d844c-3976-4fa5-8e0d-3f0e535e7619")
}

fn surgical_metamorph() -> CardIndex {
    card_index("4f328996-f9dd-4c7a-9548-bc4b9d0d943f")
}

/// Answers Charming Prince's "choose one" with the mode whose index is
/// `mode` among the printed three, looked up by position.
#[track_caller]
fn prince_enters_choosing(engine: &mut Engine<RegistryLookup>, mode: usize) {
    let p0 = PlayerId::new(0);
    cast_from_hand(engine, p0, charming_prince());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        unreachable!("the trigger asks for its mode")
    };
    let pos = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(m) if m == mode))
        .expect("the mode is offered");
    engine.apply(p0, PlayerAction::ChooseMode(pos)).unwrap();
}

// ---------------------------------------------------------------------
// Arabian Nights: Dandân, Hasran Ogress, Hurr Jackal, Junún Efreet,
// Khabál Ghoul, Kird Ape, Repentant Blacksmith.
// ---------------------------------------------------------------------

fn dandan() -> CardIndex {
    card_index("88929373-b2c8-4a81-a809-fed87fd5b0d7")
}

fn hasran_ogress() -> CardIndex {
    card_index("a57d4a9d-4ac5-4a68-adb9-a73754034d7c")
}

fn hurr_jackal() -> CardIndex {
    card_index("d17f5afa-a884-4b99-aa9e-89ddb3d43b22")
}

fn junun_efreet() -> CardIndex {
    card_index("afda663e-c5f7-4182-86f7-d95d71793717")
}

fn khabal_ghoul() -> CardIndex {
    card_index("cb558dda-0c05-426d-aedc-bc07cc54db76")
}

fn kird_ape() -> CardIndex {
    card_index("fbbc3acb-c917-44ff-ac6f-9dd6ebe3f4ad")
}

fn repentant_blacksmith() -> CardIndex {
    card_index("a83073a2-e63d-4105-8bad-9612e411fc85")
}

/// Raise the Alarm — `{1}{W}` instant: "Create two 1/1 white Soldier
/// creature tokens."
///
/// The cheap token maker the Khabál Ghoul test needs beside a creature
/// *card*: its ruling says a creature token put into a graveyard from the
/// battlefield is a creature that died, which is a different claim from a
/// card hitting the yard.
fn raise_the_alarm() -> CardIndex {
    card_index("5b2364d7-a811-4595-a1b4-224c70555ffa")
}

// ---------------------------------------------------------------------------
// Arabian Nights: the assigned six, read off their printed sentences.
// ---------------------------------------------------------------------------

fn war_elephant() -> CardIndex {
    card_index("2b7c9fe0-5a23-4172-b31f-b0d85cd465f6")
}

fn aladdin() -> CardIndex {
    card_index("9a410f83-ed92-4b55-834a-c7cec8f5d1e2")
}

fn erg_raiders() -> CardIndex {
    card_index("7feba745-7d27-4225-bc9d-9b7a8692872d")
}

fn rukh_egg() -> CardIndex {
    card_index("98116aec-2ab1-4bee-b727-9feff6274825")
}

fn sorceress_queen() -> CardIndex {
    card_index("3e4cb1b2-e2cc-4925-a226-6c6f1501d9c1")
}

fn island_fish_jasconius() -> CardIndex {
    card_index("bb217f12-532f-4833-a27a-99e290aa47d0")
}

// ---------------------------------------------------------------------
// Antiquities: the upkeep windows and the artifact redirectors.
// ---------------------------------------------------------------------

fn yawgmoth_demon() -> CardIndex {
    card_index("6c54fc14-2af8-46e8-a4dc-b2a0a88ef2e1")
}

fn argivian_blacksmith() -> CardIndex {
    card_index("80240b6b-d20d-4dfb-a2c5-c272c3b43a70")
}

fn martyrs_of_korlis() -> CardIndex {
    card_index("7ca54a23-f8eb-4982-b4ee-7392e2f2a1b3")
}

fn copper_tablet() -> CardIndex {
    card_index("16d1023b-2162-4010-8bf4-218dbe7c99a0")
}

fn juggernaut() -> CardIndex {
    card_index("4ac9116f-36bc-4d71-b696-d6ee064e1d58")
}

fn dwarven_weaponsmith() -> CardIndex {
    card_index("a3541870-3dc9-4571-be98-c0a2b6c468fb")
}

fn colossus_of_sardia() -> CardIndex {
    card_index("9be9625e-b98b-416b-aac4-9f7b2dfbd39d")
}

fn clockwork_avian() -> CardIndex {
    card_index("3d5b71d4-ed5e-4c6d-be70-bebbb1475257")
}

fn plus_one_zero(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    counters_on(
        engine,
        id,
        CounterKind::Plus {
            power: 1,
            toughness: 0,
        },
    )
}

fn triskelion() -> CardIndex {
    card_index("74f67dcf-5afb-45aa-8d4b-3cdb23f6f2a1")
}

fn onulet() -> CardIndex {
    card_index("598f948b-bdd3-490f-b3e9-f0f9dc470522")
}

fn clay_statue() -> CardIndex {
    card_index("cc1ba59c-bb70-4da9-bbd0-a466075f9053")
}

fn grapeshot_catapult() -> CardIndex {
    card_index("23f73983-0337-4464-8817-5f7596d65b38")
}

fn argivian_archaeologist() -> CardIndex {
    card_index("4b889ec0-6130-4e31-bb02-03fdabd28bee")
}
