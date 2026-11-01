//! Pool enumeration, sweep execution and independently checked population bounds.

use super::*;
use std::ops::RangeInclusive;

/// Which faces of a card one sweep walks.
type Faces = fn(&CardDef) -> Vec<usize>;

fn walk(slice: &[&'static CardDef], faces: Faces) -> (Vec<String>, Tally) {
    let mut offenders = Vec::new();
    let mut tally = Tally::default();
    for def in slice {
        for index in faces(def) {
            walk_face(def, index, &mut offenders, &mut tally);
        }
    }
    (offenders, tally)
}

fn sweep(faces: Faces) -> (Vec<String>, Tally) {
    let cards: Vec<&'static CardDef> = baylee_cards::all()
        .filter(|d| !faces(d).is_empty())
        .collect();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let chunk = cards.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = cards
            .chunks(chunk)
            .map(|slice| crate::engine::testkit::spawn_named(scope, move || walk(slice, faces)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("mana sweep chunk"))
            .fold(
                (Vec::new(), Tally::default()),
                |(mut all, mut total), (found, one)| {
                    all.extend(found);
                    total.absorb(&one);
                    (all, total)
                },
            )
    })
}

/// How small each arm of one sweep may get before it has stopped measuring,
/// and, for what the text reader counts, how large.
///
/// The text reader's buckets are bounded on both sides, because a misread
/// marker moves lines from one bucket to another and a floor alone passes
/// that. The bounds will move with the pool; a trip is the moment to measure
/// again, not to widen.
struct Bounds {
    colors: usize,
    routes: usize,
    promises: RangeInclusive<usize>,
    opponents_lands: RangeInclusive<usize>,
    creature_promises: RangeInclusive<usize>,
    fully_read: usize,
    arrived_tapped: usize,
    costly_lines: RangeInclusive<usize>,
    triggered_lines: RangeInclusive<usize>,
    granted_lines: RangeInclusive<usize>,
    unparsed: RangeInclusive<usize>,
    self_granted: RangeInclusive<usize>,
    busy: usize,
    not_implemented: usize,
}

/// Runs one sweep, prints what it counted, and holds it to `bounds`.
fn hold(what: &str, faces: Faces, bounds: &Bounds) {
    let (offenders, tally) = sweep(faces);
    report(what, &tally);
    assert!(
        offenders.is_empty(),
        "{what}: {} disagreements between a card, its printed text and the engine: \
         {offenders:#?}",
        offenders.len()
    );
    check(what, &tally, bounds);
}

/// What one sweep counted, printed.
fn report(what: &str, tally: &Tally) {
    println!(
        "{what}: {} printed promises made ({} of them the opponent's lands', {} by creatures \
         that were not summoning sick, of {} such creatures), {} faces read whole, {} arrived \
         tapped and were untapped; {} faces of cards not implemented; printed mana lines: {} \
         costing more than the tap, {} triggered, {} granted to other objects, {} unparsed:\n{}",
        tally.promises + tally.opponents_lands,
        tally.opponents_lands,
        tally.creature_promises,
        tally.creatures,
        tally.fully_read,
        tally.entered_tapped,
        tally.not_implemented,
        tally.costly_lines,
        tally.triggered_lines,
        tally.granted_lines,
        tally.unparsed.len(),
        tally.unparsed.join("\n"),
    );
    println!(
        "{what}: {} colours proven over {} routes. Skipped: {} abilities behind a board \
         condition, {} that cost more than the tap, {} `simple_mana` will not read, {} faces \
         printing two basic land types, {} that give themselves one, {} offered the mana \
         their own grant gives them, {} still answering their own arrival or gone",
        tally.colors,
        tally.routes,
        tally.conditional,
        tally.costly,
        tally.unreadable,
        tally.dual_typed,
        tally.self_typed,
        tally.self_granted,
        tally.busy
    );
}

