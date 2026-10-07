//! `cards/enchantments/mv_3/contemplation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Contemplation is a {1}{W}{W} enchantment whose entire text is "Whenever
/// you cast a spell, you gain 1 life", so the test plays three casts and asks
/// who owns each of them. Its own arrival gains nothing — an ability functions
/// only from the battlefield (CR 113.6) — the Sol Ring cast right afterwards
/// off the two white the first cast left floating gains exactly one, and the
/// Giant Growth the other seat aims at its own Strix gains p0 nothing at all,
/// because the trigger's `Filter::ControlledByYou` reads the spell and not the
/// table. Every number is read off a life total, which is the only thing this
/// card prints.
#[test]
fn contemplation_gains_one_life_for_your_spells_and_none_for_an_opponents() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[contemplation(), quiet_artifact()])
        .battlefield(1, &[forest(), baleful_strix()])
        .hand(1, &[giant_growth()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("the Strix is on the table");
    let mine = engine.state().players[0].life;
    let theirs = engine.state().players[1].life;

    cast_from_hand(&mut engine, p0, contemplation());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, contemplation()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].life,
        mine,
        "its own cast happened while it was still a spell on the stack: an \
         ability functions only from the battlefield (CR 113.6), so there was \
         nothing of its own to trigger"
    );

    // Two of the five Plains are still floating — a mana pool survives until
    // the step ends (CR 500.5) and this test never leaves p0's main phase —
    // so the second spell is paid for out of the first one's leftovers.
    cast_with_floating(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the second spell resolved, so the cast really happened"
    );
    assert_eq!(
        engine.state().players[0].life,
        mine + 1,
        "\"Whenever you cast a spell, you gain 1 life\" — one spell, one life"
    );

    // The other seat gets priority inside p0's own main phase and answers
    // with a spell of its own: the trigger is a question about who cast the
    // spell, and a whole turn would answer it with a different board.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "passing in a main phase hands priority across the table, got {:?}",
        engine.pending()
    );
    cast_from_hand(&mut engine, p1, giant_growth());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "a pump spell asks for its target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it names the creature");
    assert!(
        options.contains(&strix),
        "the Strix across the table is a creature and a legal target: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![strix],
            },
        )
        .expect("the target the question itself offered");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        mine + 1,
        "the opponent's spell is not a spell *you* cast, so \
         `Filter::ControlledByYou` declines it and no life is gained"
    );
    assert_eq!(
        engine.state().players[1].life,
        theirs,
        "and the trigger would not have paid the other seat either: the one \
         life total that moved in this game is the caster's"
    );
}
