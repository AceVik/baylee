//! `cards/creatures/mv_2/fiend_artisan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "This creature gets +1/+1 for each creature card in your graveyard.
/// {X}{B/G}, {T}, Sacrifice another creature: Search your library for a
/// creature card with mana value X or less, put it onto the battlefield, then
/// shuffle. Activate only as a sorcery."
///
/// The library is Thundering Giants (mana value 7) and one Steadfast Guard
/// (2). With X = 2 the Elves are sacrificed — the Artisan itself is not
/// offered — and the search offers the Guard alone. The Elves in the
/// graveyard make the Artisan 2/2.
#[test]
fn fiend_artisan_sacrifices_another_creature_and_finds_one_within_x() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, thundering_giant())
        .battlefield(
            0,
            &[
                fiend_artisan(),
                llanowar_elves(),
                steadfast_guard(),
                swamp(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let artisan = on_battlefield(&engine, p0, fiend_artisan()).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let guard = on_battlefield(&engine, p0, steadfast_guard()).unwrap();
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        state
            .move_object(
                guard,
                ZoneLocation::Library(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, artisan),
        (1, 1),
        "no creature card in the graveyard"
    );

    tap_mana_except(&mut engine, p0, elves);
    activate(&mut engine, p0, fiend_artisan(), 1);
    let Pending::ChooseNumber { min, max, .. } = engine.pending().clone() else {
        panic!("X is announced first, got {:?}", engine.pending())
    };
    assert!(min <= 2 && 2 <= max, "{min}..={max}");
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert_eq!(options, vec![elves], "another creature: not the Artisan");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    assert_eq!(
        pt(&engine, artisan),
        (2, 2),
        "the Elves are a creature card there"
    );

    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!()
    };
    let guard = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == steadfast_guard()))
        })
        .expect("the Guard is in the library");
    assert_eq!(options, vec![guard], "mana value 2 or less: not a Giant");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![guard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, steadfast_guard()).is_some());
}

/// "Activate only as a sorcery": on the opponent's turn the ability is not
/// offered.
#[test]
fn fiend_artisan_searches_only_as_a_sorcery() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, thundering_giant())
        .battlefield(
            0,
            &[
                fiend_artisan(),
                llanowar_elves(),
                swamp(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let artisan = on_battlefield(&engine, p0, fiend_artisan()).unwrap();
    let offered = |e: &Engine<RegistryLookup>| match e.pending() {
        Pending::Priority { legal, .. } => legal
            .abilities
            .iter()
            .any(|(id, i)| *id == artisan && *i == 1),
        _ => false,
    };
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    // The opponent's turn first, with the lands untapped and their mana
    // floating: only the timing can refuse it there.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(tap_mana_except(&mut engine, p0, elves), 3);
    assert!(!offered(&engine), "the opponent's turn");
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(tap_mana_except(&mut engine, p0, elves), 3);
    assert!(offered(&engine), "its own main phase, stack empty");
}
