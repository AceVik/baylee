//! Exact population of target checks that require information outside the view.

use super::*;

/// The first leaf of a filter tree that [`matches`] cannot read, if any.
///
/// Structural rather than object-fed, because [`Filter::And`] stops at the
/// first part that does not match. That is sound where it lives — a false
/// part makes the whole false whatever the rest would have said — and
/// wrong for a census, which would count the parts it skipped as read. So
/// the composites are walked here and *which leaves are unreadable* stays
/// [`matches`]'s own answer, in one place, rather than a second list that
/// could drift away from it.
pub(super) fn unreadable_leaf<'f>(
    view: &PlayerView,
    probe: &PublicObject,
    filter: &'f Filter,
) -> Option<&'f Filter> {
    match filter {
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().find_map(|p| unreadable_leaf(view, probe, p))
        }
        Filter::Not(f) | Filter::AttachedTo(f) => unreadable_leaf(view, probe, f),
        leaf => matches(view, probe, leaf).is_none().then_some(leaf),
    }
}

/// The structural census must distinguish readable and unknown leaves in
/// both directions, even when another predicate short-circuits first. An
/// injection that only went red would show an assertion runs, not that it
/// discriminates.
///
/// The unreadable one is deliberately hidden *behind a leaf that does not
/// match*, which is the case an object-fed census misses. The third
/// assertion is why this test is not paranoia: it shows [`matches`]
/// answering that same filter `Some(false)` without complaint, because
/// [`Filter::And`] returns at the first false part and never reaches the
/// X-dependent leaf. The structural census must find it even though
/// the probe already failed the first leaf.
#[test]
fn the_census_tells_a_filter_it_can_read_from_one_it_cannot() {
    // Both first in the body, where clippy wants an item, and where they
    // read as the two cases this test is about.
    static READABLE: Filter =
        Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::ControlledByYou]);

    static BLIND: Filter = Filter::And(&[
        // False for the probe, so a short-circuiting reader stops here
        // and never asks about the unannounced X behind it.
        Filter::HasType(TypeSet::LAND),
        Filter::CmcExactlyX,
    ]);
    static RELATIVE_PAIR: Filter = Filter::And(&[
        Filter::HasType(TypeSet::LAND),
        Filter::SharesCreatureTypeWithOtherTarget,
    ]);

    let probe = printed(1, 0, "Grizzly Bears", 0);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![probe.clone()])
        .build();

    assert!(unreadable_leaf(&view, &probe, &READABLE).is_none());

    assert!(matches!(
        unreadable_leaf(&view, &probe, &BLIND),
        Some(Filter::CmcExactlyX)
    ));

    // The hazard is real and not hypothetical: asking `matches` alone
    // reads the same filter as a clean `false`.
    assert_eq!(matches(&view, &probe, &BLIND), Some(false));
    assert_eq!(
        matches(&view, &probe, &Filter::SharesCreatureTypeWithOtherTarget),
        None
    );
    assert!(matches!(
        unreadable_leaf(&view, &probe, &RELATIVE_PAIR),
        Some(Filter::SharesCreatureTypeWithOtherTarget)
    ));
}

