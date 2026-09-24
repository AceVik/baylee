//! The rule against real printings.
//!
//! `fixtures/printings.json` holds, for each card below, its English Oracle
//! and printings in one language, **verbatim** from the catalog on the date
//! the file records. None of it is written by hand: an invented German
//! sentence tests the rule against what somebody expected a translation to
//! look like, and two of the fixtures this replaces (a Sheoldred sentence and
//! `({T}: Erzeuge {G}.)`) are printed on no German card at all. Where a card
//! has many printings with the same text, the newest two of each text are
//! kept, and the file was cut only where that left the pick unchanged.
//!
//! `cargo test -p baylee-catalog --test cardtext_provenance -- --ignored`
//! holds every row against an ingested catalog again.
//!
//! Each case names the shape it pins. One table and one check per question,
//! so a change to the rule that moves one card names that card.

use baylee_cardtext::{
    Printing, Stage, Verdict, align, localized, pick, sentences, split_cost, untranslated, verify,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct File {
    cards: Vec<Card>,
}

#[derive(Deserialize)]
struct Card {
    name: String,
    lang: String,
    oracle: Vec<String>,
    printings: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    scryfall_id: String,
    set: String,
    collector_number: String,
    released_at: String,
    layout: String,
    printed: Vec<Option<String>>,
}

impl Row {
    fn printing(&self) -> Printing {
        Printing {
            scryfall_id: self.scryfall_id.clone(),
            released_at: self.released_at.clone(),
            collector_number: self.collector_number.clone(),
            layout: self.layout.clone(),
            printed: self.printed.clone(),
        }
    }
}

fn cards() -> Vec<Card> {
    let file: File = serde_json::from_str(include_str!("../fixtures/printings.json"))
        .expect("the fixture file parses");
    file.cards
}

/// What the rule does with a card: the printing it picks, how each face
/// lines up, and what a few named rows say.
#[derive(Clone)]
struct Case {
    card: &'static str,
    lang: &'static str,
    shape: &'static str,
    /// Set and collector number of the printing picked.
    pick: Option<(&'static str, &'static str)>,
    /// Per Oracle face: the stage it lined up at, or `None` for the Oracle.
    faces: &'static [Option<Stage>],
    /// `(face, Oracle line, verdict, printed cost)`.
    rows: &'static [(usize, usize, Verdict, Option<&'static str>)],
}

use Stage::{Markers, Raw};
use Verdict::{Permuted, Positional, Subset};

