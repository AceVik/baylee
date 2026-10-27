use super::*;
use baylee_cards_dsl::Filter;
use baylee_core::types::TypeSet;

#[test]
fn negation_logic_order_and_large_constants_are_distinct() {
    let mut f = Forest::new();
    let yes = Filter::HasType(TypeSet::CREATURE).tree(&mut f);
    let no = Filter::Not(&Filter::HasType(TypeSet::CREATURE)).tree(&mut f);
    assert_ne!(yes, no);
    let a = Filter::And(&[
        Filter::HasType(TypeSet::CREATURE),
        Filter::HasType(TypeSet::ARTIFACT),
    ])
    .tree(&mut f);
    let o = Filter::Or(&[
        Filter::HasType(TypeSet::CREATURE),
        Filter::HasType(TypeSet::ARTIFACT),
    ])
    .tree(&mut f);
    assert_ne!(a, o);
    assert_ne!([12_u32, 13].tree(&mut f), [13_u32, 12].tree(&mut f));
    assert_ne!(u32::MAX.tree(&mut f), (u32::MAX - 1).tree(&mut f));
}

#[test]
fn known_summary_collisions_and_all_abilities_are_preserved() {
    let cards = ["Mana Leak", "Negate", "Remove Soul"]
        .map(|name| baylee_cards::all().find(|c| c.name() == name).unwrap());
    let f = catalog(&cards.iter().map(|c| c.index.get()).collect::<Vec<_>>()).unwrap();
    let roots = cards.map(|c| f.roots[&format!("c/{}/0", c.index.get())]);
    assert_ne!(roots[0], roots[1]);
    assert_ne!(roots[1], roots[2]);
    assert_ne!(roots[0], roots[2]);
    let mut f = Forest::new();
    let abilities = [cards[0].abilities_for_face(0)[0]; 12];
    let root = abilities.tree(&mut f);
    assert_eq!(f.nodes[root as usize].children.len(), 12);
    assert!(
        f.nodes
            .iter()
            .enumerate()
            .all(|(i, n)| n.children.iter().all(|&c| (c as usize) < i))
    );
}

#[test]
fn public_identity_is_explicit_and_unknown_selects_nothing() {
    use crate::features4::{Candidate4, EncodedState4};
    use refs::{candidate_refs, card_refs};
    assert!(card_refs(crate::carddsl::UNKNOWN_CARD, 0, 0).is_empty());
    assert_ne!(card_refs(0, 0, 0), card_refs(0, 1, 0));
    let state = EncodedState4::default();
    let mut candidate = Candidate4::default();
    assert!(candidate_refs(&candidate, &state).is_empty());
    candidate.card_id = 0;
    assert_eq!(candidate_refs(&candidate, &state), card_refs(0, 0, 0));
}

#[test]
fn room_active_list_indices_resolve_to_the_printed_door() {
    use crate::features4::{Candidate4, ENTITY_FEAT_WIDTH, EncodedState4};
    let def = baylee_cards::all()
        .find(|c| c.name() == "Walk-In Closet")
        .unwrap();
    let cid = def.index.get();
    let catalog = catalog(&[cid]).unwrap();
    let mut state = EncodedState4 {
        entity_feat: vec![vec![0.0; ENTITY_FEAT_WIDTH]],
        entity_card_ids: vec![cid],
        entity_face_ids: vec![0],
        entity_token_ids: vec![0],
        entity_rule_refs: vec![refs::card_refs(cid, 0, 0)],
        ..Default::default()
    };
    let mut candidate = Candidate4 {
        entity_idx: 0,
        ability_idx: 0,
        ..Default::default()
    };
    for (left, right, active_index, face, index) in [
        (true, false, 0, 0, 0),
        (false, true, 0, 1, 0),
        (true, true, 1, 1, 0),
    ] {
        state.entity_feat[0][39] = f32::from(left);
        state.entity_feat[0][40] = f32::from(right);
        candidate.ability_idx = active_index;
        let selected = refs::candidate_refs(&candidate, &state)
            .into_iter()
            .find(|r| r.role == 7)
            .unwrap();
        assert_eq!(selected.key, format!("a/{cid}/{face}/{index}"));
        assert!(catalog.roots.contains_key(&selected.key));
    }
    // CardDef's static slice is intentional. This tiny test fixture models a
    // future Room whose left door is blank, with rules only on its right.
    let faces = Box::leak(Box::new([
        baylee_cards_dsl::FaceDef {
            abilities: &[],
            ..def.faces[0]
        },
        baylee_cards_dsl::FaceDef { ..def.faces[1] },
    ]));
    let blank_left = baylee_cards_dsl::CardDef { faces, ..*def };
    assert!(!blank_left.abilities_for_face(0).is_empty());
    assert!(super::catalog::face_abilities(&blank_left, 0).is_empty());
    assert!(!super::catalog::face_abilities(&blank_left, 1).is_empty());
}

