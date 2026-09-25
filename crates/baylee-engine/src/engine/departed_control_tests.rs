//! What a player who leaves the game controlled (CR 800.4a, #281).
//!
//! Their effects that gave them control of something end, so it goes back to
//! whoever controls it without them. What they control by default and do
//! not own is exiled: as they leave (CR 800.4a), or later, once the last
//! effect handing it to somebody else ends (CR 800.4c). Control changes are
//! layer-2 effects with timestamps (CR 613.1b, 613.7), so a later taker who
//! leaves hands the object back to an earlier one. Three seats, so the game
//! outlives the leaver.
//!
//! Opposition Agent controls a player while they search (CR 722.2); that
//! ends too.
//!
//! No card in the pool gives control of a creature until end of combat, or
//! for as long as a source stays on the battlefield (Control Magic). Those
//! two expiries are registered by hand, like the effect such a card would
//! make; the rest are played with the printed cards.

use super::testkit::{
    Duel, RegistryLookup, card_index, cast_from_hand, cast_with_floating, in_graveyard,
    keep_mulligans, on_battlefield, pass_until, quiet_artifact, quiet_creature,
    reach_their_main_phase, seed_graveyard, stack_is_empty, tap_all_mana, walk_to_own_main,
};
use super::*;
use baylee_core::ids::CardIndex;

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// `{1}{U}` "When this creature enters, exchange control of this creature
/// and up to one target creature an opponent controls": control for the
/// rest of the game, each way.
fn gilded_drake() -> CardIndex {
    card_index("7f06c098-6482-4bf3-a9a1-110d6d5b5703")
}

/// `{3}{R}{R}` "Gain control of target creature until end of turn. Untap
/// that creature. It gains haste until end of turn": the pool's Threaten.
fn song_mad_treachery() -> CardIndex {
    card_index("81b61770-2ed5-4a50-84d0-97790002fc5a")
}

/// `{B}` "Put target creature card from a graveyard onto the battlefield
/// under your control": control by default of a card somebody else owns.
fn reanimate() -> CardIndex {
    card_index("a044474a-cd72-4e9d-bd8d-a08f2de9cdc0")
}

fn controller(engine: &Engine<RegistryLookup>, id: ObjectId) -> Option<PlayerId> {
    engine.state().object(id).map(|o| o.controller)
}

fn exiled(engine: &Engine<RegistryLookup>, owner: PlayerId, id: ObjectId) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(owner))
        .contains(&id)
}

/// The turn, phase and step the journal was in when `object` was exiled
/// because a player left.
fn exiled_during(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<(u32, Phase, Step)> {
    let mut turn = 0;
    let mut now = None;
    for entry in engine.state().journal.entries() {
        match entry.event {
            GameEvent::TurnStarted { number, .. } => turn = number,
            GameEvent::StepChanged { phase, step } => now = Some((phase, step)),
            GameEvent::ZoneChanged {
                object: moved,
                cause: Cause::PlayerLeft,
                ..
            } if moved == object => return now.map(|(phase, step)| (turn, phase, step)),
            _ => {}
        }
    }
    None
}

/// Passes priority for whoever holds it.
fn pass(engine: &mut Engine<RegistryLookup>) {
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("{:?}", engine.pending());
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
}

/// Whether any control effect still names `player` as the one gaining it.
fn holds_control_effects(engine: &Engine<RegistryLookup>, player: PlayerId) -> bool {
    engine
        .state()
        .effects
        .iter()
        .any(|fx| fx.modifier == baylee_cards_dsl::Modifier::GainControl && fx.controller == player)
}

/// Answers a target question on the table with `target`.
fn target(engine: &mut Engine<RegistryLookup>, player: PlayerId, target: ObjectId) {
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { player: asked, options, .. }
            if *asked == player && options.contains(&target)),
        "{:?}",
        engine.pending()
    );
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
}

/// `thief` casts Gilded Drake and exchanges it for `victim`; returns the
/// Drake.
fn drake_takes(engine: &mut Engine<RegistryLookup>, thief: PlayerId, victim: ObjectId) -> ObjectId {
    let owner = controller(engine, victim).expect("the victim is out");
    cast_from_hand(engine, thief, gilded_drake());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    target(engine, thief, victim);
    pass_until(engine, stack_is_empty);
    assert_eq!(controller(engine, victim), Some(thief));
    on_battlefield(engine, owner, gilded_drake()).expect("the Drake went the other way")
}

