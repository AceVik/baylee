//! `cards/lands/utility/safe_haven.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Safe Haven prints `{{2}}, {{T}}: Exile target creature you control.` and `At the beginning of your upkeep, you may sacrifice this land. If you do, return each card exiled with this land to the battlefield under its owner's control.`
///
/// Under `Coverage::Implemented`, activating ability 0 for `{{2}}, {{T}}` exiles a controlled creature with a link to Safe Haven.
/// At the beginning of the next upkeep, answering yes to the optional sacrifice trigger sacrifices Safe Haven and returns the exiled creature to the battlefield.
#[test]
fn safe_haven_exiles_creature_and_returns_it_on_upkeep_sacrifice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[safe_haven(), forest(), forest(), young_wolf()])
        .start();
    keep_mulligans(&mut engine);

    // Safe Haven is on the battlefield from the first turn, so its own
    // upkeep trigger asks its question before anybody reaches a main phase.
    // Declined here: nothing is exiled under it yet, and `reach_main_phase`
    // expects a priority it would otherwise never see.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);

    let haven = on_battlefield(&engine, p0, safe_haven()).expect("safe haven on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");

    // Float {{2}} from Forests while keeping Safe Haven untapped.
    tap_mana_except(&mut engine, p0, haven);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!is_tapped(&engine, haven));

    activate(&mut engine, p0, safe_haven(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![wolf]);

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wolf],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, young_wolf()).is_none(),
        "wolf is exiled"
    );
    assert_eq!(
        engine.state().object(wolf).expect("wolf exists").zone,
        Zone::Exile
    );
    assert!(is_tapped(&engine, haven));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // Advance to p0's next upkeep where Safe Haven's trigger asks whether to sacrifice.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });

    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::MayDo,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected YesNo prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, safe_haven()).is_none(),
        "safe haven was sacrificed"
    );
    assert!(
        on_battlefield(&engine, p0, young_wolf()).is_some(),
        "wolf returned to the battlefield"
    );
}

/// Safe Haven's "each card exiled with this land" is the card it exiled and
/// not a later object that card became (CR 400.7). The Wolf it exiled is
/// cast out of exile, and Swords to Plowshares exiles it again. Sacrificing
/// Safe Haven then returns nothing: the Wolf in exile now was exiled with
/// Swords.
///
/// The link rode along with the card through the stack and the battlefield,
/// so Safe Haven brought the Swords' Wolf back.
///
/// Nothing in the pool lets a player cast a card Safe Haven exiled, so the
/// harness grants the permission the engine uses for a card castable from
/// exile (`Rider::PlayableFromExileFor`). The cast itself is an ordinary one.
#[test]
#[allow(clippy::too_many_lines)] // an exile, a cast, a second exile and an upkeep, told in order
fn a_card_cast_out_of_safe_havens_exile_and_exiled_again_stays_exiled() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                safe_haven(),
                forest(),
                forest(),
                forest(),
                plains(),
                young_wolf(),
            ],
        )
        .hand(0, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);

    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("the Wolf");
    let lands_of = |engine: &Engine<RegistryLookup>, land: CardIndex| -> Vec<ObjectId> {
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
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == land))
            })
            .collect()
    };
    let forests = lands_of(&engine, forest());
    let plains = lands_of(&engine, plains());
    assert_eq!((forests.len(), plains.len()), (3, 1));

    // {2}, {T}: Exile target creature you control.
    tap_mana_where(&mut engine, p0, |id| forests[..2].contains(&id));
    activate(&mut engine, p0, safe_haven(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wolf],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(wolf).map(|o| o.zone),
        Some(Zone::Exile)
    );
    assert!(linked(&engine, wolf), "exiled with Safe Haven");

    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(wolf)
        .expect("the Wolf in exile")
        .riders
        .push(crate::object::Rider::PlayableFromExileFor(p0));
    engine.refresh_offer();
    tap_mana_where(&mut engine, p0, |id| id == forests[2]);
    engine
        .apply(p0, PlayerAction::CastSpell { card: wolf })
        .expect("the Wolf is cast from exile");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        on_battlefield(&engine, p0, young_wolf()),
        Some(wolf),
        "the Wolf resolved"
    );

    tap_mana_where(&mut engine, p0, |id| plains.contains(&id));
    cast_with_floating(&mut engine, p0, swords_to_plowshares());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&wolf), "{options:?}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![wolf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(wolf).map(|o| o.zone),
        Some(Zone::Exile),
        "exiled again, by Swords"
    );

    // The next upkeep: sacrifice Safe Haven, and nothing comes back.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, safe_haven()).is_none(),
        "Safe Haven was sacrificed"
    );
    assert_eq!(
        engine.state().object(wolf).map(|o| o.zone),
        Some(Zone::Exile),
        "the Wolf Swords exiled stays in exile"
    );
}
