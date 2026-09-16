//! What `LegalActions` makes reachable in a priority window, and which `PlayerAction` each thing under the pointer becomes: a land, a spell, an activated ability, a mana ability, and the pass that `confirm` means in this mode. A card is playable only because the engine listed it, a card listed as both a land and a spell is not a one-click land, and an ability offered at index 0 outranks a granted mana ability on the same object - the fetchland that tapped for mana instead of searching. The words drawn over those buttons are in `prompts`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn priority_confirms_as_a_pass_and_exposes_the_legal_actions() {
    let legal = LegalActions {
        can_pass: true,
        lands: vec![obj(1)],
        castable: vec![obj(2)],
        mana_abilities: vec![obj(3)],
        abilities: vec![(obj(4), 1)],
        suspendable: vec![],
    };
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(legal),
    });
    assert_eq!(i.confirm(), Some(PlayerAction::PassPriority));
    assert!(i.legal_actions().is_some());
}

#[test]
fn playing_a_card_maps_to_the_right_action_and_refuses_illegal_ones() {
    let legal = LegalActions {
        can_pass: true,
        lands: vec![obj(1)],
        castable: vec![obj(2)],
        mana_abilities: vec![],
        abilities: vec![],
        suspendable: vec![],
    };
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(legal),
    });
    assert_eq!(
        i.play_card(obj(1)),
        Some(PlayerAction::PlayLand { card: obj(1) })
    );
    assert_eq!(
        i.play_card(obj(2)),
        Some(PlayerAction::CastSpell { card: obj(2) })
    );
    // A card the engine did not list is not playable, whatever the board
    // looks like.
    assert_eq!(i.play_card(obj(3)), None);
}

/// The bug that made a fetchland tap for mana instead of searching.
///
/// Flooded Strand is authored correctly — one `activated!` at index 0 with
/// `SearchLibrary`, and no mana ability on it. Chromatic Lantern grants
/// every land a mana ability, which puts the Strand in `mana_abilities`,
/// and the old guard fired on the index's numeric value: index 0 plus a
/// name in `mana_abilities` meant "mana ability", whatever the engine had
/// actually offered at that index.
///
/// Nothing in `abilities.rs` covered this shape, because nothing there put
/// index 0 and a populated `mana_abilities` on the same object.
#[test]
fn an_offered_ability_at_index_zero_beats_a_granted_mana_ability() {
    let strand = obj(1);
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(LegalActions {
            can_pass: true,
            lands: vec![],
            castable: vec![],
            // Both, which is the whole situation: the grant names the
            // land, and its own printed ability is offered at index 0.
            mana_abilities: vec![strand],
            abilities: vec![(strand, 0)],
            suspendable: vec![],
        }),
    });
    assert_eq!(
        i.activate(strand, 0),
        Some(PlayerAction::ActivateAbility {
            source: strand,
            ability_index: 0,
        }),
        "the fetchland tapped for mana instead of searching"
    );
}

/// …and the shortcut still works when it is the only thing offered.
#[test]
fn index_zero_is_the_mana_shortcut_when_nothing_else_was_offered_there() {
    let forest = obj(1);
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(LegalActions {
            can_pass: true,
            lands: vec![],
            castable: vec![],
            mana_abilities: vec![forest],
            abilities: vec![],
            suspendable: vec![],
        }),
    });
    assert_eq!(
        i.activate(forest, 0),
        Some(PlayerAction::ActivateManaAbility { source: forest })
    );
}

#[test]
fn a_card_offered_as_both_a_land_and_a_spell_is_not_a_one_click_land() {
    let plains = obj(1);
    let mdfc = obj(2);
    let bolt = obj(3);
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(LegalActions {
            can_pass: true,
            lands: vec![plains, mdfc],
            castable: vec![mdfc, bolt],
            mana_abilities: vec![],
            abilities: vec![],
            suspendable: vec![],
        }),
    });
    assert!(i.plays_only_as_a_land(plains));
    // In both lists: `play_card` checks lands first, so a one-click here
    // would make the spell face unreachable by mouse.
    assert!(!i.plays_only_as_a_land(mdfc));
    assert!(!i.plays_only_as_a_land(bolt));
    // And a card the engine never listed is neither.
    assert!(!i.plays_only_as_a_land(obj(9)));
}

#[test]
fn activating_an_ability_requires_it_to_have_been_offered() {
    let legal = LegalActions {
        can_pass: true,
        lands: vec![],
        castable: vec![],
        mana_abilities: vec![obj(5)],
        abilities: vec![(obj(6), 2)],
        suspendable: vec![],
    };
    let i = interaction(Pending::Priority {
        player: me(),
        legal: Box::new(legal),
    });
    assert_eq!(
        i.activate(obj(5), 0),
        Some(PlayerAction::ActivateManaAbility { source: obj(5) })
    );
    assert_eq!(
        i.activate(obj(6), 2),
        Some(PlayerAction::ActivateAbility {
            source: obj(6),
            ability_index: 2
        })
    );
    assert_eq!(i.activate(obj(6), 3), None, "wrong ability index");
    assert_eq!(i.activate(obj(7), 0), None, "not a listed source");
}
