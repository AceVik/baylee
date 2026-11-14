use super::*;
use baylee_core::preset::{FormatId, SeatController, SeatSupplies};
use baylee_gamehost::Session;

#[test]
fn tower_supplies_have_known_rule_features_and_distinct_legal_actions() {
    let forest = baylee_cards::decks::by_name("Forest").unwrap();
    let mut preset = baylee_cards::decks::probe_preset(1, forest).unwrap();
    preset.format = FormatId::Custom;
    preset.house_rules.starting_player = Some(0);
    preset.house_rules.supplies = vec![SeatSupplies { seat: 0, healing: 1, mana: 1, seedling: false }];
    for seat in &mut preset.seats { seat.controller = SeatController::Human { user_id: 0 }; }
    let mut session = Session::new(&preset).unwrap();
    let me = PlayerId::new(0);
    session.act_slice(me, PlayerAction::MulliganKeep, 0).unwrap();
    session.act_slice(PlayerId::new(1), PlayerAction::MulliganKeep, 0).unwrap();
    let (pending, view) = session.view_for(me).unwrap();
    let state = encode_state4(&view, Some(&pending), None, &[]);
    let encoded = encode_candidates4(&state, &view, &pending);
    let mut features = Vec::new();
    for potion in &view.command[0] {
        let row = state.find_entity(potion.id).expect("command emblem is an entity");
        assert_eq!(state.entity_token_ids[row], potion.token.unwrap() + 1);
        let action = PlayerAction::ActivateAbility { source: potion.id, ability_index: 0 };
        let candidate = encoded.candidates.iter().find(|c| c.action.as_ref() == Some(&action)).expect("usable potion is a legal trained-seat action");
        assert!(candidate.entity_idx >= 0);
        assert!(candidate.feat[24..48].iter().any(|f| *f != 0.0));
        features.push(candidate.feat[24..48].to_vec());
    }
    assert_eq!(features.len(), 2);
    assert_ne!(features[0], features[1], "healing and mana have distinct rule meanings");
}
