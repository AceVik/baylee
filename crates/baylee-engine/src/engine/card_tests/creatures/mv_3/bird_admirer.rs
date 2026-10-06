//! `cards/creatures/mv_3/bird_admirer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bird Admirer // Wing Shredder — {2}{G} — a 1/4 Human Archer Werewolf with
/// reach on the front and a 3/5 Werewolf with reach on the back, joined by
/// daybound // nightbound.
///
/// The pair is the whole card, so the scenario plays the cycle around it: the
/// front face lands as a 1/4, a whole turn passes with nobody casting a spell
/// (the printed way to night) and the permanent turns over in place, and then
/// two spells cast in a single turn put the day back and turn it over again.
/// Reach is read on both faces because a back face inherits nothing
/// (CR 712.8e), so the 3/5 body is the only reading that tells them apart.
#[test]
fn bird_admirer_turns_over_when_the_day_becomes_night_and_back_again() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[bird_admirer(), exploration(), fastbond()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{G} off three tapped Forests, and the front face is the one the hand
    // offers: the back face prints `castable_from_hand = false`.
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, bird_admirer());
    pass_until(&mut engine, stack_is_empty);

    let admirer = on_battlefield(&engine, p0, bird_admirer()).expect("the front face resolved");
    assert_eq!(
        engine.state().object(admirer).map(|o| o.face_index),
        Some(0),
        "the card enters with its front face up"
    );
    assert_eq!(pt(&engine, admirer), (1, 4), "Bird Admirer's printed 1/4");
    assert!(
        keywords(&engine, admirer).contains(KeywordSet::REACH),
        "the front face prints reach"
    );

    // Night: a whole turn goes by in which nobody casts a spell, which is the
    // printed condition. The permanent never leaves the battlefield — it
    // changes which face is up, so `on_battlefield` still finds it by the one
    // card it is, and the body is what says which face came up.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, bird_admirer())
            .is_some_and(|id| e.state().object(id).map(|o| o.face_index) == Some(1))
    });
    let shredder =
        on_battlefield(&engine, p0, bird_admirer()).expect("same permanent, the other face up");
    assert_eq!(
        engine.state().object(shredder).map(|o| o.face_index),
        Some(1),
        "Wing Shredder is the face the night puts up"
    );
    assert_eq!(
        pt(&engine, shredder),
        (3, 5),
        "the 3/5 the back face prints, and not the body it turned away from"
    );
    assert!(
        keywords(&engine, shredder).contains(KeywordSet::REACH),
        "reach is printed on the back face too: a face inherits nothing"
    );

    // Day again: at least two spells in one turn is the printed condition, and
    // the two enchantments are {G} apiece, so the three Forests that untapped
    // in this turn are the whole cost of finding out. Each spell resolves
    // before the next is cast, because sorcery timing wants an empty stack.
    reach_their_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three untapped Forests, and the werewolf makes no mana of its own"
    );
    cast_with_floating(&mut engine, p0, exploration());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, fastbond());
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, bird_admirer())
            .is_some_and(|id| e.state().object(id).map(|o| o.face_index) == Some(0))
    });
    let back = on_battlefield(&engine, p0, bird_admirer()).expect("same permanent, front face up");
    assert_eq!(
        pt(&engine, back),
        (1, 4),
        "the front face is the 1/4 it started as, so the pair turns over both ways"
    );
    assert!(
        keywords(&engine, back).contains(KeywordSet::REACH),
        "and reach comes back with it"
    );
}
