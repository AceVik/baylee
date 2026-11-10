//! `cards/lands/utility/deserted_temple.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Deserted Temple: `{1}, {T}: Untap target land.` — twice, on a land that
/// needs untapping and on one that does not.
///
/// The card is old and the assertion is new, and it is about the **journal**
/// rather than about the land. Three things untap a permanent here — the
/// untap step, `Effect::UntapTarget` and now `Effect::UntapSelf` — and until
/// this test the first one journalled `GameEvent::ObjectUntapped` (CR 502.3)
/// and the second wrote the bit in silence. An untap from an effect was
/// therefore absent from the game's own record of what happened: a replay
/// reading the journal would show a land that untapped itself out of
/// nowhere, and a "becomes untapped" trigger — which the DSL has no variant
/// for yet, `Trigger::BecomesTapped` having no twin — would never fire.
///
/// It is written on `UntapTarget` and not on the new `UntapSelf`
/// deliberately. New code cannot have a regression; the rule that was wrong
/// is the one eight cards already use, and a test covering only the newcomer
/// would leave those eight exactly as they were.
///
/// **Both branches, because the quiet one is the easier to get wrong.**
/// Untapping a permanent that is already untapped is not an event, and a
/// door that recorded one anyway would put a "became untapped" in the log
/// for a land that did nothing — which is the same defect as the silence,
/// written the other way round.
#[test]
#[allow(clippy::too_many_lines)] // both branches end to end: a tapped land and an untapped one
fn deserted_temple_says_so_when_it_untaps_a_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(941, forest())
        .battlefield(
            0,
            &[
                deserted_temple(),
                deserted_temple(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = |engine: &Engine<RegistryLookup>, card: CardIndex| -> Vec<ObjectId> {
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
                    .is_some_and(|o| o.controller == p0 && o.card.is_some_and(|c| c.index == card))
            })
            .collect()
    };
    let temples = mine(&engine, deserted_temple());
    let forests = mine(&engine, forest());
    assert_eq!((temples.len(), forests.len()), (2, 3));

    // Two Forests pay the two {1}s; the third stays untapped and is what the
    // quiet branch is aimed at.
    for f in &forests[..2] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: *f })
            .expect("a Forest taps for green");
    }
    assert!(is_tapped(&engine, forests[0]));
    assert!(!is_tapped(&engine, forests[2]));

    let untaps_in = |engine: &Engine<RegistryLookup>, mark: usize, id: ObjectId| -> usize {
        engine.journal().entries()[mark..]
            .iter()
            .filter(|e| {
                matches!(
                    e.event,
                    GameEvent::ObjectUntapped { object, cause }
                        if object == id && cause == Cause::Effect
                )
            })
            .count()
    };
    let aim = |engine: &mut Engine<RegistryLookup>, temple: ObjectId, at: ObjectId| {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: temple,
                    ability_index: 1,
                },
            )
            .expect("a Forest's green pays the {1}");
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!("`target land` is a target: {:?}", engine.pending())
        };
        assert!(
            options.contains(&at),
            "any land is a legal target, tapped or not: {options:?}"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![at],
                    players: vec![],
                },
            )
            .expect("a land the engine offered");
        pass_until(engine, stack_is_empty);
    };

    // The land that had something to undo.
    let mark = engine.journal().entries().len();
    aim(&mut engine, temples[0], forests[0]);
    assert!(
        !is_tapped(&engine, forests[0]),
        "the Forest is untapped, which is the half that always worked"
    );
    assert_eq!(
        untaps_in(&engine, mark, forests[0]),
        1,
        "and the journal says it happened, with an effect named as the \
         cause — CR 502.3's turn-based untap has always recorded one, and an \
         untap from an effect recorded nothing at all"
    );

    // The land that did not.
    let mark = engine.journal().entries().len();
    aim(&mut engine, temples[1], forests[2]);
    assert!(!is_tapped(&engine, forests[2]));
    assert_eq!(
        untaps_in(&engine, mark, forests[2]),
        0,
        "an untapped land that is untapped again did not become untapped, \
         and a journal that said otherwise would be inventing an event"
    );
}