const PICKS: &[Case] = &[
    Case {
        card: "Bonesplitter",
        lang: "de",
        shape: "a keyword line whose only colon is in its reminder is drawn whole",
        pick: Some(("cmr", "458")),
        faces: &[Some(Raw)],
        rows: &[(0, 1, Positional, None)],
    },
    Case {
        card: "Brazen Borrower",
        lang: "de",
        shape: "the newest printing leaves the Adventure NULL; the newest to translate both faces wins",
        pick: Some(("woc", "85")),
        faces: &[Some(Raw), Some(Raw)],
        rows: &[(1, 0, Positional, None)],
    },
    Case {
        card: "Cascade Bluffs",
        lang: "de",
        shape: "the newest de printing (eoc) is English with garbled braces and is skipped",
        pick: Some(("tdc", "345")),
        faces: &[Some(Raw)],
        rows: &[(0, 1, Positional, Some("{U/R}, {T}"))],
    },
    Case {
        card: "Chromatic Lantern",
        lang: "de",
        shape: "a grant's quoted colon is not the Lantern's cost; a same-day tie takes the lower number",
        pick: Some(("msc", "195")),
        faces: &[Some(Raw)],
        rows: &[(0, 0, Positional, None), (0, 1, Positional, Some("{T}"))],
    },
    Case {
        card: "Clearwater Pathway",
        lang: "de",
        shape: "each face of a modal double-faced card prints the other face as two hint lines",
        pick: Some(("znr", "260")),
        faces: &[Some(Markers), Some(Markers)],
        rows: &[
            (0, 0, Positional, Some("{T}")),
            (1, 0, Positional, Some("{T}")),
        ],
    },
    Case {
        card: "Elven Palisade",
        lang: "de",
        shape: "the only German printing has no text: the Oracle",
        pick: None,
        faces: &[],
        rows: &[],
    },
    Case {
        card: "Fetid Heath",
        lang: "de",
        shape: "`{(}w/b)}` is `{W/B}`, and its parenthesis does not end the cost",
        pick: Some(("fic", "391")),
        faces: &[Some(Raw)],
        rows: &[(0, 1, Positional, Some("{W/B}, {T}"))],
    },
    Case {
        card: "Flame Spirit",
        lang: "de",
        shape: "`{R}:Der Flammengeist` has no space after its colon",
        pick: Some(("6ed", "179")),
        faces: &[Some(Raw)],
        rows: &[(0, 0, Positional, Some("{R}"))],
    },
    Case {
        card: "Flowstone Wall",
        lang: "de",
        shape: "a Wall's reminder on its own line lines up raw and stays lined up",
        pick: Some(("nem", "86")),
        faces: &[Some(Raw)],
        rows: &[(0, 0, Positional, None), (0, 1, Positional, Some("{R}"))],
    },
    Case {
        card: "Gemstone Mine",
        lang: "de",
        shape: "a cost fifty bytes long is still the cost",
        pick: Some(("dmr", "247")),
        faces: &[Some(Raw)],
        rows: &[(
            0,
            1,
            Positional,
            Some("{T}, entferne eine Minenmarke von der Edelsteinmine"),
        )],
    },
    Case {
        card: "Granger Guildmage",
        lang: "de",
        shape: "the printing swaps the two abilities; the costs put them back",
        pick: Some(("mir", "220")),
        faces: &[Some(Raw)],
        rows: &[
            (0, 0, Permuted(1), Some("{R}, {T}")),
            (0, 1, Permuted(0), Some("{W}, {T}")),
        ],
    },
    Case {
        card: "Ifnir Deadlands",
        lang: "de",
        shape: "`{2BB}` is `{2}{B}{B}`",
        pick: Some(("ecc", "153")),
        faces: &[Some(Raw)],
        rows: &[(0, 2, Positional, Some("{2}{B}{B}, {T}, opfere eine Wüste"))],
    },
    Case {
        card: "Jace, the Mind Sculptor",
        lang: "de",
        shape: "a loyalty cost is its own head",
        pick: Some(("blc", "75")),
        faces: &[Some(Raw)],
        rows: &[
            (0, 0, Positional, Some("+2")),
            (0, 2, Positional, Some("−1")),
        ],
    },
    Case {
        card: "Jace, the Mind Sculptor",
        lang: "ja",
        shape: "a full-width colon is a colon",
        pick: Some(("blc", "75")),
        faces: &[Some(Raw)],
        rows: &[
            (0, 0, Positional, Some("+2")),
            (0, 3, Positional, Some("-12")),
        ],
    },
    Case {
        card: "Kavaron, Memorial World",
        lang: "de",
        shape: "both German printings are the English text: the Oracle",
        pick: None,
        faces: &[],
        rows: &[],
    },
    Case {
        card: "Lush Portico",
        lang: "de",
        shape: "the regular frame and the borderless one tie on the day; the regular one is picked",
        pick: Some(("mkm", "263")),
        faces: &[Some(Raw)],
        rows: &[(0, 2, Positional, None)],
    },
    Case {
        card: "Mind Stone",
        lang: "de",
        shape: "a cost printed in words",
        pick: Some(("fic", "353")),
        faces: &[Some(Raw)],
        rows: &[(0, 1, Positional, Some("{1}, {T}, opfere dieses Artefakt"))],
    },
    Case {
        card: "Mind Stone",
        lang: "ja",
        shape: "a cost printed in words, before a full-width colon",
        pick: Some(("fic", "353")),
        faces: &[Some(Raw)],
        rows: &[(
            0,
            1,
            Positional,
            Some("{1}, {T}, このアーティファクトを生け贄に捧げる"),
        )],
    },
    Case {
        card: "Obelisk of Undoing",
        lang: "de",
        shape: "`6, {T}` braces only the tap",
        pick: Some(("5ed", "392")),
        faces: &[Some(Raw)],
        rows: &[(0, 0, Subset, Some("6, {T}"))],
    },
    Case {
        card: "Overgrown Tomb",
        lang: "de",
        shape: "the regular frame and the borderless one tie on the day; the regular one is picked",
        pick: Some(("rvr", "283")),
        faces: &[Some(Raw)],
        rows: &[(0, 1, Positional, None)],
    },
    Case {
        card: "Skullclamp",
        lang: "en",
        shape: "English is the Oracle, never a promo's printed wording",
        pick: None,
        faces: &[],
        rows: &[],
    },
    Case {
        card: "Spawning Pool",
        lang: "de",
        shape: "the newest printing glues two lines together; an older one that lines up wins",
        pick: Some(("ulg", "142")),
        faces: &[Some(Raw)],
        rows: &[(0, 2, Positional, Some("{1}{B}"))],
    },
    Case {
        card: "Thornscape Apprentice",
        lang: "de",
        shape: "the printing swaps the two abilities; the costs put them back",
        pick: Some(("inv", "215")),
        faces: &[Some(Raw)],
        rows: &[(0, 0, Permuted(1), Some("{R}, {T}"))],
    },
    Case {
        card: "Thornscape Apprentice",
        lang: "fr",
        shape: "the same swap in French, with a space before one colon",
        pick: Some(("inv", "215")),
        faces: &[Some(Raw)],
        rows: &[(0, 1, Permuted(0), Some("{W}, {T}"))],
    },
    Case {
        card: "Urza's Saga",
        lang: "de",
        shape: "a chapter granting a quoted ability has no cost of its own",
        pick: Some(("mh2", "259")),
        faces: &[Some(Raw)],
        rows: &[(0, 1, Positional, None), (0, 2, Positional, None)],
    },
    Case {
        card: "Witch Enchanter",
        lang: "de",
        shape: "the land face's hint ends in the creature face's mana cost",
        pick: Some(("mh3", "239")),
        faces: &[Some(Markers), Some(Markers)],
        rows: &[(0, 0, Positional, None), (1, 1, Positional, Some("{T}"))],
    },
    Case {
        card: "Wizard Class",
        lang: "de",
        shape: "`//Level_2//` markers are not rules lines",
        pick: Some(("blc", "112")),
        faces: &[Some(Markers)],
        rows: &[
            (0, 2, Positional, Some("{2}{U}")),
            (0, 4, Positional, Some("{4}{U}")),
        ],
    },
];

