//! A static ability's "you" is whoever controls its source now (CR 109.5,
//! #288).
//!
//! Its effect is not locked in (CR 611.3a): a lord that changes hands pumps
//! its new controller's creatures from that moment and its old controller's
//! no longer, and a creature that changes hands is counted and pumped as its
//! new controller's in the same projection that moved it, because layer 2 is
//! applied before the layers that read it (CR 613.1b). An effect a spell or
//! ability made as it resolved keeps the player who controlled it then.

use super::testkit::{
    Duel, RegistryLookup, card_index, cast_from_hand, cast_with_floating, keep_mulligans,
    on_battlefield, pass_until, pt, quiet_artifact, quiet_creature, reach_their_main_phase,
    stack_is_empty, tap_all_mana, walk_to_own_main,
};
use super::*;
use baylee_core::ids::CardIndex;
use baylee_core::types::TypeSet;

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

/// `{2}{W}{W}` 2/2 "Other creatures you control get +1/+1."
fn kongming() -> CardIndex {
    card_index("21e9e1a9-5d6d-473e-adab-6a1e8e2b0ebd")
}

/// `{2}{W}` "Creatures you control get +1/+1."
fn glorious_anthem() -> CardIndex {
    card_index("e3886fe8-9b76-4613-8891-4ec74657c087")
}

/// `{3}{R}{R}` "Gain control of target creature until end of turn. Untap
/// that creature. It gains haste until end of turn": the pool's Threaten.
fn song_mad_treachery() -> CardIndex {
    card_index("81b61770-2ed5-4a50-84d0-97790002fc5a")
}

/// `{1}{U}` "When this creature enters, exchange control of this creature
/// and up to one target creature an opponent controls."
fn gilded_drake() -> CardIndex {
    card_index("7f06c098-6482-4bf3-a9a1-110d6d5b5703")
}

/// `{2}{B}` "You control your opponents while they're searching their
/// libraries."
fn opposition_agent() -> CardIndex {
    card_index("1f438b8f-fe23-4f3b-ab2e-f6c33676c462")
}

/// `{1}{B}{B}` "Search your library for a card, put that card into your
/// hand, then shuffle. You lose 3 life."
fn grim_tutor() -> CardIndex {
    card_index("e62f8d69-a559-4f13-a5c9-5fb750b4af2c")
}

fn controller(engine: &Engine<RegistryLookup>, id: ObjectId) -> Option<PlayerId> {
    engine.state().object(id).map(|o| o.controller)
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

/// Passes priority until `seat`'s next turn has begun.
fn pass_into_the_turn_of(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let turn = engine.state().turn.number;
    pass_until(engine, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == seat
            && matches!(e.pending(), Pending::Priority { .. })
    });
}

