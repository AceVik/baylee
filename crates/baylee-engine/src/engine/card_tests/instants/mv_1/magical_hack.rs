//! `cards/instants/mv_1/magical_hack.rs`, played.
//!
//! The spell half (a creature spell on the stack, the change carried onto
//! the permanent, a blink shedding it), the hacked land's mana and a dual
//! collapsing to one type are played in `alpha_completion_review::text_copy_review`,
//! `hack_mana_routes` and `alpha_completion_review::hack_intrinsic_review`.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

/// "Change the text of target … permanent by replacing all instances of one
/// basic land type with another. … This effect lasts indefinitely." Bog
/// Wraith's swampwalk, on the battlefield, becomes islandwalk; a full round
/// of turns later it still is, and the card's name is not a basic land type
/// word, so it is what it was (CR 612.2).
#[test]
fn magical_hack_turns_a_permanents_swampwalk_into_islandwalk_for_good() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let hack = card_index("cba229fa-9035-405b-b091-3798898a37ee");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), index::BOG_WRAITH])
        .hand(0, &[hack])
        .battlefield(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wraith = on_battlefield(&engine, p0, index::BOG_WRAITH).expect("the Wraith is out");
    let name = engine
        .state()
        .object(wraith)
        .unwrap()
        .characteristics()
        .name;
    assert!(keywords(&engine, wraith).contains(KeywordSet::SWAMPWALK));

    cast_from_hand(&mut engine, p0, hack);
    let options = aim_at(&mut engine, p0, wraith);
    assert!(
        options.contains(&wraith),
        "\"target spell or permanent\": a creature on the battlefield"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("the pair of words is asked, got {:?}", engine.pending())
    };
    assert_eq!((player, min, max), (p0, 0, 19), "twenty ordered pairs");
    // WUBRG order: Swamp (2) -> Island (1) is 2 * 4 + 1.
    engine.apply(p0, PlayerAction::ChooseNumber(9)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, wraith).contains(KeywordSet::ISLANDWALK));
    assert!(!keywords(&engine, wraith).contains(KeywordSet::SWAMPWALK));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let after = engine.state().object(wraith).unwrap();
    assert_eq!(
        after.characteristics().name,
        name,
        "the name is not a land type word"
    );
    assert!(
        keywords(&engine, wraith).contains(KeywordSet::ISLANDWALK),
        "\"this effect lasts indefinitely\""
    );
    assert!(!keywords(&engine, wraith).contains(KeywordSet::SWAMPWALK));
    assert_eq!(pt(&engine, wraith), (3, 3), "only the text changed");
}
