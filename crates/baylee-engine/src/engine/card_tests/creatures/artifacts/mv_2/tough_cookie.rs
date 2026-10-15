//! `cards/creatures/artifacts/mv_2/tough_cookie.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn tough_cookie() -> CardIndex {
    card_index("8a1a5862-c027-4c15-99a7-1795d0da96da")
}

/// Tough Cookie: a 2/2 `{1}{G}`; "When this creature enters, create a Food
/// token." One artifact token named Food.
#[test]
fn tough_cookie_creates_a_food_token_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[tough_cookie()], &[forest(), forest()], &[]);
    let cookie = cast_and_land(&mut engine, tough_cookie());
    assert_eq!(pt(&engine, cookie), (2, 2));
    assert!(tokens_of(&engine, p0).is_empty(), "not before it resolves");
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    let chars = engine.state().object(tokens[0]).unwrap().characteristics();
    assert_eq!(engine.state().names.get(chars.name), "Food");
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(!chars.types.contains(TypeSet::CREATURE));
}

/// "{2}{G}: Until end of turn, target noncreature artifact you control
/// becomes a 4/4 artifact creature." Aimed at a Mind Stone, the Mind Stone
/// becomes a 4/4 creature, the Cookie stays a 2/2, and it is over next turn.
#[test]
fn tough_cookie_turns_a_noncreature_artifact_into_a_four_four() {
    let p0 = PlayerId::new(0);
    let mut engine = table(
        &[],
        &[tough_cookie(), mind_stone(), forest(), forest(), forest()],
        &[],
    );
    let cookie = on_battlefield(&engine, p0, tough_cookie()).expect("seated");
    let stone = on_battlefield(&engine, p0, mind_stone()).expect("seated");
    assert!(!types(&engine, stone).contains(TypeSet::CREATURE));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, tough_cookie(), 1);
    let options = aim_at(&mut engine, p0, stone);
    assert!(options.contains(&stone));
    assert!(
        !options.contains(&cookie),
        "the Cookie is a creature already: noncreature artifacts only"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(types(&engine, stone).contains(TypeSet::CREATURE));
    assert!(types(&engine, stone).contains(TypeSet::ARTIFACT));
    assert_eq!(pt(&engine, stone), (4, 4), "the target is the 4/4");
    assert_eq!(pt(&engine, cookie), (2, 2), "the Cookie is unchanged");
    to_my_next_turn(&mut engine);
    assert!(!types(&engine, stone).contains(TypeSet::CREATURE));
}

/// "{2}, {T}, Sacrifice this creature: You gain 3 life."
#[test]
fn tough_cookie_sacrifices_itself_to_gain_three_life() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[tough_cookie(), forest(), forest()], &[]);
    let cookie = on_battlefield(&engine, p0, tough_cookie()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, tough_cookie(), 2);
    assert!(
        engine.state().object(cookie).map(|o| o.zone) != Some(Zone::Battlefield),
        "sacrificed as a cost"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 0), 23);
    assert!(in_graveyard(&engine, p0, tough_cookie()).is_some());
}
