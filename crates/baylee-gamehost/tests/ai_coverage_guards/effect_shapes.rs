#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// The pool really does hide effects behind "unless you pay", and this is
/// the population that says so.
///
/// [`Effect::PlayerMayPayOr`] and [`Effect::PlayerMayPayCostOr`] carry a
/// single `&'static Effect` rather than a list, and every hand-rolled walker
/// in this workspace descended into the lists and stopped there. That is a
/// blind spot no reader reported, because an effect nobody walks looks
/// exactly like an effect that is not there — so this test asks the pool
/// what is behind the clause instead of asking a reader whether it looked.
///
/// Measured on 19.09.2026: **35 effects on 35 cards**, being 29
/// `SacrificeSelf` (every Karoo land and the upkeep creatures), 3
/// `DrawCards` (Esper Sentinel, Mystic Remora, Rhystic Study), 2
/// `CounterTargetSpell` (Flusterstorm, Malevolent Hermit) and 1
/// `CreateToken` (Smothering Tithe). Floors and not the numbers: a card
/// joining or leaving is not news, and a walk that stopped descending is.
///
/// The breakdown is the test's own, taken by tallying `behind` rather than
/// by grepping card files: the first reading of it said 29 and then wrote
/// 28, and 28 + 3 + 2 + 1 is 34 against a total of 35. A census whose parts
/// do not sum to its whole has a card in it nobody looked at.
#[test]
fn the_pool_hides_effects_behind_a_price() {
    let (all, seen) = pool_effects();
    let behind: Vec<&'static Effect> = all
        .iter()
        .flat_map(|effect| match effect {
            Effect::PlayerMayPayOr { .. }
            | Effect::PlayerMayPayManaOr { .. }
            | Effect::PlayerMayPayCostOr { .. } => effect.branches().0,
            _ => &[][..],
        })
        .collect();
    let sacrifices = behind
        .iter()
        .filter(|effect| matches!(effect, Effect::SacrificeSelf))
        .count();

    assert!(
        seen > 2_000,
        "the effect walk visited {seen} effects, which is too few to have \
         read this pool: the nesting shapes it follows have gone stale"
    );
    assert!(
        behind.len() >= 20,
        "the walk found {} effect(s) behind a price where 35 were measured. \
         Either the pool stopped printing the clause or `Effect::branches` \
         stopped descending into it — and the second is the one that makes \
         every reader in this workspace quietly short.",
        behind.len()
    );
    assert!(
        sacrifices >= 15,
        "{sacrifices} of the effects behind a price are `SacrificeSelf` \
         where 28 were measured. The Karoo lands are what that clause is \
         mostly made of here, so this reaching nought means the walk found \
         the wrapper and not what is inside it"
    );
}

