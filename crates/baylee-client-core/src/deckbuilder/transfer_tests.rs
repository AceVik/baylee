//! Import and export through the builder, as a player drives them: paste,
//! read, take, check the report; choose a format, read what it left out.

use super::transfer::{Said, Stage, Tone, Transfer, VersionRows};
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_deckio::FormatId;

/// A real "Copy for Moxfield" export.
const MOXFIELD: &str = include_str!("../../../baylee-deckio/tests/fixtures/moxfield-export.txt");

fn card(index: u32, name: &str, kinds: &[&str]) -> PoolCard {
    PoolCard {
        index,
        name: name.to_string(),
        english_name: name.to_string(),
        kinds: kinds.iter().map(|k| (*k).to_string()).collect(),
        type_line: kinds.join(" "),
        coverage: Coverage::Implemented,
        basic_land: name == "Forest",
        ..PoolCard::default()
    }
}

/// A pool with a few of the sample's cards, one of them double-faced the
/// way `baylee_cards::pool::row` builds it: the front face as its English
/// name, Scryfall's whole spelling among its other names.
fn pool() -> Vec<PoolCard> {
    let mut avacyn = card(1, "Archangel Avacyn", &["Creature"]);
    avacyn.double_faced = true;
    avacyn.alt_names = vec!["Archangel Avacyn // Avacyn, the Purifier".into()];
    let mut atraxa = card(2, "Atraxa, Grand Unifier", &["Creature"]);
    atraxa.commander = true;
    let mut bolt = card(5, "Lightning Bolt", &["Instant"]);
    bolt.alt_names = vec!["Blitzschlag".into()];
    vec![
        avacyn,
        atraxa,
        card(3, "Jetmir's Garden", &["Land"]),
        card(4, "Forest", &["Land"]),
        bolt,
    ]
}

fn builder() -> DeckBuilder {
    let mut b = DeckBuilder::new();
    b.set_pool(pool(), true);
    b
}

fn names(b: &DeckBuilder, zone: Zone) -> Vec<(String, u16)> {
    b.entries(zone)
        .iter()
        .map(|e| (b.card(e.slot).unwrap().english_name.clone(), e.count))
        .collect()
}

