import re

with open("crates/baylee-client/src/castmodes.rs", "r") as f:
    content = f.read()

replacement = """    let modal_only = baylee_engine::casting::modes_are_the_only_way(def, 0);

    // CR 202.1b: a face with no printed cost has no printed way to be cast,
    // which is the rule `casting::has_a_printed_cost` states engine-side and
    // the one that keeps a suspend-only card off this list.
    if !modal_only
        && face.mana_cost.symbols().next().is_some()
        && (face.kicked_targets.is_none()
            || !crate::targeting::ordinary_targetless(view, hand.card))
    {
        offer(CastModeKind::Normal, face.mana_cost);
    }
    
    if let Some(baylee_cards_dsl::AbilityDef::ModalSpell { modes, choose }) = def
        .abilities_for_face(0)
        .iter()
        .find(|a| matches!(a, baylee_cards_dsl::AbilityDef::ModalSpell { .. }))
    {
        if *choose == baylee_cards_dsl::ModeCount::ONE {
            for (i, mode) in modes.iter().enumerate() {
                let cost = mode.cost_override.unwrap_or(face.mana_cost);
                offer(CastModeKind::Mode(i), cost);
            }
        }
    }"""

original = """    // CR 202.1b: a face with no printed cost has no printed way to be cast,
    // which is the rule `casting::has_a_printed_cost` states engine-side and
    // the one that keeps a suspend-only card off this list.
    if face.mana_cost.symbols().next().is_some()
        && (face.kicked_targets.is_none()
            || !crate::targeting::ordinary_targetless(view, hand.card))
    {
        offer(CastModeKind::Normal, face.mana_cost);
    }"""

content = content.replace(original, replacement)

with open("crates/baylee-client/src/castmodes.rs", "w") as f:
    f.write(content)