/// The rule's own example, played: Kongming taken until end of turn pumps
/// the taker's other creatures and not its owner's, and goes home with its
/// anthem at the cleanup step.
#[test]
fn a_taken_lord_pumps_its_takers_creatures_until_it_goes_home() {
    let mut engine = Duel::table(288, quiet_creature(), 2)
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[kongming(), quiet_creature()])
        .hand(0, &[song_mad_treachery()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let kongming = on_battlefield(&engine, seat(1), kongming()).expect("seat 1's Kongming");
    let mine = on_battlefield(&engine, seat(0), quiet_creature()).expect("seat 0's Elf");
    let theirs = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");
    assert_eq!((pt(&engine, mine), pt(&engine, theirs)), ((1, 1), (2, 2)));

    treachery_takes(&mut engine, seat(0), kongming);
    assert_eq!(
        (pt(&engine, mine), pt(&engine, theirs)),
        ((2, 2), (1, 1)),
        "the taker's Elf is pumped, the owner's no longer"
    );

    pass_into_the_turn_of(&mut engine, seat(1));
    assert_eq!(controller(&engine, kongming), Some(seat(1)));
    assert_eq!((pt(&engine, mine), pt(&engine, theirs)), ((1, 1), (2, 2)));
}

/// Layer 2 before layer 7c, for the object being projected: a creature that
/// changes hands is pumped by its new controller's anthem, and not by its
/// old one's, in the projection that moved it.
#[test]
fn a_creature_taken_is_pumped_by_its_takers_anthem_at_once() {
    for anthem_is_the_takers in [true, false] {
        let anthem_seat = u8::from(!anthem_is_the_takers);
        let mut takers = vec![mountain(); 5];
        let mut owners = vec![quiet_creature()];
        if anthem_is_the_takers {
            &mut takers
        } else {
            &mut owners
        }
        .push(glorious_anthem());
        let mut engine = Duel::table(288, quiet_creature(), 2)
            .battlefield(0, &takers)
            .battlefield(1, &owners)
            .hand(0, &[song_mad_treachery()])
            .start();
        keep_mulligans(&mut engine);
        assert!(walk_to_own_main(&mut engine, seat(0)));
        let elf = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");

        treachery_takes(&mut engine, seat(0), elf);
        let expected = if anthem_is_the_takers { (2, 2) } else { (1, 1) };
        assert_eq!(
            pt(&engine, elf),
            expected,
            "the anthem is seat {anthem_seat}'s and the Elf is seat 0's now"
        );
    }
}

/// The issue's second example: a Gilded Drake takes Opposition Agent, and
/// from then on the Agent takes over the searches of its new controller's
/// opponents, not those of the player who lost it.
#[test]
fn a_drakes_opposition_agent_takes_searches_over_for_its_new_controller() {
    let mut engine = Duel::table(288, swamp(), 2)
        .battlefield(0, &[island(), island(), swamp(), swamp(), swamp()])
        .battlefield(1, &[opposition_agent(), swamp(), swamp(), swamp()])
        .hand(0, &[gilded_drake(), grim_tutor()])
        .hand(1, &[grim_tutor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let agent = on_battlefield(&engine, seat(1), opposition_agent()).expect("seat 1's Agent");
    cast_from_hand(&mut engine, seat(0), gilded_drake());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    target(&mut engine, seat(0), agent);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(controller(&engine, agent), Some(seat(0)));

    // Seat 0's own search is seat 0's to make.
    cast_from_hand(&mut engine, seat(0), grim_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(
        player,
        seat(0),
        "the Agent is seat 0's, so nobody takes it over"
    );
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine
        .apply(
            seat(0),
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Seat 1's is taken over by the Agent's controller.
    reach_their_main_phase(&mut engine, seat(1));
    cast_from_hand(&mut engine, seat(1), grim_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, seat(0), "seat 0 controls seat 1 while it searches");
}

/// One refresh settles every controller before any later layer reads one
/// (CR 613.1b before 613.1c–f): forcing a second refresh at the same
/// generation changes nothing. The taken Elf is the case a single walk
/// gets wrong, since the anthem's filter reads the Elf's controller while
/// the walk is still deciding it.
#[test]
fn one_refresh_settles_control_before_the_layers_that_read_it() {
    let mut engine = Duel::table(288, quiet_creature(), 2)
        .battlefield(0, &[glorious_anthem()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    let elf = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");
    let state = engine.dev_state_mut(seat(0)).unwrap();
    let filter = crate::effects::EffectFilter::object(state, elf);
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: seat(0),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Control,
        timestamp,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter,
        modifier: baylee_cards_dsl::Modifier::GainControl,
    });
    state.refresh_characteristics();
    let once = pt(&engine, elf);
    let state = engine.dev_state_mut(seat(0)).unwrap();
    state.invalidate_projections();
    state.refresh_characteristics();
    assert_eq!((once, pt(&engine, elf)), ((2, 2), (2, 2)));
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// `{4}{G}` "If an effect would create one or more tokens under your
/// control, it creates twice that many of those tokens instead. If an
/// effect would put one or more counters on a permanent you control, it
/// puts twice that many of those counters on that permanent instead."
fn doubling_season() -> CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}

/// `{4}` "If an artifact or creature entering the battlefield causes a
/// triggered ability of a permanent you control to trigger, that ability
/// triggers an additional time."
fn panharmonicon() -> CardIndex {
    card_index("76678885-3674-443d-b9a2-2a460cf6aac0")
}

/// `{1}{W}` Ally, "Whenever this creature or another Ally you control
/// enters, you may gain life equal to the number of Allies you control."
fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}

/// `{1}{G}` Ally.
fn harabaz_druid() -> CardIndex {
    card_index("ead985ec-f29f-4a3b-b8b1-061142cc5bd1")
}

/// `{1}{W}{U}` "Each opponent can cast spells only any time they could cast
/// a sorcery." and "+1: Until your next turn, you may cast sorcery spells
/// as though they had flash."
fn teferi() -> CardIndex {
    card_index("ae7604bb-4818-45a3-960c-cf3d83f15964")
}

/// A Threaten of the kind no card in the pool casts at a noncreature
/// permanent: `taker` gains control of `object` for `duration`, as the
/// effect a resolving spell makes.
fn threaten(
    engine: &mut Engine<RegistryLookup>,
    taker: PlayerId,
    object: ObjectId,
    duration: baylee_cards_dsl::Duration,
) {
    let state = engine.dev_state_mut(taker).unwrap();
    let filter = crate::effects::EffectFilter::object(state, object);
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: taker,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Control,
        timestamp,
        duration,
        filter,
        modifier: baylee_cards_dsl::Modifier::GainControl,
    });
    state.refresh_characteristics();
    assert_eq!(controller(engine, object), Some(taker));
}

/// Whether every static ability's effect and replacement rule names the
/// player who controls its source now — the invariant the projection keeps.
fn statics_follow_their_sources(engine: &Engine<RegistryLookup>) -> bool {
    let state = engine.state();
    let now = |source: ObjectId| {
        state
            .object(source)
            .filter(|o| o.zone == Zone::Battlefield)
            .map(|o| o.controller)
    };
    state.effects.iter().all(|fx| {
        fx.origin != crate::effects::EffectOrigin::Static
            || fx.source.and_then(now).is_none_or(|c| c == fx.controller)
    }) && state
        .replacement_rules
        .iter()
        .all(|entry| now(entry.source).is_none_or(|c| c == entry.controller))
}

/// The issue's own words: Threaten a Glorious Anthem, and the taker's
/// creatures get +1/+1 while the owner's don't, until the cleanup step
/// hands it back.
#[test]
fn a_threatened_anthem_pumps_its_takers_creatures_until_cleanup() {
    let mut engine = Duel::table(288, quiet_creature(), 2)
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[glorious_anthem(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let anthem = on_battlefield(&engine, seat(1), glorious_anthem()).expect("seat 1's Anthem");
    let mine = on_battlefield(&engine, seat(0), quiet_creature()).expect("seat 0's Elf");
    let theirs = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");

    threaten(
        &mut engine,
        seat(0),
        anthem,
        baylee_cards_dsl::Duration::UntilEndOfTurn,
    );
    assert_eq!((pt(&engine, mine), pt(&engine, theirs)), ((2, 2), (1, 1)));
    assert!(statics_follow_their_sources(&engine));

    pass_into_the_turn_of(&mut engine, seat(1));
    assert_eq!(controller(&engine, anthem), Some(seat(1)));
    assert_eq!((pt(&engine, mine), pt(&engine, theirs)), ((1, 1), (2, 2)));
    assert!(statics_follow_their_sources(&engine));
}

/// A replacement rule from a static ability reads "you" the same way:
/// a taken Doubling Season doubles what is put on its taker's permanents
/// and made under the taker's control.
#[test]
fn a_taken_doubling_season_doubles_for_its_taker() {
    let mut engine = Duel::table(288, quiet_creature(), 2)
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[doubling_season(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let season = on_battlefield(&engine, seat(1), doubling_season()).expect("seat 1's Season");
    let mine = on_battlefield(&engine, seat(0), quiet_creature()).expect("seat 0's Elf");
    let theirs = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");

    threaten(
        &mut engine,
        seat(0),
        season,
        baylee_cards_dsl::Duration::UntilEndOfTurn,
    );
    let state = engine.state();
    assert_eq!(
        (
            crate::replacement::counter_multiplier(state, mine),
            crate::replacement::counter_multiplier(state, theirs),
        ),
        (2, 1),
        "counters on the taker's Elf are doubled, on the owner's not"
    );
    assert_eq!(
        (
            crate::replacement::token_multiplier(state, seat(0)),
            crate::replacement::token_multiplier(state, seat(1)),
        ),
        (2, 1)
    );
    assert!(statics_follow_their_sources(&engine));
}

/// A taken Panharmonicon doubles the triggers of its taker's permanents:
/// an Ally entering under seat 0 makes Ondu Cleric's rally trigger twice.
#[test]
fn a_taken_panharmonicon_doubles_its_takers_triggers() {
    let mut engine = Duel::table(288, forest(), 2)
        .battlefield(0, &[ondu_cleric(), forest(), forest()])
        .battlefield(1, &[panharmonicon()])
        .hand(0, &[harabaz_druid()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let harmonicon = on_battlefield(&engine, seat(1), panharmonicon()).expect("seat 1's");
    threaten(
        &mut engine,
        seat(0),
        harmonicon,
        baylee_cards_dsl::Duration::UntilEndOfTurn,
    );
    let life = engine.state().players[0].life;

    cast_from_hand(&mut engine, seat(0), harabaz_druid());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        life + 4,
        "two Allies, and the rally trigger resolves twice"
    );
}

/// One permanent, both kinds of "you" (CR 109.5). Teferi's static follows
/// Teferi to its new controller; the +1 its old controller activated keeps
/// that player, because it is an effect of a resolved ability (CR 611.2).
#[test]
fn a_resolved_effect_keeps_its_player_while_the_static_follows_the_source() {
    let mut engine = Duel::table(288, island(), 2)
        .battlefield(0, &[teferi()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let teferi = on_battlefield(&engine, seat(0), teferi()).expect("seat 0's Teferi");
    engine
        .apply(
            seat(0),
            PlayerAction::ActivateAbility {
                source: teferi,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    threaten(
        &mut engine,
        seat(1),
        teferi,
        baylee_cards_dsl::Duration::Indefinitely,
    );
    let allows = |engine: &Engine<RegistryLookup>, player, types| {
        crate::casting::timing_allows(
            engine.state(),
            player,
            types,
            baylee_cards_dsl::KeywordSet::EMPTY,
        )
    };
    assert!(
        allows(&engine, seat(1), TypeSet::INSTANT),
        "Teferi is seat 1's, so seat 1 is nobody's opponent under it"
    );
    assert!(
        !allows(&engine, seat(1), TypeSet::SORCERY),
        "the +1 was seat 0's to use"
    );

    pass_into_the_turn_of(&mut engine, seat(1));
    assert!(
        !allows(&engine, seat(0), TypeSet::INSTANT),
        "seat 0 is the opponent of Teferi's controller now"
    );
    assert!(statics_follow_their_sources(&engine));
}

/// A control effect from a static ability depends on the effects that
/// change who controls its source (CR 613.8a): take the Control Magic and
/// the creature comes with it, in the same refresh, and goes back with it.
/// No card in the pool gives control by a static ability, so Control
/// Magic's effect is registered by hand on an artifact.
#[test]
fn a_creature_controlled_by_a_static_follows_its_source_to_a_new_controller() {
    let mut engine = Duel::table(288, quiet_creature(), 2)
        .battlefield(0, &[glorious_anthem()])
        .battlefield(1, &[quiet_creature(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let elf = on_battlefield(&engine, seat(1), quiet_creature()).expect("seat 1's Elf");
    let magic = on_battlefield(&engine, seat(1), quiet_artifact()).expect("seat 1's artifact");
    let state = engine.dev_state_mut(seat(1)).unwrap();
    let filter = crate::effects::EffectFilter::object(state, elf);
    let timestamp = state.object(magic).unwrap().timestamp;
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(magic),
        controller: seat(1),
        origin: crate::effects::EffectOrigin::Static,
        layer: baylee_cards_dsl::Layer::Control,
        timestamp,
        duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
        filter,
        modifier: baylee_cards_dsl::Modifier::GainControl,
    });

    threaten(
        &mut engine,
        seat(0),
        magic,
        baylee_cards_dsl::Duration::UntilEndOfTurn,
    );
    assert_eq!(
        controller(&engine, elf),
        Some(seat(0)),
        "in the same refresh"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "and pumped as seat 0's in it");
    assert!(statics_follow_their_sources(&engine));
    let state = engine.state();
    assert!(combat::summoning_sick(state, state.object(elf).unwrap()));

    pass_into_the_turn_of(&mut engine, seat(1));
    assert_eq!(
        (controller(&engine, magic), controller(&engine, elf)),
        (Some(seat(1)), Some(seat(1)))
    );
    assert_eq!(pt(&engine, elf), (1, 1));
}

/// `{3}` "Creatures you control are every creature type. The same is true
/// for creature spells you control and creature cards you own that aren't
/// on the battlefield."
fn maskwood_nexus() -> CardIndex {
    card_index("9b2cdbed-c733-409b-b0e4-2c8960c25111")
}

/// The mechanism, not a played scenario: no rule in the pool is consulted
/// in the middle of an event. A replacement rule whose source has left the
/// battlefield keeps the controller its source last had there (its
/// last-known information), so that when a rule like Leyline of the Void's
/// is consulted during a simultaneous event after its source moved, "you"
/// has not become the owner. The rule itself goes at the next sync.
///
/// Maskwood Nexus is what makes the difference visible: an effect that
/// reaches past the battlefield has the refresh project every card, so the
/// Season's own controller in the graveyard is its owner's again.
#[test]
fn a_rule_whose_source_has_left_keeps_its_last_controller() {
    let mut engine = Duel::table(288, quiet_creature(), 2)
        .battlefield(0, &[maskwood_nexus()])
        .battlefield(1, &[doubling_season()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, seat(0)));
    let season = on_battlefield(&engine, seat(1), doubling_season()).expect("seat 1's Season");
    threaten(
        &mut engine,
        seat(0),
        season,
        baylee_cards_dsl::Duration::UntilEndOfTurn,
    );

    let state = engine.dev_state_mut(seat(0)).unwrap();
    state
        .move_object(
            season,
            ZoneLocation::Graveyard(seat(1)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state.refresh_characteristics();
    assert_eq!(
        state.object(season).map(|o| o.controller),
        Some(seat(1)),
        "a new object in its owner's graveyard"
    );
    let rule = state
        .replacement_rules
        .iter()
        .find(|entry| entry.source == season)
        .expect("not dropped until the next sync");
    assert_eq!(rule.controller, seat(0));
}

/// `{2}{U}` "Whenever another artifact you control with mana value 3 or
/// greater enters, create a 0/0 colorless Construct artifact creature token
/// with 'This token gets +1/+1 for each artifact you control.'"
fn simulacrum_synthesizer() -> CardIndex {
    card_index("eb7a1f21-a66d-415b-8520-710b44890bb6")
}

/// `{3}` artifact, mana value 3.
fn chromatic_lantern() -> CardIndex {
    card_index("539f5396-d99a-417d-a84c-dff7930b5900")
}

/// A token's quoted ability is a static ability of the token: a Construct
/// taken until end of turn counts its taker's artifacts, itself among them.
#[test]
fn a_taken_construct_counts_its_takers_artifacts() {
    let mut engine = Duel::table(288, island(), 2)
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .battlefield(
            1,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[song_mad_treachery()])
        .hand(1, &[simulacrum_synthesizer(), chromatic_lantern()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, seat(1));
    tap_all_mana(&mut engine, seat(1));
    cast_with_floating(&mut engine, seat(1), simulacrum_synthesizer());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing else")
    };
    engine
        .apply(seat(1), super::testkit::look_answer(&cards, &[]))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, seat(1), chromatic_lantern());
    pass_until(&mut engine, stack_is_empty);
    let construct = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_none() && o.controller == seat(1))
        })
        .expect("the Lantern made a Construct");
    assert_eq!(
        pt(&engine, construct),
        (3, 3),
        "the Synthesizer, the Lantern, itself"
    );

    pass_into_the_turn_of(&mut engine, seat(0));
    assert!(walk_to_own_main(&mut engine, seat(0)));
    treachery_takes(&mut engine, seat(0), construct);
    assert_eq!(pt(&engine, construct), (4, 4), "three Sol Rings and itself");
    assert!(statics_follow_their_sources(&engine));
}