/// How much of the pool [`provably_targetless`] can actually reach.
///
/// This replaces three frozen numbers that used to sit in the module
/// header and in `docs/client.md`. They were measured once and a card
/// batch moved the pool under them the same week, which is the argument
/// for measuring here instead: the population is what the claim is about,
/// so the test re-counts it and the prose names the buckets.
///
/// The unit is a **spell ability**, not a card: one card may carry two,
/// and `blind` below is deduplicated for the report alone and never
/// counted, because three numbers in two units look exactly like three
/// numbers in one.
/// **Both numbers, because a fix that erases what it fixed leaves its own
/// size invisible.** The `CardInGraveyard` arm landed in the same commit
/// as this sweep and took the blind bucket from **2 to 0**: Reanimate and
/// Sevinne's Reclamation, the two spells in this pool that target a card
/// in a graveyard and require at least one. Entreat the Dead is not among
/// them and that is correct — it asks for `x_targets`, so `min` is 0 and
/// "up to X" is cast with none.
///
/// Measured 20.09.2026: 218 spell abilities require a target, 31 can
/// never be targetless, provable 185 -> 187, blind 2 -> 0.
#[test]
fn the_pools_targets_are_a_population_this_client_can_read() {
    // Exact membership replaces a growing budget. These predicates require
    // history, unannounced X, or stack-source facts absent from PlayerView.
    // They deliberately remain unknown; their printed cards are unchanged.
    const EXPECTED_BLIND: [&str; 12] = [
        "Bind",
        "Detonate",
        "Dominate",
        "Fatal Blow",
        "Fire and Brimstone",
        "Glyph of Delusion",
        "Interdict",
        "Kaervek's Purge",
        "Rust",
        "Spell Blast",
        "Stir the Grave",
        "Venomous Vines",
    ];

    // Two seats, because `Filter::ControlledByOpponent` refuses to answer
    // at any other table size. And one well-formed permanent, because on
    // an empty battlefield `count` never calls `matches` at all and would
    // report every filter in the pool as readable — a census that
    // inspected nothing, which is the failure this test exists to notice.
    let probe = printed(1, 0, "Grizzly Bears", 0);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![probe.clone()])
        .build();

    let (mut always, mut provable, mut unread) = (0usize, 0usize, 0usize);
    let mut blind: Vec<String> = Vec::new();
    let mut blind_names = Vec::new();

    for def in baylee_cards::all() {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                let AbilityDef::Spell {
                    targets: Some(req), ..
                } = ability
                else {
                    continue;
                };
                // "Up to one target" is cast with none (CR 601.2c), the
                // same reason `provably_targetless` skips it.
                if req.min == 0 {
                    continue;
                }
                // Exhaustive on purpose, with no wildcard: a new
                // `TargetSpec` variant must be a compile error here and
                // not drift silently into the bucket below, which claims
                // a spell can never be targetless.
                let filter = match &req.spec {
                    // A player is always there to point at, and the other
                    // two name something the spell already holds, so none
                    // of these can ever be targetless. The proof is
                    // refused for them by construction, not by a gap.
                    TargetSpec::Player(_)
                    | TargetSpec::AnyPlayer
                    | TargetSpec::AnyOpponent
                    | TargetSpec::OpponentOrObject(_)
                    | TargetSpec::PlayerOrObject(_)
                    | TargetSpec::AnyTarget
                    | TargetSpec::ThisObject
                    | TargetSpec::EventObject => {
                        always += 1;
                        continue;
                    }
                    TargetSpec::Object(f)
                    | TargetSpec::ObjectOfEachOpponent(f)
                    | TargetSpec::ObjectOfFirstTargetsPlayer(f)
                    | TargetSpec::ObjectControlledBy(f, _)
                    | TargetSpec::ObjectOfEventPlayer(f)
                    | TargetSpec::Spell(f)
                    | TargetSpec::StackOrBattlefield(f)
                    | TargetSpec::AbilityOnStack(f)
                    | TargetSpec::SpellOrAbility(f)
                    | TargetSpec::CardInGraveyard(f, _)
                    | TargetSpec::CardInFirstTargetsGraveyard(f)
                    | TargetSpec::CardInPlayersGraveyard(f, _)
                    | TargetSpec::CardInGraveyardBelowEvent(f, _)
                    | TargetSpec::CardInGraveyardBelowValue(f, _, _) => *f,
                    TargetSpec::PlayerWhoAttackedThisTurn => &Filter::Any,
                };
                let name = def.faces[0].name;
                if legal_targets(&view, &req.spec).is_none() {
                    unread += 1;
                    blind_names.push(name);
                    blind.push(format!("{name} — spec {:?} has no arm", req.spec));
                } else if let Some(leaf) = unreadable_leaf(&view, &probe, filter) {
                    unread += 1;
                    blind_names.push(name);
                    blind.push(format!("{name} — filter {leaf:?}"));
                } else {
                    provable += 1;
                }
            }
        }
    }
    blind.sort();
    blind.dedup();
    let total = always + provable + unread;

    // The floor, for the reason the pool sweeps in `baylee-cards` and
    // `cross-read` carry one: a sweep that inspected nothing reports the
    // same clean result as one that found nothing wrong.
    assert!(
        total >= 200,
        "the sweep read only {total} spell abilities requiring a target, \
         which is too few to be reading this pool at all"
    );

    blind_names.sort_unstable();
    assert_eq!(
        blind_names,
        EXPECTED_BLIND,
        "the exact unread target population changed:\n  {}",
        blind.join("\n  ")
    );
    assert_eq!(
        unread,
        EXPECTED_BLIND.len(),
        "count abilities before deduplication"
    );
}

#[test]
fn history_and_unannounced_x_do_not_prove_an_existing_creature_illegal() {
    let mut creature = printed(3, 1, "Grizzly Bears", 3);
    creature.damage = 0;
    creature.mana_value = 2;
    let view = ViewBuilder::new(2)
        .with_battlefield(1, vec![creature])
        .build();
    for name in ["Fatal Blow", "Kaervek's Purge"] {
        assert!(
            !provably_targetless(&view, in_hand(name)),
            "{name}: zero marked damage is not a history, and X is not announced"
        );
        let empty = ViewBuilder::new(2).build();
        assert!(
            provably_targetless(&empty, in_hand(name)),
            "{name}: no battlefield objects still proves that no creature can be targeted"
        );
        let mut land = printed(4, 0, "Forest", 4);
        land.types = TypeSet::LAND;
        let noncreature = ViewBuilder::new(2).with_battlefield(0, vec![land]).build();
        assert!(provably_targetless(&noncreature, in_hand(name)));
    }
}

#[test]
fn an_empty_current_combat_does_not_prove_no_player_attacked_this_turn() {
    let view = ViewBuilder::new(2).build();
    assert!(view.combat.attackers.is_empty());
    assert!(!provably_targetless(&view, in_hand("Fire and Brimstone")));
}
