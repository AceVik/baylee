//! `cards/creatures/mv_7/fleshpulper_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn fleshpulper_giant() -> CardIndex {
    card_index("7cdeecb5-354a-4a6c-989f-d8c9554befb7")
}

/// Fleshpulper Giant — a 4/4 for {5}{R}{R}. "When this creature enters, you
/// may destroy target creature with toughness 2 or less": the menu holds the
/// Ogre (toughness 2) and not the 4/5 Elemental; answered yes the Ogre dies.
#[test]
fn fleshpulper_giant_destroys_a_small_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(
        &lots(mountain(), 7),
        &[fleshpulper_giant()],
        &[gray_ogre(), earth_elemental()],
    );
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    let elemental = on_battlefield(&engine, p1, earth_elemental()).expect("out");
    pay_exactly_and_cast(&mut engine, fleshpulper_giant());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&ogre), "toughness 2 is small enough");
    assert!(!options.contains(&elemental), "toughness 5 is not");
    point_at(&mut engine, p0, &[ogre], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p1, earth_elemental()).is_some());
    let giant = on_battlefield(&engine, p0, fleshpulper_giant()).expect("out");
    assert_body(&engine, giant, (4, 4), KeywordSet::default());
}

/// The "may": declined, the Ogre lives.
#[test]
fn fleshpulper_giant_may_decline() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&lots(mountain(), 7), &[fleshpulper_giant()], &[gray_ogre()]);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    pay_exactly_and_cast(&mut engine, fleshpulper_giant());
    let _ = pass_until_targets(&mut engine, p0);
    point_at(&mut engine, p0, &[ogre], &[]);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declined");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some(), "spared");
}
