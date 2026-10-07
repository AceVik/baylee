//! `cards/creatures/mv_3/aven_mindcensor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aven Mindcensor is a {2}{W} 2/1 Bird Wizard printing Flash and Flying,
/// and the third line — "if an opponent would search a library, that player
/// searches the top four cards instead" — is the `Coverage::Partial` gap the
/// card head names, so nothing here pretends to reach it. What is proven is
/// the two lines that *are* written, and Flash is proven by its timing: the
/// cast happens during the opponent's main phase, where a creature without
/// Flash is refused the action outright, and the body and Flying are then
/// read off the permanent that landed while it is still the opponent's turn.
#[test]
fn aven_mindcensor_flashes_in_on_the_opponents_turn_and_flies_once_it_lands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(17, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[aven_mindcensor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        on_battlefield(&engine, p0, aven_mindcensor()).is_none(),
        "it starts in hand, not on the table"
    );

    // The opponent's own first main phase: a sorcery-speed creature could
    // not be cast here at all, so this is what Flash buys.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert_eq!(
        engine.state().turn.active,
        p1,
        "the cast below happens on the opponent's turn"
    );

    cast_from_hand(&mut engine, p0, aven_mindcensor());
    pass_until(&mut engine, stack_is_empty);

    let bird = on_battlefield(&engine, p0, aven_mindcensor())
        .expect("three Plains pay {2}{W} on the opponent's turn, which is Flash");
    assert_eq!(
        engine.state().turn.active,
        p1,
        "and it resolved without the turn ever coming back around"
    );
    assert_eq!(pt(&engine, bird), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "Flying is on the permanent, not only in the card's keyword list"
    );
}
