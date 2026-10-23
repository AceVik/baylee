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
