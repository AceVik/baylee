//! Costs: their order, what they spend and the numbers they announce.

use super::*;

/// The cost this lint exists for, written the one way round that breaks.
///
/// Zero cards in the pool are built this way, so the sweep below finds
/// nothing and would find nothing if it read no costs at all. This is
/// the half that makes the other half mean something: the sacrifice
/// first, the counter after it, exactly as a transcription of "remove
/// two pressure counters and sacrifice this" would come out if the two
/// clauses were read in the order a careless reader meets them.
#[test]
fn the_order_lint_catches_a_cost_that_spends_a_permanent_it_already_gave_up() {
    const BROKEN: &[CostPart] = &[
        CostPart::SacrificeSelf,
        CostPart::RemoveCounterSelf {
            kind: crate::dsl::CounterKind::P1P1,
            n: 2,
        },
    ];
    const PRINTED: &[CostPart] = &[
        CostPart::RemoveCounterSelf {
            kind: crate::dsl::CounterKind::P1P1,
            n: 2,
        },
        CostPart::SacrificeSelf,
    ];

    assert_eq!(
        cost_order_fault(BROKEN),
        Some((
            CostPart::SacrificeSelf,
            CostPart::RemoveCounterSelf {
                kind: crate::dsl::CounterKind::P1P1,
                n: 2,
            },
        )),
        "a counter spent after the permanent is in the graveyard",
    );
    assert_eq!(
        cost_order_fault(PRINTED),
        None,
        "and the printed order is fine, which is the whole point",
    );
    assert_eq!(
        cost_order_fault(&[CostPart::SacrificeSelf, CostPart::PayLife(1)]),
        None,
        "a part that never looks the source up does not care that it is gone",
    );
    assert_eq!(
        cost_order_fault(&[CostPart::TapSelf, CostPart::SacrificeSelf]),
        None,
        "and a fetchland is not a finding",
    );
}

/// No cost in the pool pays a part after the source it names is gone.
#[test]
fn no_cost_asks_for_a_permanent_it_has_already_spent() {
    let mut wrong = Vec::new();
    let mut read = 0usize;
    let mut check = |who: &str, ability: &AbilityDef| {
        for parts in cost_lists(ability) {
            read += 1;
            if let Some((mover, then)) = cost_order_fault(parts) {
                wrong.push(format!("{who} — {mover:?} and then {then:?}"));
            }
        }
    };
    for def in crate::all() {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                check(def.name(), ability);
            }
        }
    }
    for token in crate::tokens::ALL {
        for ability in token.abilities {
            check(token.name, ability);
        }
    }
    // An alternative cost is printed on a face and reached from nowhere
    // else, and `pay_cost` walks its parts in printed order like any
    // other. Walked after the abilities because `check` borrows the
    // counters until its last call.
    for def in crate::all() {
        for parts in face_cost_lists(def) {
            read += 1;
            if let Some((mover, then)) = cost_order_fault(parts) {
                wrong.push(format!(
                    "{} (alternative cost) — {mover:?} and then {then:?}",
                    def.name()
                ));
            }
        }
    }
    // The floor, for the reason `cross-read` carries one: a sweep that
    // read nothing reports the same "no offenders" as one that read the
    // pool. Measured at 725 cost lists on 2026-09-16 and at 2022 on
    // 2026-09-21 — the pool grew, and eleven of the new ones are the
    // alternative costs this sweep did not open until now.
    //
    // Raised with the measurement rather than left where it was: a floor
    // of 600 against a pool of 2022 would pass a reader that had gone
    // blind on two doors out of three, which is the failure it exists to
    // catch. The pool only grows, so a floor under the count cannot go
    // red on its own.
    assert!(
        read >= 1800,
        "read {read} activation cost lists out of the pool, which is not the pool"
    );
    assert!(
        wrong.is_empty(),
        "{} cost(s) pay a part after the source is gone — `pay_cost` walks \
         them in printed order, so the second one is asked of an object \
         that has left the battlefield.\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// Crew is alone in its cost (`CostPart::Crew`). Its one question is
/// answered with any number of creatures and `pay_cost` taps every
/// answer left, so a part printed beside it would be paid with a
/// creature named for the crew, or never asked at all. `crew!` writes it
/// alone; this holds a cost written by hand to the same.
#[test]
fn crew_is_alone_in_its_cost() {
    let mut wrong = Vec::new();
    let mut crews = 0usize;
    let mut check = |who: &str, ability: &AbilityDef| {
        for cost in costs(ability) {
            if cost.parts.iter().any(|p| matches!(p, CostPart::Crew(_))) {
                crews += 1;
                if cost.parts.len() != 1 || cost.mana != baylee_cards_dsl::ManaCost::ZERO {
                    wrong.push(format!("{who} — {cost:?}"));
                }
            }
        }
    };
    for def in crate::all() {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                check(def.name(), ability);
            }
        }
    }
    for token in crate::tokens::ALL {
        for ability in token.abilities {
            check(token.name, ability);
        }
    }
    assert!(
        crews >= 1,
        "read {crews} crew costs out of the pool; Unlicensed Hearse alone has one"
    );
    assert!(
        wrong.is_empty(),
        "a crew cost shares its cost with another part: {wrong:?}"
    );
}

