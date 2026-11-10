//! `cards/instants/mv_1/demystify.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Demystify prints one line — "Destroy target enchantment" — for {W}, and
/// the scenario reads each printed word off a different place. The *filter* is
/// the offer: an opponent's Underworld Breach is on it while the Llanowar
/// Elves beside it are not, so `Filter::ENCHANTMENT` is read rather than
/// skipped, and "target" is why the enchantment only dies because it was
/// named. The destruction is the graveyard the card lands in — its owner's,
/// the seat that controlled it — while the creature the spell did not name
/// never moves, and the {W} off the only Plains on this board is what paid.
#[test]
fn demystify_destroys_the_enchantment_it_names_and_leaves_the_creature_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[demystify()])
        // An enchantment across the table, and a creature beside it that
        // "target enchantment" has to decline.
        .battlefield(1, &[underworld_breach(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let doomed = on_battlefield(&engine, p1, underworld_breach()).expect("the enchantment is out");
    let bystander = on_battlefield(&engine, p1, llanowar_elves()).expect("the creature is out");

    cast_from_hand(&mut engine, p0, demystify());
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
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster aims its own spell");
    assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
    assert!(
        options.contains(&doomed),
        "the enchantment across the table is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&bystander),
        "a creature is no enchantment: {options:?}"
    );
    assert_eq!(options.len(), 1, "and that enchantment is the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the enchantment the question offered is the one it destroys");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, underworld_breach()).is_none(),
        "the named enchantment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, underworld_breach()).is_some(),
        "and it is in its owner's graveyard — the seat that controlled it, \
         not the seat that cast the spell"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, demystify()).is_some(),
        "and the instant itself resolved into its caster's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} the only Plains made is what paid for it"
    );
}
