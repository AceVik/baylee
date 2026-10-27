use super::*;

#[test]
fn detonate_chooses_a_payable_artifact_value_instead_of_repeating_max_x() {
    let mut v = view(0, &[20, 20], vec![]);
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![hand_card(1, "Detonate")];
    v.seats[0].mana_pool.red = 7;
    let mut artifact = permanent(obj(2), PlayerId::new(1), 2);
    artifact.types = baylee_core::types::TypeSet::ARTIFACT;
    artifact.mana_value = 3;
    v.battlefield = vec![artifact];
    let legal = baylee_engine::choice::LegalActions {
        castable: vec![obj(1)],
        can_pass: true,
        ..Default::default()
    };
    let agent = HeuristicAgent::new(AIProfile::SHARP);
    let context = baylee_engine::engine::DecisionContext {
        source: Some(obj(1)),
        printed: baylee_engine::object::PrintedFace::new(v.hand[0].card.index, 0),
        whole_spell: true,
        cost: Some(baylee_core::mana::ManaCost::parse("{X}{R}")),
        ..Default::default()
    };
    assert_eq!(
        agent.spell_or_mana(&v, &legal),
        Some(PlayerAction::CastSpell { card: obj(1) })
    );
    assert_eq!(agent.number(&v, 0, 6, &context), 3);
    v.battlefield[0].keywords = baylee_cards_dsl::KeywordSet::HEXPROOF.bits();
    assert_eq!(agent.spell_or_mana(&v, &legal), None);
    v.battlefield[0].keywords = 0;
    v.battlefield[0].mana_value = 0;
    assert_eq!(agent.number(&v, 0, 6, &context), 0);
    assert_eq!(
        agent.spell_or_mana(&v, &legal),
        Some(PlayerAction::CastSpell { card: obj(1) })
    );
    v.battlefield[0].mana_value = 8;
    assert_eq!(agent.spell_or_mana(&v, &legal), None);
}

/// Six-seat selfplay seed 891015 repeatedly announced Stir the Grave with
/// X=1 although every creature in its graveyard cost more. The engine rolls
/// back such a cast, so the same offer is not evidence that it can finish.
#[test]
fn stir_the_grave_waits_until_a_visible_target_fits_the_payable_x() {
    let mut v = view(0, &[20, 20], vec![]);
    v.phase = baylee_view::Phase::FirstMain;
    v.hand = vec![hand_card(1, "Stir the Grave")];
    let mut creature = permanent(obj(2), v.seat, 2);
    creature.mana_value = 2;
    v.graveyards = vec![vec![creature], vec![]];
    v.seats[0].mana_pool.black = 2;
    let legal = baylee_engine::choice::LegalActions {
        castable: vec![obj(1)],
        can_pass: true,
        ..Default::default()
    };
    let agent = HeuristicAgent::new(AIProfile::SHARP);
    assert_eq!(agent.spell_or_mana(&v, &legal), None);
    v.graveyards[0][0].mana_value = 1;
    assert_eq!(
        agent.spell_or_mana(&v, &legal),
        Some(PlayerAction::CastSpell { card: obj(1) })
    );
    // The other player's graveyard cannot rescue an own-graveyard spell.
    v.graveyards[0][0].owner = PlayerId::new(1);
    v.graveyards[0][0].controller = PlayerId::new(1);
    assert_eq!(agent.spell_or_mana(&v, &legal), None);
}