/// `thief` casts Song-Mad Treachery's front face on `victim`.
fn treachery_takes(engine: &mut Engine<RegistryLookup>, thief: PlayerId, victim: ObjectId) {
    tap_all_mana(engine, thief);
    cast_with_floating(engine, thief, song_mad_treachery());
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let slot = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(0)))
            .expect("the front face is one of the ways to play it");
        engine.apply(thief, PlayerAction::ChooseMode(slot)).unwrap();
    }
    target(engine, thief, victim);
    pass_until(engine, stack_is_empty);
    assert_eq!(controller(engine, victim), Some(thief));
}

/// Seat 0, in its first main phase, reanimates the Elf in seat 1's
/// graveyard; returns it.
fn seat_0_reanimates_seat_1s_elf(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    assert!(walk_to_own_main(engine, seat(0)));
    seed_graveyard(engine, seat(1), 1);
    let elf = in_graveyard(engine, seat(1), quiet_creature()).expect("seeded");
    cast_from_hand(engine, seat(0), reanimate());
    target(engine, seat(0), elf);
    pass_until(engine, stack_is_empty);
    let obj = engine
        .state()
        .object(elf)
        .expect("the same card, on the battlefield");
    assert_eq!(
        (obj.zone, obj.owner, obj.controller),
        (Zone::Battlefield, seat(1), seat(0))
    );
    elf
}