fn texts(lines: &[super::transfer::Line]) -> String {
    lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn import(b: &mut DeckBuilder, text: &str) {
    b.open_import();
    b.import_paste(text);
    assert!(b.import_confirm(Lang::En), "{:?}", b.transfer());
}

#[test]
fn the_moxfield_sample_imports_and_reports_what_the_pool_lacks() {
    let mut b = builder();
    b.open_import();
    b.import_paste(MOXFIELD);
    let before = texts(&b.import_lines(Lang::En));
    assert!(
        before.starts_with("Read as Moxfield. Rows: 129 · Cards: 129"),
        "{before}"
    );
    assert!(b.import_confirm(Lang::En));

    // `Archangel Avacyn / Avacyn, the Purifier` reached the pool's row for
    // the card through its whole name; the foil stayed a foil.
    assert_eq!(
        names(&b, Zone::Main),
        [
            ("Archangel Avacyn".to_string(), 1),
            ("Atraxa, Grand Unifier".to_string(), 1),
            ("Forest".to_string(), 1),
        ]
    );
    assert_eq!(names(&b, Zone::Side), [("Jetmir's Garden".to_string(), 1)]);
    let garden = &b.entries(Zone::Side)[0];
    assert_eq!(garden.print.finish, Some(baylee_core::preset::Finish::Foil));
    assert_eq!(garden.print.collector_number.as_deref(), Some("250s"));

    // Every other card is missing, named, and blocks the save.
    assert_eq!(b.missing().len(), 125);
    assert!(
        !b.saveable(),
        "a deck missing cards must not save over them"
    );
    let report = texts(&b.import_lines(Lang::En));
    assert!(report.contains("Imported from Moxfield. Rows: 129 · Cards: 129"));
    assert!(report.contains("Not in this gateway's card pool: 125."));
    assert!(report.contains("  Aether Channeler"));
    assert!(report.contains("  and 117 more"));
    assert!(report.contains("The file names no deck, so it is called “Imported deck”."));
    assert!(report.contains("The file names no commander."), "{report}");
    assert!(b.dirty(), "an import is not saved yet");
    assert_eq!(b.editing(), None, "and it is a new deck");
}

#[test]
fn an_import_before_the_pool_arrives_resolves_when_it_does() {
    let mut b = DeckBuilder::new();
    import(
        &mut b,
        "COMMANDER:\n1 Atraxa, Grand Unifier\n\n4 Lightning Bolt\n",
    );
    let waiting = texts(&b.import_lines(Lang::En));
    assert!(waiting.contains("Waiting for the card pool"), "{waiting}");
    assert!(
        b.missing().is_empty(),
        "nothing is missing before the pool says so"
    );
    b.set_pool(pool(), true);
    assert_eq!(
        names(&b, Zone::Main),
        [
            ("Atraxa, Grand Unifier".to_string(), 1),
            ("Lightning Bolt".to_string(), 4),
        ]
    );
    assert_eq!(b.commander_names(), ["Atraxa, Grand Unifier"]);
    let after = texts(&b.import_lines(Lang::En));
    assert!(!after.contains("Waiting"), "{after}");
}

#[test]
fn a_moxfield_link_is_answered_in_the_players_language_and_takes_nothing() {
    let mut b = builder();
    b.open_import();
    b.import_paste("https://moxfield.com/decks/Xq3ExampleDeck0000000a");
    assert!(matches!(
        b.transfer(),
        Some(Transfer::Import(i)) if matches!(i.stage(), Stage::Instruction(..))
    ));
    let en = texts(&b.import_lines(Lang::En));
    assert!(en.contains("More → Export → Copy for Moxfield"), "{en}");
    let de = texts(&b.import_lines(Lang::De));
    assert!(de.contains("Öffne das Deck auf Moxfield"), "{de}");
    assert!(!b.import_confirm(Lang::En), "a link is not a deck");
    assert!(b.entries(Zone::Main).is_empty());
}

#[test]
fn a_paste_replaces_the_last_one_and_a_huge_one_is_refused() {
    let mut b = builder();
    b.open_import();
    b.import_paste("4 Forest");
    b.import_paste("1 Lightning Bolt");
    assert!(b.import_confirm(Lang::En));
    assert_eq!(names(&b, Zone::Main), [("Lightning Bolt".to_string(), 1)]);

    b.open_import();
    b.import_paste(&"1 Forest\n".repeat(40_000));
    let Some(Transfer::Import(importing)) = b.transfer() else {
        panic!("open");
    };
    assert!(importing.pasted().len() <= baylee_deckio::MAX_DOCUMENT_BYTES + 1);
    assert!(texts(&b.import_lines(Lang::En)).contains("too large"));
    assert!(!b.import_confirm(Lang::En));
}

#[test]
fn every_format_carries_the_deck_back_into_the_builder() {
    let mut a = builder();
    import(
        &mut a,
        "# baylee deck export v1\n# name: Rundreise\n\
         CMD: 1 Atraxa, Grand Unifier (ONE) 196 [de] *G* # die Anführerin\n\
         3 Lightning Bolt (M11) 149 [ja] *F* scryfall=e3285e6a-8c1d-4c9f-9a3f-2f0a4d2f0a4d\n\
         1 Archangel Avacyn (SOI) 5 *E*\n\
         SB: 2 Forest\n",
    );
    assert_eq!(a.name(), "Rundreise");
    assert_eq!(a.commander_names(), ["Atraxa, Grand Unifier"]);
    for format in FormatId::ALL {
        let written = a.export(format);
        let mut b = builder();
        import(&mut b, &written.text);
        assert_eq!(b.entries(Zone::Main).len(), a.entries(Zone::Main).len());
        assert_eq!(names(&b, Zone::Main), names(&a, Zone::Main), "{format:?}");
        assert_eq!(names(&b, Zone::Side), names(&a, Zone::Side), "{format:?}");
        assert_eq!(b.commander_names(), a.commander_names(), "{format:?}");
        if format == FormatId::Moxfield {
            // Moxfield keeps set, number and foil/etched, and says it lost
            // the rest.
            for (x, y) in a.entries(Zone::Main).iter().zip(b.entries(Zone::Main)) {
                assert_eq!(x.print.set, y.print.set);
                assert_eq!(x.print.collector_number, y.print.collector_number);
            }
            assert!(!written.losses.is_empty());
        } else {
            assert_eq!(b.entries(Zone::Main), a.entries(Zone::Main), "{format:?}");
            assert_eq!(b.name(), "Rundreise", "{format:?}");
        }
    }
}

#[test]
fn moxfield_export_spells_a_double_faced_card_as_moxfield_does() {
    let mut b = builder();
    import(&mut b, "1 Archangel Avacyn\n");
    let moxfield = b.export(FormatId::Moxfield).text;
    assert_eq!(moxfield, "1 Archangel Avacyn / Avacyn, the Purifier\n");
    // Baylee's own text writes the name the builder saves.
    let own = b.export(FormatId::Baylee).text;
    assert!(own.ends_with("\n1 Archangel Avacyn\n"), "{own}");
}

#[test]
fn the_export_dialog_names_what_the_format_leaves_out() {
    let mut b = builder();
    import(&mut b, "# name: Notiert\n1 Forest [de] # der Wald\n");
    b.open_export();
    b.export_choose(FormatId::Moxfield);
    let written = b.export(FormatId::Moxfield);
    let lines = b.export_lines(&written, Lang::En);
    let said = texts(&lines);
    assert!(said.contains("Moxfield cannot say everything"), "{said}");
    assert!(said.contains("the deck's name"));
    assert!(said.contains("languages, rows: 1"));
    assert!(said.contains("notes, rows: 1"));
    assert!(lines.iter().skip(1).all(|l| l.tone == Tone::Warn));

    b.export_choose(FormatId::Json);
    let written = b.export(FormatId::Json);
    assert_eq!(
        texts(&b.export_lines(&written, Lang::En)),
        "This format keeps everything about the deck."
    );
    b.export_said(Said::Copied);
    assert!(texts(&b.export_lines(&written, Lang::De)).contains("In die Zwischenablage kopiert."));
    b.export_step(true);
    assert!(matches!(b.transfer(), Some(Transfer::Export(e)) if e.format == FormatId::Yaml));
    b.close_transfer();
    b.open_export();
    assert!(
        matches!(b.transfer(), Some(Transfer::Export(e)) if e.format == FormatId::Yaml),
        "the dialog opens on the format last chosen"
    );
}

#[test]
fn an_export_file_is_named_after_the_deck_and_safe_everywhere() {
    let mut b = builder();
    b.set_name("Bolt / Burn: v2?");
    assert_eq!(b.export_file_name(FormatId::Json), "Bolt _ Burn_ v2_.json");
    assert_eq!(
        b.export_file_name(FormatId::Moxfield),
        "Bolt _ Burn_ v2_-moxfield.txt"
    );
    b.set_name("  ");
    assert_eq!(b.export_file_name(FormatId::Yaml), "deck.yaml");
}

#[test]
fn a_hand_typed_list_finds_its_cards() {
    let b = builder();
    assert_eq!(
        b.slot_of("lightning bolt"),
        Some(4),
        "case, for a typed list"
    );
    assert_eq!(b.slot_of("Blitzschlag"), Some(4), "another language's name");
    assert_eq!(
        b.slot_of("Archangel Avacyn // Avacyn, the Purifier"),
        Some(0),
        "Scryfall's whole name"
    );
    assert_eq!(
        b.slot_of("Archangel Avacyn // Somebody Else"),
        None,
        "never a split"
    );
}

#[test]
fn a_leader_the_rules_will_not_seat_is_said_not_marked() {
    let mut b = builder();
    import(&mut b, "CMD: 1 Lightning Bolt\n");
    assert!(b.commanders().is_empty());
    let report = texts(&b.import_lines(Lang::En));
    assert!(
        report.contains("Lightning Bolt cannot lead a deck"),
        "{report}"
    );
}

#[test]
fn a_maybeboard_is_reported_not_kept() {
    let mut b = builder();
    import(&mut b, "1 Forest\nMB: 1 Lightning Bolt\n");
    assert_eq!(names(&b, Zone::Main), [("Forest".to_string(), 1)]);
    assert!(texts(&b.import_lines(Lang::En)).contains("Maybeboard rows not kept"));
}

#[test]
fn a_data_url_carries_every_byte_and_ends_nowhere_early() {
    let url = super::transfer::data_url("text/plain", "1 Jötun Grunt # a, b?\n");
    assert_eq!(
        url,
        "data:text/plain;charset=utf-8,1%20J%C3%B6tun%20Grunt%20%23%20a%2C%20b%3F%0A"
    );
}

#[test]
fn a_saved_export_never_replaces_a_file_that_is_there() {
    let dir = std::env::temp_dir().join(format!("baylee-export-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let first = super::transfer::save_to(&dir, "Deck.txt", "one").expect("saved");
    let second = super::transfer::save_to(&dir, "Deck.txt", "two").expect("saved");
    assert_eq!(first, dir.join("Deck.txt"));
    assert_eq!(second, dir.join("Deck (2).txt"));
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "one");
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "two");
    let _ = std::fs::remove_dir_all(&dir);
}

/// History's Export… writes a saved version's rows, printings and all, and
/// names the file after it; the deck in hand is untouched, and closing the
/// dialog (or opening the deck's own export) forgets the version.
#[test]
fn a_saved_version_exports_its_own_rows_and_not_the_deck_in_hand() {
    let mut b = builder();
    import(&mut b, "# name: Weltenbaum\n1 Forest\n");
    let in_hand = b.export(FormatId::Baylee).text;
    b.open_version_export(VersionRows {
        name: "Weltenbaum v7".into(),
        cards: vec!["4 Lightning Bolt (M11) 149 *F*".into()],
        sideboard: vec!["2 Forest".into()],
        commanders: Vec::new(),
    });
    assert!(matches!(b.transfer(), Some(Transfer::Export(_))));
    let written = b.export(FormatId::Baylee).text;
    assert!(
        written.contains("4 Lightning Bolt (M11) 149 *F*"),
        "{written}"
    );
    assert!(written.contains("2 Forest"), "{written}");
    assert!(written.contains("Weltenbaum v7"), "{written}");
    assert!(!written.contains("1 Forest"), "{written}");
    assert_eq!(b.export_file_name(FormatId::Baylee), "Weltenbaum v7.txt");
    b.close_transfer();
    assert!(b.export_version().is_none());
    b.open_version_export(VersionRows {
        name: "x".into(),
        cards: Vec::new(),
        sideboard: Vec::new(),
        commanders: Vec::new(),
    });
    b.open_export();
    assert_eq!(
        b.export(FormatId::Baylee).text,
        in_hand,
        "the deck's own export"
    );
}
