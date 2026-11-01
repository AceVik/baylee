#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// Only the named cards grant mana with a price beyond a tap.
///
/// `simple_mana` reads output, not whether a non-mana price is free.
/// `PublicObject::granted_mana` and `GrantSource::mana` therefore require
/// `tap_only`: neither projection has a field for life or sacrifice costs.
/// Each name below owes real-board evidence of its paid activation and a
/// documented AI limitation. The shared projection boundary is covered by
/// `view::tests::grants::granted_mana_refuses_a_priced_grant`.
///
/// The population bound is the load-bearing half. A grant hides in **two**
/// shapes — `AbilityDef::Static`, which `ability_effects` deliberately
/// returns nothing for, and `Effect::CreateContinuousEffect` inside an
/// ordinary effect list — and a walk that found only one of them would
/// report a clean zero over a third of the sites. Chromatic Lantern and
/// Great Divide Guide are statics; Urza's Saga writes both of its through
/// chapters.
#[test]
fn no_granted_mana_ability_charges_more_than_the_tap() {
    let mut sites = 0;
    let mut mana_sites = 0;
    let mut priced: Vec<&str> = Vec::new();
    for def in baylee_cards::all() {
        let mut modifiers: Vec<&'static Modifier> = Vec::new();
        for ability in abilities(def) {
            if let AbilityDef::Static(statics) = ability {
                modifiers.push(&statics.modifier);
            }
        }
        for effects in abilities(def).flat_map(ability_effects) {
            let mut seen = 0;
            Effect::walk(effects, &mut seen, &mut |effect| {
                if let Effect::CreateContinuousEffect { modifier, .. } = effect {
                    modifiers.push(modifier);
                }
            });
        }
        for modifier in modifiers {
            let Modifier::GrantActivated {
                cost, mana_ability, ..
            } = modifier
            else {
                continue;
            };
            sites += 1;
            if !*mana_ability {
                continue;
            }
            mana_sites += 1;
            if !baylee_cards_dsl::tap_only(cost) {
                priced.push(def.faces[0].name);
            }
        }
    }
    // Four on 20.09.2026 — Chromatic Lantern, Great Divide Guide and both of
    // Urza's Saga's — of which three grant mana. Bounds and not equalities,
    // because a card added is not news and a walk that stopped descending is.
    assert!(
        sites >= 4,
        "the walk found {sites} `GrantActivated` sites and there were four: \
         it is reaching only one of the two shapes a grant is written in"
    );
    assert!(
        mana_sites >= 3,
        "the walk found {mana_sites} granted *mana* abilities and there were \
         three, so the assertion below is over a list too short to fail"
    );
    // `view::tests::grants::granted_mana_refuses_a_priced_grant` builds
    // the Basal Sliver and Forgotten Monument boards and checks both mana
    // fields while the engine still offers the paid activation. Basal Sliver
    // additionally sacrifices the recipient for exactly {B}{B}.
    // `baylee-engine::engine::card_tests::instants::mv_1::rain_of_filth`
    // exercises Rain of Filth on a real board: an existing controlled land
    // is sacrificed for exactly {B}, without using the stack; later and
    // opposing lands receive no grant, and it expires at end of turn.
    // This card test proves the paid activation, not a gamehost projection.
    // None of these grants can be planned by the AI today;
    // docs/ai-coverage-todo.md records the price-aware planning debt.
    // Sort names so this is membership, independent of generated index order.
    // Keep exact equality: additions and removals must both be reviewed.
    priced.sort_unstable();
    assert_eq!(
        priced,
        ["Basal Sliver", "Forgotten Monument", "Rain of Filth"],
        "the pool's priced granted mana abilities have changed. Each one is \
         dropped from `PublicObject::granted_mana` rather than reported free \
         — see `baylee_cards_dsl::tap_only` — and each owes a test with a \
         board under it, because this walk cannot see a projection"
    );
}