/// The lint catches the pair it exists for, and nothing that merely
/// looks like it.
#[test]
fn the_announcement_lint_catches_a_cost_that_asks_before_it_shows() {
    use crate::dsl::ability::{ActivationLimit, ActivationTiming, ActivationZone};
    use crate::dsl::counters::STORAGE;

    static A_CREATURE: TargetSpec = TargetSpec::Object(&Filter::CREATURE);
    static ANNOUNCED: [CostPart; 2] = [
        CostPart::TapSelf,
        CostPart::RemoveCounterSelfX { kind: STORAGE },
    ];
    static COUNTED: [CostPart; 2] = [
        CostPart::TapSelf,
        CostPart::RemoveCounterSelf {
            kind: STORAGE,
            n: 1,
        },
    ];

    let storage = |parts: &'static [CostPart], target: Option<TargetSpec>| AbilityDef::Activated {
        cost: crate::dsl::Cost {
            mana: ManaCost::ZERO,
            parts,
        },
        effects: &[],
        targets: target.map(TargetReq::one),
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        cost_reduction: None,
    };

    assert_eq!(
        announced_number_beside_a_target(&storage(&ANNOUNCED, Some(A_CREATURE))),
        Some((STORAGE, A_CREATURE)),
        "a number announced with the activation, and a target chosen after it"
    );
    assert_eq!(
        announced_number_beside_a_target(&storage(&ANNOUNCED, None)),
        None,
        "the storage lands as they are printed: no target to be asked about"
    );
    assert_eq!(
        announced_number_beside_a_target(&storage(&COUNTED, Some(A_CREATURE))),
        None,
        "a fixed number announces nothing, so the order cannot be observed"
    );
}

/// Both branches of [`two_xs_in_one_cost`], because a sweep that finds
/// nothing says nothing until the shape it looks for has been seen to
/// fire.
#[test]
fn a_cost_with_two_xs_is_found_and_a_cost_with_one_is_not() {
    use crate::dsl::counters::STORAGE;

    static STORAGE_X: [CostPart; 1] = [CostPart::RemoveCounterSelfX { kind: STORAGE }];
    static TAP: [CostPart; 1] = [CostPart::TapSelf];

    let cost = |mana: &str, parts: &'static [CostPart]| Cost {
        mana: ManaCost::parse(mana),
        parts,
    };

    assert_eq!(
        two_xs_in_one_cost(&cost("{X}{G}", &STORAGE_X)),
        Some(CostPart::RemoveCounterSelfX { kind: STORAGE }),
        "one announced number would have to be two: a mana {{X}} bounded \
         by the pool and a counter X bounded by the permanent"
    );
    assert_eq!(
        two_xs_in_one_cost(&cost("{X}{G}", &TAP)),
        None,
        "Lair of the Hydra: a mana {{X}} and nothing else to announce"
    );
    assert_eq!(
        two_xs_in_one_cost(&cost("{1}", &STORAGE_X)),
        None,
        "the storage lands as printed: a counter X and a fixed price"
    );
}

