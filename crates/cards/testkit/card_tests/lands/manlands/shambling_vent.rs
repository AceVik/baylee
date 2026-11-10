//! `cards/lands/manlands/shambling_vent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shambling Vent prints three sentences and this scenario plays all three on
/// one permanent: it enters tapped, it taps for {W} or {B}, and {1}{W}{B}
/// turns it into a 2/3 white and black Elemental with lifelink until end of
/// turn — still a land while it is a creature.
///
/// The land is *played* rather than seeded, because `starting_battlefield`
/// places a permanent without an entry, so a Vent put there would arrive
/// untapped with the first sentence unread; and nothing is asked of it before
/// its controller's next turn, because a land that arrives tapped pays for
/// nothing until the untap step (CR 302.6 for the animated half, the tap
/// symbol for the mana half). The mana ability is pressed by hand and not
/// through `tap_all_mana`, because the question *is* the card: "Add {W} or
/// {B}" is two colours and not the five of "any colour".
#[test]
fn shambling_vent_enters_tapped_taps_for_white_or_black_and_animates_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Three mana for {1}{W}{B} off lands that are not the Vent, so the one
    // permanent under test is never also the thing paying for itself.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), swamp()])
        .hand(0, &[shambling_vent()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // "This land enters tapped" — a real land drop, so the printed
    // replacement is what puts it down.
    let vent = play_land(&mut engine, p0, shambling_vent());
    assert!(entered_tapped(&engine, vent), "\"This land enters tapped\"");
    assert!(
        !types(&engine, vent).contains(TypeSet::CREATURE),
        "and a land on the table is nothing else until it is paid for"
    );

    // A turn around: the untap step is what makes either printed sentence
    // payable. The Forest is the turn's land drop, which also keeps the hand
    // at seven for the walk across the end step below.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, vent), "the untap step gave it back");
    let _ = play_land(&mut engine, p0, forest());

    // "{T}: Add {W} or {B}" — a printed mana ability, so it is an ordinary
    // `(source, index)` entry and the question it asks is the card.
    activate(&mut engine, p0, shambling_vent(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{B}}\" is a colour choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped names it");
    assert_eq!(
        options.len(),
        2,
        "\"{{W}} or {{B}}\" is two colours and not a rainbow: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "the two the card prints: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two on offer");
    assert!(is_tapped(&engine, vent), "the {{T}} was its own cost");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "and the colour that was named is in a pool that was empty before it"
    );

    // {1}{W}{B} is three mana, and `legal.abilities` is filtered on what can
    // be paid, so the pool is filled before the offer is read.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(vent, 1)),
        "with {{1}}{{W}}{{B}} in the pool the animate line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, shambling_vent(), 1);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine
        .state()
        .object(vent)
        .expect("the animated land is still an object")
        .characteristics();
    assert!(
        chars.types.contains(TypeSet::LAND) && chars.types.contains(TypeSet::CREATURE),
        "\"it becomes a 2/3 … Elemental creature. It's still a land\": {:?}",
        chars.types
    );
    assert_eq!(
        (chars.power, chars.toughness),
        (Some(2), Some(3)),
        "the body the card prints"
    );
    assert!(
        chars.keywords.contains(KeywordSet::LIFELINK),
        "and the lifelink that comes with it"
    );

    // "Until end of turn": the animation goes with the turn it was made in.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !types(&engine, vent).contains(TypeSet::CREATURE),
        "and on the next turn it is a plain land again"
    );
    assert!(
        types(&engine, vent).contains(TypeSet::LAND),
        "which it never stopped being"
    );
}