/// Holds one sweep's counts to its bounds.
fn check(what: &str, tally: &Tally, bounds: &Bounds) {
    let floors = [
        ("colours proven", tally.colors, bounds.colors),
        ("routes checked", tally.routes, bounds.routes),
        ("faces read whole", tally.fully_read, bounds.fully_read),
        (
            "faces that arrived tapped",
            tally.entered_tapped,
            bounds.arrived_tapped,
        ),
    ];
    for (arm, got, floor) in floors {
        assert!(
            got >= floor,
            "{what}: {got} {arm}, under the floor of {floor}"
        );
    }
    let ranges = [
        (
            "printed promises made, the opponent's lands' aside",
            tally.promises,
            &bounds.promises,
        ),
        (
            "promises of the opponent's lands made",
            tally.opponents_lands,
            &bounds.opponents_lands,
        ),
        (
            "promises made by creatures",
            tally.creature_promises,
            &bounds.creature_promises,
        ),
        (
            "mana lines costing more than the tap",
            tally.costly_lines,
            &bounds.costly_lines,
        ),
        (
            "triggered mana lines",
            tally.triggered_lines,
            &bounds.triggered_lines,
        ),
        (
            "mana lines granted to other objects",
            tally.granted_lines,
            &bounds.granted_lines,
        ),
        (
            "mana lines not parsed",
            tally.unparsed.len(),
            &bounds.unparsed,
        ),
        (
            "faces offered their own grant",
            tally.self_granted,
            &bounds.self_granted,
        ),
    ];
    for (arm, got, range) in ranges {
        assert!(
            range.contains(&got),
            "{what}: {got} {arm}, outside {range:?}"
        );
    }
    assert!(
        tally.busy <= bounds.busy,
        "{what}: {} faces could not be judged on their board, over the ceiling of {}",
        tally.busy,
        bounds.busy
    );
    assert!(
        tally.not_implemented <= bounds.not_implemented,
        "{what}: {} faces had their printed mana counted rather than judged, over the \
         ceiling of {}",
        tally.not_implemented,
        bounds.not_implemented
    );
}

/// Measured 2026-09-29 over the whole pool, with the printed text as the
/// oracle, taplands untapped by the harness and a Plains, an Island and a
/// Wastes across the table: **801 printed promises made, 1531 colours proven
/// over 1064 routes**, Exotic Orchard's promise of the opponent's lands, Dryad
/// Arbor judged as the one creature, 784 faces read whole, 588 that arrived
/// tapped. On 2026-09-10, when the card was its own oracle and taplands were
/// skipped, it was 146 colours over 132 routes.
///
/// The reader's buckets: 153 lines costing more than the tap, 2 triggered
/// (Crumbling Vestige, Branch of Vitu-Ghazi), 7 granted to other objects,
/// 183 not parsed (a rider sentence, a colour chosen as the land entered,
/// restricted mana, two colours at once, an amount a board counts), listed
/// in the output; 251 faces of cards not `Coverage::Implemented` whose
/// promises are counted rather than judged.
const LANDS: Bounds = Bounds {
    colors: 1400,
    routes: 950,
    // Havengul, Tecutlan, Barracks and Underdome add four promises to Thran Portal's 1013.
    // 1022 measured 2026-10-13 once main (250c4bece) and c41/b8-astra-review
    // (8703073ae) were merged into the partials lane, which counted at most
    // 1017 before. 1028 measured 2026-10-13 after wave 1 pair B's lands went
    // Implemented. 1029 after wave 2 pair D's Contested War Zone went
    // Implemented; 1032 after wave 3 pair E's Den of the Bugbear, Dalkovan
    // Encampment and Archway of Innovation; 1033 after wave 4's Dakmor
    // Salvage. 1034 once the full corpus (2026-10-14) added its generated
    // lands. 1044 measured 2026-10-15 after seven Partial -> Implemented
    // land promotions. Secret Tunnel, Arena of Glory, Fire Nation Palace,
    // Inkmoth Nexus, Restless Anchorage and Stardew Valley each contribute
    // one newly judged simple promise. Chocobo Camp's Bird rider remains
    // explicitly unparsed, not a seventh simple promise. Keep the original
    // six-promise slack budget above the current measured population.
    // 1055 non-opponent-dependent promises after the October card batches.
    promises: 700..=1061,
    opponents_lands: 1..=3,
    creature_promises: 1..=5,
    fully_read: 700,
    arrived_tapped: 500,
    costly_lines: 120..=200,
    triggered_lines: 1..=8,
    granted_lines: 4..=15,
    unparsed: 150..=250,
    self_granted: 0..=3,
    busy: 3,
    not_implemented: 300,
};

