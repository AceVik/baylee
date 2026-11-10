//! `cards/instants/mv_2/cabal_ritual.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Add {B}{B}{B}." The pool is read on both sides of the resolution: two
/// Swamps float {B}{B}, the cast takes both of them, and what stands in the
/// pool afterwards is three black and nothing at all beside it.
///
/// Reading the card file cannot say this. `Effect::mana(ManaColor::Black, 3)`
/// is an amount and a colour on paper; whether the engine puts three *black*
/// into the caster's pool — rather than one, or three colourless, or three
/// carrying a rider `total()` counts and `available()` does not — is visible
/// only by casting the spell and looking. The empty pool between the cast and
/// the resolution is what makes those three the spell's own rather than
/// change the Swamps left behind, and the other five colours read at zero are
/// the half that fails against an engine adding mana generously.
///
/// The threshold line is `Coverage::Partial` and is not what this proves: the
/// graveyard stays empty, so the printed default is the only branch here.
#[test]
fn a_ritual_adds_three_black_and_leaves_nothing_else_floating() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(59, forest())
        // Exactly two, so the {1}{B} is paid to the last mana: a third Swamp
        // would leave a black floating that the assertion below could not
        // tell from the spell's own.
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[cabal_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool starts empty, so everything counted below arrived during \
         this test"
    );

    let ritual = in_hand(&engine, p0, cabal_ritual()).expect("the Ritual is in hand");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "two Swamps and nothing else, so {{B}}{{B}} is the whole board's worth"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&ritual),
        "{{B}}{{B}} floating against a cost of {{1}}{{B}}: the engine offers \
         the Ritual, and the test presses what was offered"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("the spell the offer just quoted");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{B}} took both Swamps' mana, so the pool is empty while the \
         spell is on the stack — whatever is in it after this is the \
         Ritual's"
    );

    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        3,
        "\"Add {{B}}{{B}}{{B}}\" is three black mana, and it is black rather \
         than the generic a cost would accept anywhere"
    );
    assert_eq!(
        pool.total(),
        3,
        "three and no fourth, and none of them restricted: `total()` counts \
         the riders `available()` cannot see"
    );
    for color in ManaColor::ALL {
        if color == ManaColor::Black {
            continue;
        }
        assert_eq!(
            pool.available(color),
            0,
            "the Ritual adds one colour, and {color:?} is not it"
        );
    }
    assert!(
        in_graveyard(&engine, p0, cabal_ritual()).is_some(),
        "the mana is there because the instant resolved, and a resolved \
         instant lies in its owner's graveyard (CR 608.2n)"
    );
}