/// A face states at most one *list* of modes.
///
/// The question the agent answers names no ability: a modal trigger asks
/// through the permanent and a mode number, and which of its abilities is on
/// the stack lives in the engine's trigger queue. So a face with two modal
/// abilities over two different mode lists is two lists and one number, and
/// `baylee_ai::filter::modal_modes` refuses it and falls back to the printed
/// order rather than picking a list.
///
/// What the pool prints is the harmless half of that, and it is why the rule
/// is about lists and not abilities: Derevi, Empyrial Tactician is one
/// printed sentence with two trigger conditions ("when this enters **and**
/// whenever a creature you control deals combat damage to a player"), written
/// as two `modal_triggered!` over the same `TAP_OR_UNTAP` modes. Naming that
/// shared list is unambiguous however the trigger arrived.
///
/// It asks through `abilities_for_face`, which is the lookup the agent uses
/// and not the union `abilities` builds: face 0 falls back to the card-level
/// list only when it states none of its own, and a probe reading both at once
/// would find two lists where the card has one.
///
/// Two injected findings, because the assertion is an emptiness. `modal`
/// proves the walk reaches modal abilities at all; `shared` proves it reaches
/// the two-abilities-one-list case, which is the case the assertion is a
/// statement about and would otherwise be indistinguishable from a pool where
/// every face has exactly one.
#[test]
fn no_pool_face_states_two_mode_lists() {
    let mut faces = 0;
    let mut modal = 0;
    let mut split: Vec<&'static str> = Vec::new();
    for def in baylee_cards::all() {
        for (index, face) in def.faces.iter().enumerate() {
            faces += 1;
            let mut lists: Vec<&'static [baylee_cards::dsl::SpellMode]> = Vec::new();
            for ability in def.abilities_for_face(index) {
                let (AbilityDef::ModalSpell { modes, .. }
                | AbilityDef::ModalTriggered { modes, .. }
                | AbilityDef::ModalActivated { modes, .. }) = ability
                else {
                    continue;
                };
                if !lists.iter().any(|seen| std::ptr::eq(*seen, *modes)) {
                    lists.push(modes);
                }
            }
            if !lists.is_empty() {
                modal += 1;
            }
            if lists.len() > 1 {
                split.push(face.name);
            }
        }
    }

    // 2836 faces over 2716 cards on 19.09.2026, 10 of them modal and one —
    // Derevi — then stating one list twice. The floor is the population and not
    // the number: a card added or removed is not news, a walk that stopped
    // descending into faces is.
    //
    // The 10 is also why the walk is compiled rather than textual, and it is
    // wrong in both directions. Grepping the card files for `ModalSpell` and
    // `ModalTriggered` finds **four**, because seven more are written with
    // the `modal_triggered!` macro and never spell a variant at all — and
    // grepping for the macro as well finds **eleven**, because Marionette
    // Apprentice names it in a `// NOT SUPPORTED:` comment about the
    // fabricate clause it does *not* have.
    assert!(
        faces > 2_500,
        "the walk visited {faces} faces, which is too few to have read this \
         pool: it has stopped descending into the cards it is given"
    );
    assert!(
        modal > 0,
        "the walk found no modal ability in the whole pool, so it is not \
         reaching `ModalSpell` at all and the emptiness below is empty"
    );
    // Derevi now makes its optional tap/untap choice during resolution.
    // It no longer supplies a shared modal list; keep the actual invariant
    // (at most one distinct list), plus the nonempty modal/face census above.
    assert!(
        split.is_empty(),
        "{split:?} state two different mode lists on one face. \
         `baylee_ai::filter::modal_modes` refuses such a face and the agent \
         takes the printed order there: give the mode list an ability handle, \
         or record here why the printed order is the right answer for it."
    );
}

/// #76: the land the agent would miss if it read only the face that is up.
///
/// A modal double-faced card is filed under its front, and the front is the
/// spell — so `HandObject::types` says sorcery and the land is on the other
/// side. The engine does not read it that way: `compute_legal` offers the
/// land drop for a modal card's land back (CR 712.12) and not for a
/// transforming one's (CR 712.8a, #152). `policy::plays_as_land` asks the
/// same function, and this is the population it is worth having: a floor
/// rather than the number, because a card joining the pool does not make
/// the rule less true, and nought would mean the rule is being kept for
/// nothing.
#[test]
fn the_pool_prints_lands_on_a_back_face() {
    let hidden = baylee_cards::all()
        .filter(|def| {
            def.faces
                .iter()
                .skip(1)
                .any(|f| f.types.contains(TypeSet::LAND) && f.castable_from_hand)
                && !def.faces[0].types.contains(TypeSet::LAND)
        })
        .count();
    let front = baylee_cards::all()
        .filter(|def| def.faces[0].types.contains(TypeSet::LAND))
        .count();
    assert!(
        hidden >= 45,
        "{hidden} modal card(s) print a land on a face that is not the front, \
         where 50 were measured on 24.09.2026 (82 with the transforming ones). \
         Below this the rule `plays_as_land` states is being kept for a \
         handful of cards and is worth re-reading."
    );
    assert!(
        front > hidden,
        "{front} card(s) print a land on the front against {hidden} behind \
         one, which is the wrong way round for this pool: the front-face \
         reading would then be the special case, not the rule."
    );
}