/// **No cost in the pool announces one `X` in two places.**
///
/// `Engine::activation_x` is one field holding one answer, and the
/// engine asks for it at whichever shape it meets first — see
/// [`two_xs_in_one_cost`] for what a cost carrying both would do. This
/// is the pool-side half of that, and it walks the same three doors the
/// sweep above does: an ability's own cost, the cost of an ability a
/// continuous effect grants, and a face's alternative cost.
#[test]
fn no_cost_announces_two_different_xs() {
    let mut wrong = Vec::new();
    let mut variable = 0usize;

    let mut check = |who: &str, cost: &Cost| {
        if cost.mana.has_variable() {
            variable += 1;
        }
        if let Some(part) = two_xs_in_one_cost(cost) {
            wrong.push(format!("{who} — {} and {part:?}", cost.mana));
        }
    };
    for def in crate::all() {
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                for cost in costs(ability) {
                    check(def.name(), &cost);
                }
            }
        }
        for cost in face_costs(def) {
            check(def.name(), &cost);
        }
    }
    for token in crate::tokens::ALL {
        for ability in token.abilities {
            for cost in costs(ability) {
                check(token.name, &cost);
            }
        }
    }

    // Four activation costs in the pool print a mana `{X}` — Blast
    // Zone, Kessig Wolf Run, Lair of the Hydra and Treasure Vault,
    // counted on 2026-09-22. Without the floor this passes over a
    // reader that opened no door at all, which is how the sweep beside
    // it was found reading none.
    assert!(
        variable >= 4,
        "read {variable} costs with a mana {{X}} out of the pool, and              four cards print one"
    );
    assert!(
        wrong.is_empty(),
        "{} cost(s) announce one X in two places (CR 107.3i makes every \
         instance of X on an object one value, and CR 601.2b through \
         602.2b announces it once). The counter bound would be the only \
         one asked about, so the player could name a number the mana \
         cannot pay:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// **No card in the pool announces a number and then chooses a target.**
///
/// The other half of the sweep is the one that keeps it honest: every
/// `RemoveCounterSelfX` the pool prints has to be one this walk *saw*.
/// The `Debug` of a card prints every cost of every ability of every
/// face, including the ones a static ability grants, so the two counts
/// disagreeing means a door was added that this reader does not know
/// about — which is the failure mode a lint over a hand-written match
/// has, and it reports "no offenders" while it happens.
#[test]
fn no_cost_announces_a_number_on_an_ability_that_also_targets() {
    let mut wrong = Vec::new();
    let mut announced = 0usize;
    let mut printed = 0usize;
    let mut granted = 0usize;

    let mut check = |who: &str, ability: &AbilityDef| {
        for parts in cost_lists(ability) {
            announced += parts
                .iter()
                .filter(|part| matches!(part, CostPart::RemoveCounterSelfX { .. }))
                .count();
        }
        // A granted ability's cost is read above and can never be a
        // finding, because `Modifier::GrantActivated` has no target.
        if let AbilityDef::Static(_) = ability {
            granted += 1;
        }
        if let Some((kind, target)) = announced_number_beside_a_target(ability) {
            wrong.push(format!(
                "{who} — {kind:?} counters announced, then {target:?}"
            ));
        }
    };
    for def in crate::all() {
        printed += format!("{def:?}").matches("RemoveCounterSelfX").count();
        for face in 0..def.faces.len() {
            for ability in def.abilities_for_face(face) {
                check(def.name(), ability);
            }
        }
    }
    for token in crate::tokens::ALL {
        printed += format!("{token:?}").matches("RemoveCounterSelfX").count();
        for ability in token.abilities {
            check(token.name, ability);
        }
    }
    // The face-level door, walked after the abilities because `check`
    // borrows the counters until its last call.
    for def in crate::all() {
        announced += face_cost_lists(def)
            .iter()
            .flat_map(|parts| parts.iter())
            .filter(|part| matches!(part, CostPart::RemoveCounterSelfX { .. }))
            .count();
    }

    // The floor and the door, in that order. Sixteen cards in the pool
    // carry this cost, counted on 2026-09-21 — one fewer than the
    // seventeen `CostPart::RemoveCounterSelfX` counts, because that
    // number was measured over `//! Oracle:` headers and Crucible of the
    // Spirit Dragon prints "remove X storage counters" while still being
    // a stub. The two agree; they are counting a printing and a compiled
    // ability. A reader that has gone blind reports nought here rather
    // than passing with an empty `wrong`.
    assert!(
        announced >= 16,
        "read {announced} announced-number costs out of the pool, and \
         sixteen storage lands print one"
    );
    assert_eq!(
        announced, printed,
        "the pool prints {printed} `RemoveCounterSelfX` and this walk \
         reached {announced} of them, so a cost door exists that \
         `cost_lists` does not open ({granted} static abilities were \
         walked)"
    );
    assert!(
        wrong.is_empty(),
        "{} abilit(ies) announce a number with the activation \
         (CR 601.2b) and then choose a target (CR 601.2c). The engine \
         asks for the number first for both printed spellings of this \
         cost, which is legal only while no card does both — this one \
         asks the player how many counters to spend before showing them \
         what the ability can point at.\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

#[test]
fn the_loyalty_lint_catches_a_walker_with_no_starting_loyalty() {
    let walker = FaceDef {
        types: TypeSet::PLANESWALKER,
        ..FaceDef::DEFAULT
    };
    assert!(
        loyalty_fault(&walker),
        "a planeswalker that never says what it starts on slipped through"
    );
    let jace = FaceDef {
        types: TypeSet::PLANESWALKER,
        loyalty: Some(3),
        ..FaceDef::DEFAULT
    };
    assert!(
        !loyalty_fault(&jace),
        "a walker with loyalty is not a fault"
    );
}
