//! Historical counts at the turn boundary, observed through a resolving
//! effect rather than a particular printed card's implementation.
use super::synthetic::{
    SyntheticLookup, forest, keep_mulligans, land, permanents, preset_both, tapped, walk_past,
};
use super::*;
use baylee_cards_dsl::prelude::*;

const OBSERVER: u32 = 1910;
const STILL: u32 = 1911;
static OBSERVE: &[AbilityDef] = &[activated!(
    Cost::FREE,
    &[Effect::GainLife {
        amount: Amount::UntappedLandsAtTurnStart,
    }]
)];
static SKIP: &[AbilityDef] = &[static_ability!(
    Filter::Any,
    Modifier::SkipUntapStep {
        who: PlayerRel::EachPlayer
    }
)];

fn lookup() -> SyntheticLookup {
    SyntheticLookup::new(vec![
        land(OBSERVER, "Turn observer", OBSERVE),
        land(STILL, "Still turn", SKIP),
    ])
}

fn advance_until(
    engine: &mut Engine<SyntheticLookup>,
    done: impl Fn(&Engine<SyntheticLookup>) -> bool,
) {
    for _ in 0..200 {
        if done(engine) {
            return;
        }
        let question = engine.pending().clone();
        assert!(walk_past(engine, &question), "unexpected {question:?}");
    }
    panic!("did not reach the expected turn boundary");
}

fn priority(engine: &Engine<SyntheticLookup>, player: PlayerId) -> bool {
    matches!(engine.pending(), Pending::Priority { player: p, .. } if *p == player)
}

/// Resolving the generic amount really changes life by the saved count.
fn observe(engine: &mut Engine<SyntheticLookup>, expected: i32) {
    let p0 = PlayerId::new(0);
    advance_until(engine, |e| priority(e, p0));
    let before = engine.state().players[0].life;
    let source = permanents(engine, OBSERVER)[0];
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    advance_until(engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).is_empty() && priority(e, p0)
    });
    assert_eq!(engine.state().players[0].life - before, expected);
}

#[test]
fn snapshot_precedes_untapping_and_follows_the_active_seat() {
    let p0 = PlayerId::new(0);
    let f = forest();
    // A real nonland is included below by its stable pool lookup.
    let nonland = baylee_cards::by_oracle_id("6ad8011d-3471-4369-9d68-b264cc027487")
        .unwrap()
        .index
        .get();
    let mut engine = Engine::new(
        &preset_both(1910, &[OBSERVER, f, f, f, nonland], &[f, f, f, f, f]),
        lookup(),
    )
    .unwrap();
    keep_mulligans(&mut engine);
    observe(&mut engine, 4);
    let lands: Vec<_> = permanents(&engine, f)
        .into_iter()
        .filter(|id| engine.state().object(*id).unwrap().controller == p0)
        .collect();
    for source in lands.iter().take(2) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: *source })
            .unwrap();
    }
    observe(&mut engine, 4); // Later taps cannot rewrite history.
    advance_until(&mut engine, |e| {
        e.state().turn.number == 2 && priority(e, p0)
    });
    observe(&mut engine, 5); // The observer's controller is not the active player.
    advance_until(&mut engine, |e| {
        e.state().turn.number == 3 && priority(e, p0)
    });
    assert!(lands.iter().all(|id| !tapped(&engine, *id)));
    observe(&mut engine, 2); // Observer + the one land that was already untapped.
}

#[test]
fn skipped_untap_still_records_the_new_turn() {
    let p0 = PlayerId::new(0);
    let f = forest();
    let mut engine =
        Engine::new(&preset_both(1911, &[OBSERVER, STILL, f, f], &[]), lookup()).unwrap();
    keep_mulligans(&mut engine);
    observe(&mut engine, 4);
    let source = permanents(&engine, f)[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source })
        .unwrap();
    advance_until(&mut engine, |e| {
        e.state().turn.number == 2 && priority(e, p0)
    });
    observe(&mut engine, 0);
    advance_until(&mut engine, |e| {
        e.state().turn.number == 3 && priority(e, p0)
    });
    assert!(tapped(&engine, source), "the untap step was skipped");
    observe(&mut engine, 3);
}

#[test]
fn phased_out_lands_are_absent_from_the_pre_phasing_snapshot() {
    let p0 = PlayerId::new(0);
    let f = forest();
    let mut engine = Engine::new(&preset_both(1912, &[OBSERVER, f, f], &[]), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let phased = permanents(&engine, f)[0];
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(phased)
        .unwrap()
        .status
        .insert(Status::PHASED_OUT);
    engine.refresh_offer();
    advance_until(&mut engine, |e| {
        e.state().turn.number == 3 && priority(e, p0)
    });
    assert!(
        !engine
            .state()
            .object(phased)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT),
        "it has phased back in"
    );
    observe(&mut engine, 2); // It was absent when the turn began.
}

#[test]
fn an_observer_entering_later_can_read_the_turn_start() {
    let p0 = PlayerId::new(0);
    let f = forest();
    let mut setup = preset_both(1913, &[f, f], &[]);
    let mut entry = setup.seats[0].deck[0];
    entry.card = baylee_core::ids::CardIndex::new(OBSERVER);
    setup.seats[0].starting_hand = Some(vec![entry]);
    let mut engine = Engine::new(&setup, lookup()).unwrap();
    keep_mulligans(&mut engine);
    advance_until(&mut engine, |e| {
        e.state().turn.phase == Phase::FirstMain && priority(e, p0)
    });
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();
    observe(&mut engine, 2); // The observer itself was not present then.
}

#[test]
fn an_extra_turn_replaces_the_snapshot_even_for_the_same_active_player() {
    let p0 = PlayerId::new(0);
    let f = forest();
    let mut engine = Engine::new(&preset_both(1914, &[OBSERVER, f, f], &[f]), lookup()).unwrap();
    keep_mulligans(&mut engine);
    observe(&mut engine, 3);
    let source = permanents(&engine, f)
        .into_iter()
        .find(|id| engine.state().object(*id).unwrap().controller == p0)
        .unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source })
        .unwrap();
    engine.dev_state_mut(p0).unwrap().extra_turns.push_front(p0);
    advance_until(&mut engine, |e| {
        e.state().turn.number == 2 && priority(e, p0)
    });
    assert_eq!(engine.state().turn.active, p0);
    assert!(!tapped(&engine, source));
    observe(&mut engine, 2);
    advance_until(&mut engine, |e| {
        e.state().turn.number == 3 && priority(e, p0)
    });
    observe(&mut engine, 1);
}