#[test]
fn changing_opponents_hidden_deal_cannot_change_actor_inputs() {
    use crate::{features4::encode_state4, housedeck::HouseDeck, selfplay::table_for};
    use baylee_core::{ids::PlayerId, preset::AIProfile};
    use baylee_gamehost::Session;
    let deck = HouseDeck::named("victory").unwrap();
    let original = table_for(99, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    let mut changed = original.clone();
    changed.seats[1].deck.reverse();
    let left = Session::new(&original).unwrap();
    let right = Session::new(&changed).unwrap();
    assert_ne!(left.snapshot_hash(), right.snapshot_hash());
    let a = left.observe_view(PlayerId::new(0));
    let b = right.observe_view(PlayerId::new(0));
    let a = encode_state4(&a, None, None, &[]);
    let b = encode_state4(&b, None, None, &[]);
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
}

#[test]
fn copy_exceptions_resolve_their_definition_not_the_primary_face_index() {
    use crate::features4::{Candidate4, ENTITY_FEAT_WIDTH, EncodedState4};
    let effigy = baylee_cards::all()
        .find(|c| c.name() == "Machine God's Effigy")
        .unwrap();
    let cards = ["Goblin Striker", "Thrun, Breaker of Silence"]
        .map(|name| baylee_cards::all().find(|c| c.name() == name).unwrap());
    let mut forest = catalog(&[
        effigy.index.get(),
        cards[0].index.get(),
        cards[1].index.get(),
    ])
    .unwrap();
    let baylee_cards_dsl::AbilityDef::CopyOnEnter { mods, .. } = effigy.abilities[0] else {
        panic!("copy clause")
    };
    let baylee_cards_dsl::CopyMod::GrantAbility(blue) = mods[2] else {
        panic!("quoted blue ability")
    };
    let key = format!("a/{}/0/0/copy/2", effigy.index.get());
    let expected = blue.tree(&mut forest);
    assert_eq!(forest.roots[&key], expected);
    assert_ne!(
        forest.roots[&key],
        forest.roots[&format!("a/{}/0/0", effigy.index.get())]
    );
    for card in cards {
        let index = u32::try_from(card.abilities.len()).unwrap();
        let mut references = refs::card_refs(card.index.get(), 0, 0);
        references.push(refs::RuleRef {
            key: key.clone(),
            role: 6,
            slot: None,
            ability: Some(index),
        });
        let state = EncodedState4 {
            entity_feat: vec![vec![0.0; ENTITY_FEAT_WIDTH]],
            entity_card_ids: vec![card.index.get()],
            entity_face_ids: vec![0],
            entity_token_ids: vec![0],
            entity_rule_refs: vec![references],
            ..Default::default()
        };
        let candidate = Candidate4 {
            entity_idx: 0,
            ability_idx: i64::from(index),
            ..Default::default()
        };
        let selected = refs::candidate_refs(&candidate, &state)
            .into_iter()
            .find(|r| r.role == 7)
            .unwrap();
        assert_eq!(selected.key, key);
        assert!(forest.roots.contains_key(&selected.key));
        assert!(
            !forest
                .roots
                .contains_key(&format!("a/{}/0/{index}", card.index.get()))
        );
    }
}