/// Measured 2026-09-29 over the whole pool on the same boards: **78 printed
/// promises made, 122 colours proven over 79 routes**, Fellwar Stone's
/// promise of the opponent's lands, 35 promises made by creatures among the
/// 90 creature faces judged — the count that says the CR 302.6 board is not a
/// vacuum — and 133 faces read whole.
///
/// The reader's buckets: 60 lines costing more than the tap (the Eggs, the
/// Signets, every "Sacrifice this creature: Add …"), 13 triggered (Priest of
/// Gix, Lotus Cobra, Su-Chi), 6 granted to other objects (Chromatic Lantern,
/// Enduring Vitality), 22 not parsed (the Talismans' damage rider, a chosen
/// colour, a commander's identity, an amount a board counts, Mox Opal's
/// condition); 2 faces offered the mana their own grant gives them (Enduring
/// Vitality, Great Divide Guide), 1 that could not stand on its board
/// (Faeburrow Elder, not implemented and 0/0 without its static), 2 faces of
/// cards not implemented.
const OTHER_PERMANENTS: Bounds = Bounds {
    colors: 100,
    routes: 65,
    promises: 60..=100,
    opponents_lands: 1..=3,
    creature_promises: 28..=50,
    fully_read: 110,
    arrived_tapped: 0,
    costly_lines: 50..=80,
    triggered_lines: 10..=20,
    granted_lines: 4..=10,
    unparsed: 15..=35,
    self_granted: 1..=4,
    busy: 3,
    not_implemented: 10,
};

/// CR 605.1: a mana ability is an activated ability that produces mana, does
/// not target, and does not use the stack — so the only place a player ever
/// reads its outcome is the pool.
#[test]
fn every_land_in_the_pool_makes_the_mana_its_own_card_promises() {
    hold("lands", land_faces, &LANDS);
}

/// The same claims over every artifact, creature and enchantment face that
/// prints a mana line or carries a mana ability, each creature on a board
/// where it has been under its controller's control since their turn began
/// (CR 302.6).
#[test]
fn every_other_permanent_in_the_pool_makes_the_mana_its_card_prints() {
    let (offenders, tally) = sweep(other_permanent_faces);
    report("other permanents", &tally);
    assert!(offenders.is_empty(), "{offenders:#?}");
    check_other_permanent_population(&tally);
    check("other permanents", &tally, &FULL_CORPUS_OTHER_PERMANENTS);
}

/// Rest-batch-01-retests measured this complete population after attaching
/// the mana Aura: 194 wholly readable faces, 91 costly lines, 14 granted
/// lines, and 6 self-granting faces. Pin the new buckets exactly instead of
/// giving them growth headroom. All other pre-expansion bounds remain.
///
/// The full corpus (2026-10-14, the rest of the generated cards) re-measured
/// it, after leaving out the mana Auras the attached fixture cannot place:
/// 366 wholly readable faces (146 Implemented, 220 Generated), 128 costly
/// lines, 41 granted, 7 self-granting; 211 promises (90 by creatures of 227)
/// and 55 unparsed lines, each listed in the output. Pinned exactly again.
///
/// Reviewed 2026-10-23 against the last pins (6db0d8a60): Benthic Explorers
/// lost its unsupported free tap-for-any-color ability (-1 route, -5 colors).
/// Lotus Blossom adds one wholly readable face and one costly line/ability;
/// Thran Turbine adds one wholly readable face and one triggered line;
/// Witch Engine adds one creature and one unparsed line (its targeted ability
/// correctly uses the stack). Holding Squandered Resources removes one costly
/// ability, balancing Lotus Blossom. No printed promise was lost: 212 plus
/// Fellwar Stone's opponent-land promise still make 213. The independent
/// census and the executed sweep both reach 379 wholly readable faces.
/// Urza's Destiny batch 2 adds Metalworker as a Partial 1/2 creature (its
/// hand-reveal mana ability is omitted), adding 1 non-fully-read eligible
/// face, 1 creature, and 1 unparsed mana line.
///
/// Mercadian Masques (`mmq`) Batch 1 adds `Black Market` (`Coverage::Implemented`),
/// adding 1 wholly readable face and 1 triggered mana line. Batch 3 adds
/// `Food Chain` (`Coverage::Implemented`), adding 1 wholly readable face, 1
/// costly mana line, and 1 costly mana ability. Batch 4 adds `Kyren Toy`
/// (`Coverage::Unimplemented`), adding 1 non-fully-read eligible face and 1
/// costly mana line. Nemesis (`nem`) Batch 2 adds `Mana Cache`
/// (`Coverage::Unimplemented`), adding 1 non-fully-read eligible face and 1
/// costly mana line. Batch 3 adds `Overlaid Terrain`
/// (`Coverage::Unimplemented`), adding 1 non-fully-read eligible face and 1
/// granted mana line.
///
/// This remains the existing reader's population: it recognizes "add", not
/// "adds". The separate coverage gap (including Eladamri's Vineyard) and the
/// Aura fixtures needed to close it are recorded in observed-faults #65.
const FULL_CORPUS_OTHER_PERMANENTS: Bounds = Bounds {
    colors: 475,
    routes: 215,
    promises: 214..=214,
    creature_promises: 92..=92,
    unparsed: 62..=62,
    fully_read: 385,
    costly_lines: 149..=149,
    triggered_lines: 29..=29,
    granted_lines: 42..=42,
    self_granted: 7..=7,
    busy: 0,
    not_implemented: 0,
    ..OTHER_PERMANENTS
};

