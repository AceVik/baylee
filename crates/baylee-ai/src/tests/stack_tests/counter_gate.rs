//! Census of the stack-selection predicate over the compiled pool.

use super::*;

/// Which of the pool's spells the counter gate lets resolve (#226), by
/// name. The predicate is a positive list over effects, so a card written
/// later with only those effects joins it without anybody deciding it
/// should, and this list moves. Ancestral Recall (it draws three), Night's
/// Whisper (it costs life) and every permanent spell stay out.
///
/// Open Communications still only draws: its unsupported additional clause
/// keeps it on this list until that clause is implemented. Borne Upon a Wind
/// and Gitaxian Probe now have their additional effects and leave it.
/// Index only reorders the library: the existing predicate also deliberately
/// permits pure selection spells without spending a counterspell.
/// The full corpus (2026-10-14) adds seven more of exactly those shapes:
/// Consider, Deliberate, Introduction to Prophecy and Preordain (look or
/// scry, then draw), Curate, Contingency Plan and Taigam's Scheming (look or
/// surveil into the graveyard, no draw).
/// PR #394's review holds Lim-Dûl's Vault and implements Ransack as `ScryFor`
/// on its chosen player's library. Neither has the former pure-selection
/// shape accepted by this deliberately narrow predicate.
#[test]
fn the_counter_gate_lets_only_a_cantrip_resolve() {
    let mut held: Vec<&str> = baylee_cards::all()
        .filter(|def| {
            let mut o = permanent(obj(1), PlayerId::new(1), 0);
            o.card = Some(baylee_view::CardIdentity {
                index: def.index,
                print: baylee_core::ids::PrintRef::new(0),
                face: 0,
            });
            o.types = def.faces[0].types;
            crate::tactics::only_replaces_itself(&o)
        })
        .map(baylee_cards_dsl::CardDef::name)
        .collect();
    held.sort_unstable();

    // An ability on the stack arrives with no types and its source's
    // card: Opt's card with an ability's blank base is not Opt.
    let mut ability = permanent(obj(1), PlayerId::new(1), 0);
    ability.card = Some(hand_card(1, "Opt").card);
    ability.types = TypeSet::EMPTY;
    ability.stack_item = Some(baylee_view::StackItem::Ability {
        token: None,
        source: obj(2),
        ability: None,
        text: None,
        rules: None,
    });
    assert!(
        !crate::tactics::only_replaces_itself(&ability),
        "an ability was read as the spell its source card prints"
    );

    assert_eq!(
        held,
        [
            "Abeyance",
            "Brainstorm",
            "Consider",
            "Contingency Plan",
            "Curate",
            "Deliberate",
            "Dream Cache",
            "Index",
            "Introduction to Prophecy",
            "Open Communications",
            "Opt",
            "Preordain",
            "Reach Through Mists",
            "Serum Visions",
            "Taigam's Scheming",
            "Whispers of the Muse",
        ]
    );
}
