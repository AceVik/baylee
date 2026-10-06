//! `cards/creatures/mv_6/trumpeting_carnosaur.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "When this creature enters, discover 5." Off the top: a Mountain (a
/// land), a second Carnosaur (mana value 6), then Llanowar Elves — the first
/// nonland card with mana value 5 or less. The two passed over go to the
/// bottom, and the Elves are offered for nothing and cast.
#[test]
fn trumpeting_carnosaur_discovers_past_a_land_and_a_six_and_casts_for_free() {
    let p0 = PlayerId::new(0);
    let (mut engine, ids) =
        carnosaur_discovers(&[llanowar_elves(), trumpeting_carnosaur(), mountain()], &[]);
    let [elves, six, land] = ids[..] else {
        unreachable!()
    };
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::Discover { card },
                ..
            } if *player == p0 && *card == elves
        ),
        "{:?}",
        engine.pending()
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&elves)
    );
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let bottom: Vec<ObjectId> = library.iter().take(2).copied().collect();
    assert!(
        bottom.contains(&six) && bottom.contains(&land),
        "the two passed over are under the library: {bottom:?}"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert!(
        on_stack(&engine, llanowar_elves()).is_some(),
        "cast, and nothing was paid: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
    assert!(in_hand(&engine, p0, llanowar_elves()).is_none());
}

/// Damn has two modes, and its overload is an alternative cost: cast
/// without paying its mana cost, it can only be the one-target mode
/// (CR 118.9a). So yes goes straight to the target, and the one creature
/// named is the one destroyed.
#[test]
fn trumpeting_carnosaur_casts_damn_in_its_one_free_mode() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, _) = carnosaur_discovers(&[damn()], &[steadfast_guard()]);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let guard = on_battlefield(&engine, p1, steadfast_guard()).unwrap();
    let carnosaur = on_battlefield(&engine, p0, trumpeting_carnosaur()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the target of Damn's one mode, got {:?}", engine.pending())
    };
    assert!(options.contains(&guard) && options.contains(&carnosaur));
    let _ = aim_at(&mut engine, p0, guard);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, steadfast_guard()).is_some());
    assert!(on_battlefield(&engine, p0, trumpeting_carnosaur()).is_some());
}

/// "{2}{R}, Discard this card: It deals 3 damage to target creature or
/// planeswalker." From the hand, with the card itself as part of the cost.
#[test]
fn trumpeting_carnosaur_discarded_deals_three_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[mountain(); 3])
        .battlefield(1, &[thundering_giant()])
        .hand(0, &[trumpeting_carnosaur()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, trumpeting_carnosaur(), 1);
    let _ = aim_at(&mut engine, p0, giant);
    assert!(
        in_graveyard(&engine, p0, trumpeting_carnosaur()).is_some(),
        "discarded as the cost"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, thundering_giant()).is_some());
}
