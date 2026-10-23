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
fn unreadable_leaf<'f>(
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

/// The ceiling below is a bound over a measured zero, so no card in this
/// pool fires it. These two do, and in **both** directions: an injection
/// that only went red would show the assert runs, not that it
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
    // The ceiling is a **budget on a known gap, not a target**. Five, so
    // one new blind spell costs nobody a red gate and a drift to six
    // trips it — the point is to notice the blind spot spreading, not to
    // police the pool. Six since the wave 2 partials (13.10.2026): Glyph
    // of Delusion targets a Wall that blocked this turn, and this turn's
    // blocks are not in the view (`Filter::BlockedThisTurnBy`).
    const CEILING: usize = 6;

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
                    blind.push(format!("{name} — spec {:?} has no arm", req.spec));
                } else if let Some(leaf) = unreadable_leaf(&view, &probe, filter) {
                    unread += 1;
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

    assert!(
        unread <= CEILING,
        "{unread} of {total} spell abilities requiring a target are ones this \
         client cannot reason about, over a budget of {CEILING}.\n\n\
         This is not a defect in these cards. An unreadable filter or an \
         unhandled spec makes the client *offer* the spell, which is the safe \
         direction; the blind spot has simply grown. The repair is one arm in \
         crates/baylee-client/src/targeting.rs — in `legal_targets` if the \
         line below says `spec`, in `matches` if it says `filter` — and never \
         a change to the card:\n  {}",
        blind.join("\n  ")
    );
}
