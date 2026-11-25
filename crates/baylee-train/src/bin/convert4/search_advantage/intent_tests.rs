use super::{Event, GameRoots, intent_of};
use serde_json::json;

/// An event of runtime 0852c2a4b, with `ends`, `own_intents` and
/// `taken_back` beside the fields read before.
fn event(own_intents: &serde_json::Value, chosen: usize) -> serde_json::Value {
    json!({"accepted": true, "seat": 0, "record_input_n": 5,
        "action": {"ActivateManaAbility": {"source": 7}},
        "candidates": [{"ActivateManaAbility": {"source": 7}}, "PassPriority"],
        "scripts": [null, null], "worlds": [[0.5, 0.4], [0.5, 0.45]], "chosen": chosen,
        "leaves": {"stopped": 0, "boundary": 2}, "diffs": [0.0, -0.075],
        "ses": [null, 0.025], "ends": [["boundary", "boundary"], ["boundary", "boundary"]],
        "own_intents": own_intents, "taken_back": [false, false]})
}

#[test]
fn the_new_fields_parse_and_own_intents_name_the_spell() {
    let cast = json!({"CastSpell": {"card": 29}});
    let both = event(&json!([cast, cast]), 0);
    let game = json!({"fallback_answers": 0, "refused_answers": 0,
        "search_events": [both.clone()]});
    let roots = GameRoots::from_game(&game).unwrap();
    assert!(roots.roots(false).asks(0, 5), "a 0852c2a4b event is a root");
    let parsed: Event = serde_json::from_value(both).unwrap();
    // The worlds name the spell: it stands, and the record's differs.
    assert_eq!(intent_of(&parsed, Some(30)), (29, "events"));
    let split: Event =
        serde_json::from_value(event(&json!([cast, {"CastSpell": {"card": 31}}]), 1)).unwrap();
    assert_eq!(intent_of(&split, None), (u32::MAX, "events_disagree"));
    let none: Event = serde_json::from_value(event(&json!([null, null]), 0)).unwrap();
    assert_eq!(intent_of(&none, Some(29)), (u32::MAX, "events_none"));
    // Before 0852c2a4b events carry none: the record's chain.
    let old: Event = serde_json::from_value(event(&json!([]), 0)).unwrap();
    assert_eq!(intent_of(&old, Some(29)), (29, "record"));
    assert_eq!(intent_of(&old, None), (u32::MAX, "none"));
    // A pass that let the mana go names no spell.
    let pass: Event =
        serde_json::from_value(event(&json!(["PassPriority", "PassPriority"]), 0)).unwrap();
    assert_eq!(intent_of(&pass, Some(29)), (u32::MAX, "events"));
}