/// One printing that is *not* the one picked, pinned for the shape it has.
struct Placed {
    card: &'static str,
    lang: &'static str,
    at: (&'static str, &'static str),
    face: usize,
    untranslated: bool,
    stage: Option<Stage>,
}

const PRINTINGS: &[Placed] = &[
    Placed {
        card: "Lush Portico",
        lang: "de",
        at: ("mkm", "327"),
        face: 0,
        untranslated: false,
        stage: Some(Stage::Reminders),
    },
    Placed {
        card: "Overgrown Tomb",
        lang: "de",
        at: ("rvr", "407"),
        face: 0,
        untranslated: false,
        stage: Some(Stage::Reminders),
    },
    Placed {
        card: "Spawning Pool",
        lang: "de",
        at: ("10e", "358"),
        face: 0,
        untranslated: false,
        stage: None,
    },
    Placed {
        card: "Brazen Borrower",
        lang: "de",
        at: ("spg", "30"),
        face: 0,
        untranslated: false,
        stage: Some(Markers),
    },
    Placed {
        card: "Brazen Borrower",
        lang: "de",
        at: ("soc", "190"),
        face: 1,
        untranslated: true,
        stage: None,
    },
    Placed {
        card: "Cascade Bluffs",
        lang: "de",
        at: ("eoc", "153"),
        face: 0,
        untranslated: true,
        stage: None,
    },
    Placed {
        card: "Kavaron, Memorial World",
        lang: "de",
        at: ("eoe", "255"),
        face: 0,
        untranslated: true,
        stage: None,
    },
    Placed {
        card: "Kavaron, Memorial World",
        lang: "de",
        at: ("eoe", "281"),
        face: 0,
        untranslated: true,
        stage: None,
    },
    Placed {
        card: "Elven Palisade",
        lang: "de",
        at: ("exo", "109"),
        face: 0,
        untranslated: true,
        stage: None,
    },
    Placed {
        card: "Mind Stone",
        lang: "de",
        at: ("c15", "259"),
        face: 0,
        untranslated: false,
        stage: None,
    },
];

fn find<'c>(cards: &'c [Card], name: &str, lang: &str) -> &'c Card {
    cards
        .iter()
        .find(|c| c.name == name && c.lang == lang)
        .unwrap_or_else(|| panic!("{name} ({lang}) is not in the fixture file"))
}

/// Every disagreement between the table and the rule, one line each.
fn check(cases: &[Case]) -> Vec<String> {
    let cards = cards();
    let mut wrong = Vec::new();
    for case in cases {
        let card = find(&cards, case.card, case.lang);
        let oracle: Vec<&str> = card.oracle.iter().map(String::as_str).collect();
        let printings: Vec<Printing> = card.printings.iter().map(Row::printing).collect();
        let layer = pick(case.lang, &oracle, &printings);
        let at = layer.as_ref().map(|l| {
            let row = card
                .printings
                .iter()
                .find(|r| r.scryfall_id == l.printing.scryfall_id)
                .expect("picked from the file");
            (row.set.as_str(), row.collector_number.as_str())
        });
        let name = format!("{} ({}) — {}", case.card, case.lang, case.shape);
        if at != case.pick {
            wrong.push(format!("{name}: picked {at:?}, table says {:?}", case.pick));
            continue;
        }
        let Some(layer) = layer else {
            continue;
        };
        let stages: Vec<Option<Stage>> = layer
            .faces
            .iter()
            .map(|f| f.as_ref().map(|a| a.stage))
            .collect();
        if stages != case.faces {
            wrong.push(format!(
                "{name}: faces {stages:?}, table says {:?}",
                case.faces
            ));
            continue;
        }
        for &(face, line, verdict, head) in case.rows {
            let aligned = layer.faces[face]
                .as_ref()
                .expect("the table names a face that lined up");
            let got = verify(&card.oracle[face], aligned, line);
            let printed = localized(&card.oracle[face], aligned, line)
                .and_then(split_cost)
                .map(|s| s.head);
            if (got, printed) != (verdict, head) {
                wrong.push(format!(
                    "{name}: face {face} line {line} is {got:?} with cost {printed:?}, table says {verdict:?} with {head:?}"
                ));
            }
        }
    }
    wrong
}

