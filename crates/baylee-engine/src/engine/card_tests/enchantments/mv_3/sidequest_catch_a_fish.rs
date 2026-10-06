//! `cards/enchantments/mv_3/sidequest_catch_a_fish.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sidequest: Catch a Fish` // `Cooking Campsite` (`Coverage::Partial`):
/// "At the beginning of your upkeep, look at the top card of your library. If it's an artifact or
/// creature card, you may reveal it and put it into your hand. If you put a card into your hand
/// this way, create a Food token and transform this enchantment. // `{{T}}`: Add `{{W}}`. `{{3}}`, `{{T}}`,
/// Sacrifice an artifact: Put a +1/+1 counter on each creature you control. Activate only as a sorcery."
///
/// Under `Coverage::Partial`, the front-face upkeep reveal-and-transform trigger is omitted,
/// leaving the front face as a `{{2}}{{W}}` enchantment with no abilities. The test casts
/// `Sidequest: Catch a Fish` from hand, confirms it enters as an enchantment on face 0, verifies
/// that with floating mana `LegalActions::abilities` offers no activated abilities on it, and
/// advances to the following turn's upkeep and main phase confirming no trigger fires and it remains on face 0.
#[test]
fn sidequest_catch_a_fish_casts_and_enters_as_enchantment_without_upkeep_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(108, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[sidequest_catch_a_fish()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, sidequest_catch_a_fish());
    pass_until(&mut engine, stack_is_empty);

    let quest = on_battlefield(&engine, p0, sidequest_catch_a_fish())
        .expect("Sidequest: Catch a Fish on battlefield");
    assert_eq!(
        engine.state().object(quest).map(|o| o.face_index),
        Some(0),
        "Sidequest is on face 0"
    );

    let t = types(&engine, quest);
    assert!(
        t.contains(TypeSet::ENCHANTMENT),
        "Sidequest is an enchantment"
    );
    assert!(!t.contains(TypeSet::LAND), "Sidequest is not a land");

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == quest),
        "front face offers no activated abilities with floating mana"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        library_size(&engine, p0),
        lib_before - 1,
        "two turns pass, so p0 draws once for its own draw step (CR 504.1) — \
         and under `Coverage::Partial` the upkeep trigger adds no second card"
    );
    assert_eq!(
        engine.state().object(quest).map(|o| o.face_index),
        Some(0),
        "Sidequest remains on face 0 in the following turn"
    );
}
