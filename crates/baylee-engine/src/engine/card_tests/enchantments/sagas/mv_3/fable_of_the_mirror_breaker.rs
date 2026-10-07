//! `cards/enchantments/sagas/mv_3/fable_of_the_mirror_breaker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Chapter III exiles the Saga and returns it transformed as Reflection of
/// Kiki-Jiki, a 2/2 Goblin Shaman creature; a turn later its "{1}, {T}:
/// Create a token that's a copy of another target nonlegendary creature you
/// control, except it has haste. Sacrifice it at the beginning of the next
/// end step." copies the chapter I Goblin Shaman — the Reflection itself is
/// not on the menu — and the copy is gone at the end step.
#[test]
fn fable_transforms_and_its_reflection_copies_another_creature_until_the_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cast_fable(&[]);
    let saga = on_battlefield(&engine, p0, fable_of_the_mirror_breaker())
        .expect("Fable of the Mirror-Breaker on battlefield");
    assert_eq!(counters_on(&engine, saga, CounterKind::Lore), 1);
    let goblin = goblin_shamans(&engine, p0)[0];

    to_fable_chapter_two(&mut engine);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let saga = on_battlefield(&engine, p0, fable_of_the_mirror_breaker()).unwrap();
    assert_eq!(counters_on(&engine, saga, CounterKind::Lore), 2);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);
    let kiki = on_battlefield(&engine, p0, fable_of_the_mirror_breaker())
        .expect("Reflection of Kiki-Jiki on battlefield");
    assert_eq!(engine.state().object(kiki).unwrap().face_index, 1);
    assert_eq!(
        pt(&engine, kiki),
        (2, 2),
        "Reflection of Kiki-Jiki is a 2/2"
    );
    assert!(types(&engine, kiki).contains(TypeSet::CREATURE));

    // It came back this turn, so its {T} waits a turn.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, fable_of_the_mirror_breaker(), 0);
    let menu = aim_at(&mut engine, p0, goblin);
    assert_eq!(menu, vec![goblin], "another creature: not the Reflection");
    pass_until(&mut engine, stack_is_empty);
    let copies: Vec<ObjectId> = goblin_shamans(&engine, p0)
        .into_iter()
        .filter(|id| *id != goblin)
        .collect();
    assert_eq!(copies.len(), 1, "a copy of the Goblin Shaman");
    let copy = copies[0];
    assert_eq!(pt(&engine, copy), (2, 2));
    assert!(
        keywords(&engine, copy).contains(KeywordSet::HASTE),
        "except it has haste"
    );
    assert!(!keywords(&engine, goblin).contains(KeywordSet::HASTE));
    assert!(is_tapped(&engine, kiki), "the {{T}} of the cost");

    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert!(
        engine
            .state()
            .object(copy)
            .is_none_or(|o| o.zone != crate::zone::Zone::Battlefield),
        "sacrificed at the beginning of the next end step"
    );
    assert_eq!(
        goblin_shamans(&engine, p0),
        vec![goblin],
        "the original stays"
    );
}

/// "III — Exile this Saga, then return it to the battlefield transformed
/// under your control." Seat 1 steals seat 0's Fable after chapter I, the
/// Saga goes on under seat 1 (CR 714.3b puts lore counters on the Sagas a
/// player controls), and chapter III is seat 1's ability. Reflection of
/// Kiki-Jiki enters under seat 1 as its own default, with no control effect
/// holding it (the one on the Saga named an object that is gone, CR 400.7),
/// and it is still seat 0's card (CR 108.3). The effect used to return
/// every card under its owner's control, which handed it back to seat 0.
#[test]
fn a_stolen_fable_returns_under_the_thiefs_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = cast_fable(&[]);
    let saga = on_battlefield(&engine, p0, fable_of_the_mirror_breaker()).expect("the Saga");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        // A lasting layer-2 steal, written by the harness.
        let filter = crate::effects::EffectFilter::object(state, saga);
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: p1,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Control,
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter,
            modifier: baylee_cards_dsl::Modifier::GainControl,
        });
        state.refresh_characteristics();
    }
    engine.refresh_offer();
    assert_eq!(
        engine.state().object(saga).map(|o| (o.owner, o.controller)),
        Some((p0, p1)),
        "stolen"
    );
    for _ in 0..600 {
        let back = engine
            .state()
            .object(saga)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield && o.face_index == 1);
        if back && stack_is_empty(&engine) {
            break;
        }
        if let Pending::ChooseCards {
            player,
            prompt: ChoicePrompt::Discard,
            ..
        } = engine.pending().clone()
        {
            assert_eq!(player, p1, "chapter II is the thief's");
            engine
                .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                .unwrap();
            continue;
        }
        let (player, action) = answer_one(&engine).expect("a question to answer");
        engine.apply(player, action).unwrap();
    }
    let kiki = engine
        .state()
        .object(saga)
        .expect("Reflection of Kiki-Jiki");
    assert_eq!(
        (kiki.zone, kiki.face_index),
        (crate::zone::Zone::Battlefield, 1),
        "chapter III turned it over"
    );
    assert_eq!(
        (kiki.owner, kiki.controller, kiki.base_controller),
        (p0, p1, p1),
        "under your control: the thief's, by default, and still seat 0's card"
    );
}