/// The rule's Act of Treason: a creature the leaver took until end of turn
/// is its owner's again as they leave, not at the cleanup step.
#[test]
fn a_creature_taken_until_end_of_turn_goes_home_as_its_taker_leaves() {
    let mut engine = Duel::table(281, quiet_creature(), 3)
        .battlefield(0, &[mountain(); 5])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[song_mad_treachery()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let elf = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");
    treachery_takes(&mut engine, seat(0), elf);

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert_eq!(controller(&engine, elf), Some(seat(1)));
    assert_eq!(engine.state().turn.step, Step::Main, "still seat 0's turn");
    assert!(
        !holds_control_effects(&engine, seat(0)),
        "the effect ended as they left (CR 800.4a)"
    );
}

/// The rule's Mind Control, played with a Gilded Drake. Whichever of the two
/// players leaves, what they were given goes back to the other.
#[test]
fn a_drakes_exchange_ends_with_either_player_who_leaves() {
    for leaver in [seat(0), seat(1)] {
        let mut engine = Duel::table(281, quiet_creature(), 3)
            .battlefield(0, &[island(), island()])
            .battlefield(1, &[quiet_creature()])
            .hand(0, &[gilded_drake()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, seat(0)));
        let elf = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");
        let drake = drake_takes(&mut engine, seat(0), elf);

        engine.apply(leaver, PlayerAction::Concede).unwrap();
        let (kept, owner) = if leaver == seat(0) {
            (elf, seat(1))
        } else {
            (drake, seat(0))
        };
        assert_eq!(
            controller(&engine, kept),
            Some(owner),
            "{leaver:?} left, so what they were given is its owner's again"
        );
        assert!(
            engine
                .state()
                .object(if kept == elf { drake } else { elf })
                .is_none(),
            "{leaver:?}'s own card left the game with them"
        );
    }
}

/// Indefinite control changes are layer-2 effects, so an exchange changes no
/// default controller, and it is still journaled and still restarts
/// summoning sickness (CR 302.6).
#[test]
fn a_drakes_exchange_is_a_control_effect_on_each_card() {
    let mut engine = Duel::table(281, quiet_creature(), 3)
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[gilded_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let elf = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");
    let drake = drake_takes(&mut engine, seat(0), elf);

    let state = engine.state();
    for (id, default, now) in [(elf, seat(1), seat(0)), (drake, seat(0), seat(1))] {
        let obj = state.object(id).unwrap();
        assert_eq!((obj.base_controller, obj.controller), (default, now));
        assert!(state.journal.entries().iter().any(|e| e.event
            == GameEvent::ControllerChanged {
                object: id,
                old: default,
                new: now,
            }));
    }
    assert!(combat::summoning_sick(state, state.object(elf).unwrap()));
}

/// An effect for the rest of the game that names one card ends when the
/// card moves on (CR 400.7), rather than sit in the effect table.
#[test]
fn a_control_effect_goes_with_the_card_it_named() {
    let mut engine = Duel::table(281, quiet_creature(), 3)
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[gilded_drake()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let elf = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");
    drake_takes(&mut engine, seat(0), elf);
    assert!(holds_control_effects(&engine, seat(0)));

    engine
        .dev_state_mut(seat(0))
        .unwrap()
        .move_object(
            elf,
            ZoneLocation::Graveyard(seat(1)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass(&mut engine);
    assert!(!holds_control_effects(&engine, seat(0)));
    assert!(holds_control_effects(&engine, seat(1)), "the Drake's stays");
}

/// The rule's Bribery: what the leaver put onto the battlefield under their
/// control out of another player's graveyard is exiled as they leave.
#[test]
fn a_creature_the_leaver_reanimated_is_exiled_as_they_leave() {
    let mut engine = Duel::table(281, quiet_creature(), 3)
        .battlefield(0, &[swamp()])
        .hand(0, &[reanimate()])
        .start();
    keep_mulligans(&mut engine);
    let elf = seat_0_reanimates_seat_1s_elf(&mut engine);

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert!(exiled(&engine, seat(1), elf), "into its owner's exile");
    assert_eq!(
        exiled_during(&engine, elf).map(|(_, phase, _)| phase),
        Some(Phase::FirstMain)
    );
}

/// CR 613.7: the later of two control effects wins, so when the later taker
/// leaves, the earlier one's control comes back.
#[test]
fn a_later_taker_who_leaves_hands_control_back_to_the_earlier() {
    let mut engine = Duel::table(281, quiet_creature(), 3)
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[quiet_creature()])
        .battlefield(2, &[mountain(); 5])
        .hand(0, &[gilded_drake()])
        .hand(2, &[song_mad_treachery()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let elf = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");
    drake_takes(&mut engine, seat(0), elf);
    reach_their_main_phase(&mut engine, seat(2));
    treachery_takes(&mut engine, seat(2), elf);

    engine.apply(seat(2), PlayerAction::Concede).unwrap();
    assert_eq!(controller(&engine, elf), Some(seat(0)));
}

/// The other way round: the earlier taker leaves, the later one keeps the
/// creature until end of turn, and then it is its owner's, because the
/// exchange never made the Drake's player its default controller.
#[test]
fn an_earlier_taker_who_leaves_leaves_the_later_ones_control_standing() {
    let mut engine = Duel::table(281, quiet_creature(), 3)
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[quiet_creature()])
        .battlefield(2, &[mountain(); 5])
        .hand(0, &[gilded_drake()])
        .hand(2, &[song_mad_treachery()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let elf = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");
    drake_takes(&mut engine, seat(0), elf);
    reach_their_main_phase(&mut engine, seat(2));
    treachery_takes(&mut engine, seat(2), elf);

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert_eq!(controller(&engine, elf), Some(seat(2)));
    pass_until(&mut engine, |e| e.state().turn.active != seat(2));
    assert_eq!(controller(&engine, elf), Some(seat(1)));
}

/// CR 800.4c at the cleanup step: seat 0 reanimated seat 1's Elf, seat 2
/// took it until end of turn, and seat 0 left in between. The Elf stays
/// while seat 2 has it and is exiled as the effect ends, in that cleanup
/// step and not in the next turn.
#[test]
fn a_leavers_creature_is_exiled_when_a_turns_taker_lets_go() {
    let mut engine = Duel::table(281, quiet_creature(), 3)
        .battlefield(0, &[swamp()])
        .battlefield(2, &[mountain(); 5])
        .hand(0, &[reanimate()])
        .hand(2, &[song_mad_treachery()])
        .start();
    keep_mulligans(&mut engine);
    let elf = seat_0_reanimates_seat_1s_elf(&mut engine);
    reach_their_main_phase(&mut engine, seat(2));
    treachery_takes(&mut engine, seat(2), elf);
    let turn = engine.state().turn.number;

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert_eq!(
        controller(&engine, elf),
        Some(seat(2)),
        "a player in the game controls it"
    );
    pass_until(&mut engine, |e| e.state().turn.active != seat(2));
    assert!(exiled(&engine, seat(1), elf));
    assert_eq!(
        exiled_during(&engine, elf),
        Some((turn, Phase::Ending, Step::Cleanup))
    );
}

/// A control effect of the kind no card in the pool makes, registered by
/// hand for `taker` on `object`: with a source, that source's static
/// ability (Control Magic, `taker` controlling it); without, a resolved
/// spell's.
fn hand_registered_control(
    engine: &mut Engine<RegistryLookup>,
    taker: PlayerId,
    object: ObjectId,
    source: Option<ObjectId>,
    duration: baylee_cards_dsl::Duration,
) {
    let state = engine.dev_state_mut(taker).unwrap();
    let filter = crate::effects::EffectFilter::object(state, object);
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source,
        controller: taker,
        origin: if source.is_some() {
            crate::effects::EffectOrigin::Static
        } else {
            crate::effects::EffectOrigin::Resolution
        },
        layer: baylee_cards_dsl::Layer::Control,
        timestamp,
        duration,
        filter,
        modifier: baylee_cards_dsl::Modifier::GainControl,
    });
    state.refresh_characteristics();
    assert_eq!(controller(engine, object), Some(taker));
}

/// CR 800.4c as a source leaves: Control Magic's effect ends as the Aura
/// does. Stood in for by an artifact of seat 2's.
#[test]
fn a_leavers_creature_is_exiled_when_the_source_of_its_taking_leaves() {
    let mut engine = Duel::table(281, quiet_creature(), 3)
        .battlefield(0, &[swamp()])
        .battlefield(2, &[quiet_artifact()])
        .hand(0, &[reanimate()])
        .start();
    keep_mulligans(&mut engine);
    let elf = seat_0_reanimates_seat_1s_elf(&mut engine);
    let aura = on_battlefield(&engine, seat(2), quiet_artifact()).expect("seat 2's artifact");
    hand_registered_control(
        &mut engine,
        seat(2),
        elf,
        Some(aura),
        baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
    );

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert!(!exiled(&engine, seat(1), elf));
    engine
        .dev_state_mut(seat(2))
        .unwrap()
        .move_object(
            aura,
            ZoneLocation::Graveyard(seat(2)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    pass(&mut engine);
    assert!(exiled(&engine, seat(1), elf));
}

/// CR 800.4c at the end of combat, when "until end of combat" effects
/// expire (CR 511.2): exiled as the combat phase ends, before anybody has
/// priority in the second main phase.
#[test]
fn a_leavers_creature_is_exiled_when_a_combats_taker_lets_go() {
    let mut engine = Duel::table(281, quiet_creature(), 3)
        .battlefield(0, &[swamp()])
        .hand(0, &[reanimate()])
        .start();
    keep_mulligans(&mut engine);
    let elf = seat_0_reanimates_seat_1s_elf(&mut engine);
    reach_their_main_phase(&mut engine, seat(1));
    hand_registered_control(
        &mut engine,
        seat(2),
        elf,
        None,
        baylee_cards_dsl::Duration::UntilEndOfCombat,
    );
    let turn = engine.state().turn.number;

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert!(!exiled(&engine, seat(1), elf));
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(exiled(&engine, seat(1), elf));
    assert_eq!(
        exiled_during(&engine, elf),
        Some((turn, Phase::SecondMain, Step::Main))
    );
}

/// `{2}{B}` "You control your opponents while they're searching their
/// libraries": the one card in the pool that controls a player (CR 722.2).
fn opposition_agent() -> CardIndex {
    card_index("1f438b8f-fe23-4f3b-ab2e-f6c33676c462")
}

/// `{2}{B}` "Search your library for a card, put that card into your hand,
/// then shuffle. You lose 3 life."
fn grim_tutor() -> CardIndex {
    card_index("e62f8d69-a559-4f13-a5c9-5fb750b4af2c")
}

/// The Agent's controller leaves while making seat 1's search for them. The
/// effect that gave them control of seat 1 ends (CR 800.4a), so seat 1 makes
/// the choice (CR 722.5), and the card goes where the tutor sends it: into
/// seat 1's hand, not into exile.
#[test]
fn a_search_taken_over_by_a_leaver_goes_back_to_the_searcher() {
    let mut engine = Duel::table(281, swamp(), 3)
        .battlefield(0, &[opposition_agent()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[grim_tutor()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, seat(1));
    cast_from_hand(&mut engine, seat(1), grim_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, seat(0), "the Agent's controller searches");

    engine.apply(seat(0), PlayerAction::Concede).unwrap();
    assert!(
        !engine
            .state()
            .effects
            .iter()
            .any(|fx| fx.modifier == baylee_cards_dsl::Modifier::SearchTakeover),
        "the effect ended as they left, not at the machine's next pass"
    );
    let Pending::ChooseCards {
        player,
        options: now,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the search is still asking: {:?}", engine.pending());
    };
    assert_eq!(
        (player, prompt, &now),
        (
            seat(1),
            crate::choice::ChoicePrompt::SearchLibrary,
            &options
        )
    );
    let hand = engine.state().zones.list(ZoneLocation::Hand(seat(1))).len();
    engine
        .apply(
            seat(1),
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(seat(1)))
            .contains(&options[0])
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(seat(1))).len(),
        hand + 1
    );
}

/// Opposition Agent is the one card in the pool that controls a player
/// (CR 722.2), through `Modifier::SearchTakeover`, and that is the effect a
/// player leaving ends besides control of objects (CR 800.4a). This fails
/// the day a card controls a player some other way, which
/// `sba::eliminate_player` then has to end too.
#[test]
fn every_card_that_controls_a_player_does_it_by_taking_over_a_search() {
    let mut read = 0;
    let mut takers = Vec::new();
    for (oracle_id, def) in baylee_cards::generated::ALL {
        if matches!(def.coverage, baylee_cards_dsl::Coverage::Unimplemented) {
            continue;
        }
        for face in 0..def.faces.len() {
            let Some(text) = baylee_cards::oracle::face(def.index, face) else {
                continue;
            };
            read += 1;
            if !controls_a_player(text) {
                continue;
            }
            let takes_over = def
                .abilities
                .iter()
                .chain(def.faces.iter().flat_map(|f| f.abilities.iter()))
                .any(|a| {
                    matches!(a, baylee_cards_dsl::AbilityDef::Static(sa)
                        if sa.modifier == baylee_cards_dsl::Modifier::SearchTakeover)
                });
            assert!(
                takes_over,
                "{} ({oracle_id}) prints control of a player, and not as a search \
                 taken over. If the engine does it, sba::eliminate_player must end \
                 it as its player leaves (CR 800.4a) and this test must know it; \
                 if the card's Partial note leaves it out, say so here: {text}",
                def.faces[face].name
            );
            takers.push(def.faces[face].name);
        }
    }
    assert!(
        takers.contains(&"Opposition Agent"),
        "the reader finds the card that does: {takers:?}"
    );
    // 2,656 cards claim some coverage (September 2026); a few have a
    // second face with text.
    assert!(
        (2_000..4_000).contains(&read),
        "read {read} faces of implemented and partial cards"
    );
}

/// Whether Oracle text says somebody controls a player: "you control target
/// player", "gain control of target opponent", "you control your
/// opponents", "you control that player". One clause at a time, so that
/// "…an opponent controls. That player…" and "…you control: Target
/// player…" are two.
fn controls_a_player(text: &str) -> bool {
    text.split(['.', ';', ':', ',', '\n']).any(|sentence| {
        let words: Vec<String> = sentence
            .split_whitespace()
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '’')
                    .to_lowercase()
            })
            .chain([String::new(), String::new()])
            .collect();
        words.windows(4).any(|w| {
            let rest = if w[1] == "of" { &w[2..] } else { &w[1..3] };
            matches!(w[0].as_str(), "control" | "controls" | "controlled")
                && matches!(
                    rest[0].as_str(),
                    "target" | "that" | "each" | "your" | "a" | "an" | "all"
                )
                && matches!(
                    rest[1].as_str(),
                    "player" | "players" | "opponent" | "opponents"
                )
        })
    })
}

#[test]
fn the_player_control_reader_reads_the_printed_forms() {
    for text in [
        "You control target player during that player's next turn.",
        "When you cast this spell, you gain control of target opponent during that player's next turn.",
        "You control your opponents while they're searching their libraries.",
        "You control that player until Word of Command finishes resolving.",
    ] {
        assert!(controls_a_player(text), "{text}");
    }
    for text in [
        "Gain control of target creature that player controls.",
        "Each opponent controls a Food token.",
        "Target player sacrifices a creature. Creatures that player controls get -1/-1.",
        "You control that player's creatures.",
        "Destroy target nonbasic land an opponent controls. That player may search their library.",
        "Tap an untapped creature you control: Target player mills a card.",
    ] {
        assert!(!controls_a_player(text), "{text}");
    }
}
