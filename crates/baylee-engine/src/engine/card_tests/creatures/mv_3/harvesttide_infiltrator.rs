//! `cards/creatures/mv_3/harvesttide_infiltrator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Harvesttide Infiltrator is a {2}{R} 3/2 Human Werewolf with trample and
/// daybound, and the face behind it — Harvesttide Assailant — is a 4/4 Werewolf
/// with trample and nightbound. One game plays both halves: the front face is
/// cast off exactly three Mountains, so the body, the keyword and the emptied
/// pool are read off a permanent that really arrived, and the table then walks
/// a turn in which nobody casts a spell, which is what turns day into night
/// (CR 730.2a) and the daybound permanent over (CR 702.145b). A daybound keyword
/// that turned nothing over would leave the 3/2 standing while every claim
/// about the cast stayed green.
#[test]
fn harvesttide_infiltrator_lands_as_a_three_two_and_turns_over_when_night_falls() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[harvesttide_infiltrator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{R} out of exactly three Mountains — the whole board's mana — so an
    // empty pool afterwards is a statement about the printed cost and not about
    // a board that never held the mana.
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, harvesttide_infiltrator());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, harvesttide_infiltrator()).is_some()
    });
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "three Mountains paid {{2}}{{R}} to the last mana"
    );

    let day = on_battlefield(&engine, p0, harvesttide_infiltrator())
        .expect("the Infiltrator resolved onto the battlefield");
    assert_eq!(
        engine.state().object(day).map(|o| o.face_index),
        Some(0),
        "what arrived on the battlefield is the front face"
    );
    assert_eq!(
        pt(&engine, day),
        (3, 2),
        "the 3/2 body the front face prints, and not the back face's 4/4"
    );
    assert!(
        types(&engine, day).contains(TypeSet::CREATURE),
        "it arrives as a creature and not as a shell waiting on something else"
    );
    let day_kw = keywords(&engine, day);
    assert!(
        day_kw.contains(KeywordSet::TRAMPLE),
        "the printed trample reaches the permanent: {day_kw:?}"
    );
    assert!(
        day_kw.contains(KeywordSet::DAYBOUND),
        "and so does the daybound that is about to turn it over: {day_kw:?}"
    );
    assert!(
        !day_kw.contains(KeywordSet::NIGHTBOUND),
        "the back face's keyword is not on the front face: {day_kw:?}"
    );

    // The turn its controller cast the werewolf on keeps the day; the next turn
    // casts nothing at all, which is what makes it night and turns the daybound
    // permanent over. `pass_until` answers every priority and both combat steps,
    // so the only thing that can stop the walk is the turn itself.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, harvesttide_infiltrator())
            .is_some_and(|id| e.state().object(id).is_some_and(|o| o.face_index == 1))
    });

    let night = on_battlefield(&engine, p0, harvesttide_infiltrator())
        .expect("the werewolf is still on the battlefield, on its other face");
    assert_eq!(
        engine.state().object(night).map(|o| o.face_index),
        Some(1),
        "night fell and the daybound permanent turned over"
    );
    assert_eq!(
        pt(&engine, night),
        (4, 4),
        "the Assailant's 4/4 body, which no daybound keyword alone could produce"
    );
    assert!(
        types(&engine, night).contains(TypeSet::CREATURE),
        "and it is still the creature it was, on the face that came up"
    );
    let night_kw = keywords(&engine, night);
    assert!(
        night_kw.contains(KeywordSet::TRAMPLE),
        "the back face prints trample of its own: {night_kw:?}"
    );
    assert!(
        night_kw.contains(KeywordSet::NIGHTBOUND),
        "and nightbound in place of the daybound: {night_kw:?}"
    );
    assert!(
        !night_kw.contains(KeywordSet::DAYBOUND),
        "a permanent shows only the keywords of the face that is up: {night_kw:?}"
    );
    assert!(
        in_graveyard(&engine, p0, harvesttide_infiltrator()).is_none(),
        "turning over is not dying: the card never left the battlefield"
    );
}
