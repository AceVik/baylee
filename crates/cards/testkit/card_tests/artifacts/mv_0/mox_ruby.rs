//! `cards/artifacts/mv_0/mox_ruby.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mox Ruby prints one line: "{T}: Add {R}." Being a {0} artifact, it can be
/// cast without spending anything, so its own tap is the entire price — and
/// the board is built so that the red mana has nowhere else to come from: the
/// only land beside it is a Forest, which makes green and never red, and the
/// pool is read empty before the activation. "Exactly one red" is therefore a
/// statement about the Mox and not about a board that happened to have a
/// Mountain on it.
#[test]
fn mox_ruby_taps_for_one_red_off_an_empty_board() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[mox_ruby()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {0} spends nothing, so the pool is empty before the tap and whatever
    // is in it afterwards came off the Mox.
    let card = in_hand(&engine, p0, mox_ruby()).expect("the Mox is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("{0} is affordable on an empty board");
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mox = on_battlefield(&engine, p0, mox_ruby()).expect("the Mox resolved onto the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is still out");
    assert!(
        types(&engine, mox).contains(TypeSet::ARTIFACT),
        "it is the artifact it prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no land was tapped to pay for a zero-cost artifact"
    );

    // Ability 0 is the printed "{T}: Add {R}". Its whole price is its own
    // tap, so it is offered without a single mana floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mox, 0)),
        "an untapped Mox is a paid {{T}}, so the one line the card prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, mox_ruby(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "one red, off one tap");
    assert_eq!(pool.total(), 1, "one mana, and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, mox), "the Mox paid its own {{T}}");
    assert!(
        !is_tapped(&engine, land),
        "and the Forest beside it never moved: the only land on this board \
         makes green, so the red mana has no other source on it"
    );
}