#[test]
fn every_card_is_picked_placed_and_verified_as_the_table_says() {
    let wrong = check(PICKS);
    assert!(
        wrong.is_empty(),
        "{} disagreement(s):\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The injection: the same check over a table with one expectation changed
/// has to fail, or a green run above says nothing about the rule.
#[test]
fn a_doctored_table_is_caught() {
    let mut doctored = PICKS.to_vec();
    let guildmage = doctored
        .iter_mut()
        .find(|c| c.card == "Granger Guildmage")
        .unwrap();
    guildmage.rows = &[(0, 0, Positional, Some("{W}, {T}"))];
    let wrong = check(&doctored);
    assert_eq!(wrong.len(), 1, "{wrong:?}");
    assert!(wrong[0].starts_with("Granger Guildmage"), "{wrong:?}");

    let mut doctored = PICKS.to_vec();
    doctored
        .iter_mut()
        .find(|c| c.card == "Spawning Pool")
        .unwrap()
        .pick = Some(("10e", "358"));
    assert_eq!(check(&doctored).len(), 1);
}

#[test]
fn each_pinned_printing_is_placed_as_the_table_says() {
    let cards = cards();
    let mut wrong = Vec::new();
    for p in PRINTINGS {
        let card = find(&cards, p.card, p.lang);
        let row = card
            .printings
            .iter()
            .find(|r| (r.set.as_str(), r.collector_number.as_str()) == p.at)
            .unwrap_or_else(|| panic!("{} {:?} is not in the fixture file", p.card, p.at));
        let oracle = &card.oracle[p.face];
        let printed = row.printed.get(p.face).cloned().flatten();
        let is_untranslated = untranslated(oracle, printed.as_deref());
        let stage = printed
            .as_deref()
            .and_then(|t| align(oracle, t, &row.layout))
            .map(|a| a.stage);
        let stage = if is_untranslated { None } else { stage };
        if (is_untranslated, stage) != (p.untranslated, p.stage) {
            wrong.push(format!(
                "{} {:?} face {}: untranslated {is_untranslated}, {stage:?}; table says {}, {:?}",
                p.card, p.at, p.face, p.untranslated, p.stage
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The table and the file describe the same cards: a fixture nobody states
/// an expectation for is a row nothing checks.
#[test]
fn every_fixture_card_has_a_case() {
    let cards = cards();
    for card in &cards {
        assert!(
            PICKS
                .iter()
                .any(|c| c.card == card.name && c.lang == card.lang),
            "{} ({}) has printings in the file and no case in the table",
            card.name,
            card.lang
        );
        assert!(!card.oracle.is_empty() && card.oracle.iter().all(|o| sentences(o).count() > 0));
    }
    assert_eq!(cards.len(), PICKS.len());
}

/// The table reaches every outcome the rule has, so a rule that stopped
/// producing one of them would show up as a disagreement rather than as a
/// table nobody noticed had stopped covering it.
#[test]
fn the_table_reaches_every_outcome() {
    let stages: Vec<Stage> = PICKS
        .iter()
        .flat_map(|c| c.faces.iter().flatten().copied())
        .chain(PRINTINGS.iter().filter_map(|p| p.stage))
        .collect();
    for stage in [Raw, Markers, Stage::Reminders] {
        assert!(stages.contains(&stage), "no case lines up at {stage:?}");
    }
    let verdicts: Vec<Verdict> = PICKS
        .iter()
        .flat_map(|c| c.rows.iter().map(|r| r.2))
        .collect();
    assert!(verdicts.contains(&Positional) && verdicts.contains(&Subset));
    assert!(verdicts.iter().any(|v| matches!(v, Permuted(_))));
    assert!(PICKS.iter().any(|c| c.pick.is_none()));
    assert!(PICKS.iter().flat_map(|c| c.rows).any(|r| r.3.is_none()));
    assert!(
        PRINTINGS
            .iter()
            .any(|p| !p.untranslated && p.stage.is_none()),
        "no printing is refused"
    );
}
