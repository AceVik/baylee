//! `cards/instants/mv_1/three_steps_ahead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Off four Islands with a Llanowar Elves in play and nothing on the stack,
/// Three Steps Ahead offers the copy ({3}{U}) and the draw ({2}{U}) and
/// nothing else: "Counter target spell" has no spell to point at, and the
/// two together cost {5}{U}. The copy alone takes the spell's one instance
/// of "target" and makes a token Elf; the whole {3}{U} is spent.
#[test]
fn three_steps_ahead_offers_what_it_can_pay_for_and_point_at() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[three_steps_ahead()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, three_steps_ahead());
    let offered = choose_modes(&mut engine, p0, 0b010);
    let cost = baylee_core::mana::ManaCost::parse;
    assert_eq!(
        offered.iter().map(|o| (o.kind, o.cost)).collect::<Vec<_>>(),
        [
            (CastModeKind::Modes(0b010), cost("{3}{U}")),
            (CastModeKind::Modes(0b100), cost("{2}{U}")),
        ]
    );
    let _ = aim_at(&mut engine, p0, elves);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(tokens_of(&engine, p0).len(), 1, "a token copy of the Elves");
}

/// Both halves: the Ritual is countered, so it adds no mana, and the Elves
/// are copied — each mode read its own target.
#[test]
fn three_steps_ahead_counters_one_target_and_copies_the_other() {
    let p0 = PlayerId::new(0);
    let (mut engine, ritual, _) = three_steps_ahead_of_a_ritual(&[], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(ritual).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    let [token] = tokens_of(&engine, p0)[..] else {
        panic!("one token: {:?}", tokens_of(&engine, p0))
    };
    assert_eq!(power_of(&engine, token), Some(1), "a 1/1 Elf, not a Ritual");
    assert!(in_graveyard(&engine, p0, three_steps_ahead()).is_some());
}

/// The spell its counter pointed at is gone, countered by a Counterspell in
/// response. That instance of "target" lost everything, the other did not,
/// so the spell resolves and does what its legal target lets it (CR 608.2b):
/// the Elves are copied all the same.
#[test]
fn three_steps_ahead_still_copies_when_its_spell_target_is_gone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut engine, ritual, _) =
        three_steps_ahead_of_a_ritual(&[island(), island()], &[counterspell()]);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, counterspell());
    let _ = aim_at(&mut engine, p1, ritual);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(ritual).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
    let [token] = tokens_of(&engine, p0)[..] else {
        panic!("the copy still happened: {:?}", tokens_of(&engine, p0))
    };
    assert_eq!(power_of(&engine, token), Some(1), "and it copied the Elves");
    assert!(in_graveyard(&engine, p0, three_steps_ahead()).is_some());
}

/// A copy of a modal spell copies the modes chosen for it (CR 700.2g).
/// Storm of Saruman copies the second spell of the turn, a Three Steps
/// Ahead cast for "+ {2} — Draw two cards, then discard a card", and both
/// the copy and the spell draw two and discard one. Before the copy carried
/// its modes it carried none, and resolved to nothing at all.
#[test]
fn three_steps_ahead_copied_draws_and_discards_twice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[swamp(), island(), island(), island(), storm_of_saruman()],
        )
        .hand(0, &[dark_ritual(), three_steps_ahead()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    let library = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, three_steps_ahead());
    // UUU and BBB float, nothing is on the stack to counter and nothing on
    // the battlefield to copy: the draw is the one set there is, and a
    // question with one answer is not asked.
    assert!(
        on_stack(&engine, three_steps_ahead()).is_some(),
        "{:?}",
        engine.pending()
    );
    let mut discards = 0;
    while !stack_is_empty(&engine) {
        if let Pending::ChooseCards {
            player,
            options,
            min,
            ..
        } = engine.pending().clone()
        {
            assert_eq!(player, p0);
            discards += 1;
            let objects = options.into_iter().take(usize::from(min)).collect();
            engine
                .apply(p0, PlayerAction::ChooseObjects { objects })
                .unwrap();
            continue;
        }
        let (player, action) = answer_one(&engine).unwrap();
        engine.apply(player, action).unwrap();
    }
    assert_eq!(library_size(&engine, p0), library - 4, "drew two, twice");
    assert_eq!(discards, 2, "and discarded once each time");
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 2);
}