/// Enumerate eligibility without building any boards, separately from the
/// sweep's success counters. Report both coverage tiers so a Generated face
/// cannot silently disappear behind an aggregate floor or a failed fixture.
fn check_other_permanent_population(tally: &Tally) {
    let mut eligible = [0_usize; 3];
    let mut readable = [0_usize; 3];
    for def in baylee_cards::all() {
        let tier = match def.coverage {
            baylee_cards_dsl::Coverage::Implemented => 0,
            baylee_cards_dsl::Coverage::Generated => 1,
            _ => 2,
        };
        for face in other_permanent_faces(def) {
            eligible[tier] += 1;
            if def.is_fully_read()
                && prints(def, face)
                    .expect("face has Oracle text")
                    .unparsed
                    .is_empty()
            {
                readable[tier] += 1;
            }
        }
    }
    println!(
        "other permanents census [Implemented, Generated, other]: eligible={eligible:?}, wholly readable={readable:?}"
    );
    assert_eq!(
        eligible,
        [445, 0, 19],
        "the coverage-tier population changed"
    );
    assert_eq!(readable, [385, 0, 0], "the readable coverage tiers changed");
    let expected: usize = readable.iter().sum();
    assert_eq!(
        expected, 385,
        "the independently enumerated full-corpus population changed; review additions/removals before updating its pin"
    );
    assert_eq!(
        tally.fully_read, expected,
        "an eligible wholly readable face was skipped by the board sweep"
    );
    // Pin successful work as well as eligibility: losing a route, color or
    // printed promise cannot be compensated by still counting its face.
    assert_eq!(
        (
            tally.colors,
            tally.routes,
            tally.promises,
            tally.opponents_lands
        ),
        (475, 215, 214, 2)
    );
    assert_eq!((tally.creatures, tally.creature_promises), (244, 92));
    assert_eq!((tally.triggered_lines, tally.unparsed.len()), (29, 62));
    assert_eq!(
        (
            tally.conditional,
            tally.costly,
            tally.unreadable,
            tally.dual_typed,
            tally.self_typed
        ),
        (4, 104, 84, 0, 0),
        "the measured skip categories changed"
    );
}

/// These reviewed definitions explain every change from the old pins.
/// In particular, a partial creature must not recover the free mana route
/// removed for omitting a printed cost; a held card stays in the text census.
#[test]
fn reviewed_cards_explain_the_mana_population_changes() {
    use baylee_core::generated::index;

    for (card, fully_read, expected) in [
        (index::BENTHIC_EXPLORERS, false, (0, 1, 1, 0, 0, 0)),
        (index::SQUANDERED_RESOURCES, false, (0, 0, 1, 0, 0, 0)),
        (index::LOTUS_BLOSSOM, true, (1, 0, 1, 0, 0, 1)),
        (index::THRAN_TURBINE, true, (1, 0, 0, 1, 0, 0)),
        (index::WITCH_ENGINE, true, (0, 1, 0, 0, 1, 0)),
        (index::METALWORKER, false, (0, 1, 0, 0, 1, 0)),
        (index::FOOD_CHAIN, true, (1, 0, 1, 0, 0, 1)),
        (index::KYREN_TOY, false, (0, 0, 1, 0, 0, 0)),
    ] {
        let def = baylee_cards::by_index(card).expect("reviewed card remains registered");
        assert_eq!(def.is_fully_read(), fully_read, "{}", def.name());
        assert_eq!(other_permanent_faces(def), [0], "{}", def.name());
        let (offenders, tally) = walk(&[def], other_permanent_faces);
        report(def.name(), &tally);
        assert!(offenders.is_empty(), "{offenders:#?}");
        assert_eq!(
            (
                tally.fully_read,
                tally.creatures,
                tally.costly_lines,
                tally.triggered_lines,
                tally.unparsed.len(),
                tally.costly,
            ),
            expected,
            "{}: read faces, creatures, costly/triggered/unparsed lines, costly abilities",
            def.name(),
        );
        assert_eq!(
            (tally.routes, tally.colors, tally.promises, tally.busy),
            (0, 0, 0, 0),
            "{} has no simple tap-only mana route to exercise or skipped board",
            def.name(),
        );
    }
}
