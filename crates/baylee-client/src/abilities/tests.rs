use super::*;
use baylee_client_core::card_face::{CardTextEntry, FaceText};
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{GRANTED_ABILITY, LegalActions, PREPARED_CAST, Pending};

#[test]
fn required_intrinsic_mana_names_current_output_with_mana_symbols() {
    use baylee_core::generated::subtypes::land;
    let mut forest = baylee_client_core::test_support::printed(9, 0, "Forest", 1);
    // A Hack-changed Forest taps for blue: do not infer its printed name.
    forest.subtypes = baylee_core::types::SubtypeSet::default();
    forest.subtypes.insert(land::ISLAND);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![forest])
        .build();
    let label = required_mana_label(
        Lang::De,
        &view,
        baylee_engine::choice::ManaAbilityChoice {
            source: baylee_core::ids::DamageSourceRef {
                object: ObjectId::new(9, 0),
                version: 1,
            },
            ability_index: None,
        },
    );
    assert!(label.contains("{T}"));
    assert!(label.contains("{U}"));
    assert!(!label.contains("{G}"));
}

#[test]
fn an_effigy_copying_harabaz_keeps_its_variable_mana_as_a_written_choice() {
    let effigy = baylee_cards::decks::by_name("Machine God's Effigy").unwrap();
    let druid = baylee_cards::decks::by_name("Harabaz Druid").unwrap();
    let mut object = baylee_client_core::test_support::printed(1, 0, "Harabaz Druid", 1);
    object.card.as_mut().unwrap().index = effigy;
    object.rules = Some(baylee_view::RulesFace {
        card: druid,
        face: 0,
    });
    object.granted_mana = Some(baylee_view::GrantedMana {
        slot: 0,
        colors: vec![ManaColor::Blue],
        amount: 1,
    });
    let id = object.id;
    let view = ViewBuilder::new(2).with_battlefield(0, [object]).build();
    let interaction = offering(vec![(id, 0), (id, GRANTED_ABILITY)], vec![id]);
    let options = options(Lang::En, &view, &interaction, id);
    assert_eq!(Split::of(&options), Split { pips: 1, rows: 1 });
    let pour = options[0].pour.as_ref().unwrap();
    assert_eq!(pour.color, ManaColor::Blue);
    assert_eq!(pour.step.tap, Tap::Ability(GRANTED_ABILITY));
    assert_eq!(options[1].printed_index(), Some(0));
    assert!(options[1].mana);
    assert!(
        options[1].pour.is_none(),
        "the X-mana ability remains an explicit choice"
    );
    let colors = options_for(Lang::En, &view, &interaction, id, Some(0));
    assert_eq!(colors.len(), 5);
    let blue = colors
        .iter()
        .find_map(|o| o.pour.as_ref().filter(|p| p.color == ManaColor::Blue))
        .unwrap();
    assert_eq!(
        blue.step.tap,
        Tap::Ability(0),
        "choosing blue on Harabaz still uses Harabaz, not the grant"
    );
}

/// A bundle is drawn as what arrives, and the five-colour case is the one
/// that matters.
///
/// `mana_choice` answers "any color" as soon as all five are listed, which
/// is true of a choice and false of Composite Golem — it adds one of each
/// and offers nothing. The negative assertion is paired with a positive
/// one on purpose: "does not say any color" passes just as well on an
/// empty string, so the same label is required to carry all five pips.
#[test]
fn a_bundle_is_drawn_as_what_arrives_and_never_as_a_choice() {
    let bundle = |colors: Vec<ManaColor>| baylee_client_core::manaplan::Source {
        id: ObjectId::new(1, 0),
        tap: Tap::Ability(0),
        amount: u8::try_from(colors.len()).unwrap_or(u8::MAX),
        colors,
        bundle: true,
        priced: false,
        preserve: 0,
        dear: None,
    };

    let karoo = bundle(vec![ManaColor::White, ManaColor::Blue]);
    let label = mana_label(Lang::En, &karoo);
    assert!(
        label.contains(pip(ManaColor::White)) && label.contains(pip(ManaColor::Blue)),
        "a Karoo's label names both colours: {label}"
    );

    let golem = bundle(vec![
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ]);
    let label = mana_label(Lang::En, &golem);
    assert!(
        !label.contains(Phrase::AnyColor.text(Lang::En)),
        "Composite Golem adds one of each and offers no choice: {label}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            label.contains(pip(color)),
            "every colour it makes is drawn: {label}"
        );
    }
}

fn offering(abilities: Vec<(ObjectId, u32)>, mana: Vec<ObjectId>) -> Interaction {
    Interaction::new(
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                abilities,
                mana_abilities: mana,
                ..LegalActions::default()
            }),
        },
        PlayerId::new(0),
    )
}

#[test]
fn a_legal_owner_ability_on_an_opponents_permanent_stays_available() {
    let mut object = baylee_client_core::test_support::printed(1, 1, "Personal Incarnation", 1);
    object.owner = PlayerId::new(0);
    object.card.as_mut().unwrap().index =
        baylee_cards::decks::by_name("Personal Incarnation").unwrap();
    object.rules = object.card.map(baylee_view::RulesFace::from);
    let id = object.id;
    let view = ViewBuilder::new(2).with_battlefield(1, [object]).build();
    assert_ne!(view.object(id).unwrap().controller, view.seat);
    let rows = options(Lang::En, &view, &offering(vec![(id, 0)], vec![]), id);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].action,
        PlayerAction::ActivateAbility {
            source: id,
            ability_index: 0
        }
    );
    assert_eq!(
        rows[0].cost.as_deref(),
        Some("{0}"),
        "a printed {{0}} is the cost column, as every other cost is"
    );
    let words = printed_words(None, &view, id, &rows[0]).unwrap();
    let sentence = words
        .blocks
        .iter()
        .map(TextBlock::text)
        .collect::<Vec<_>>()
        .join(" ");
    // The `{0}` is the cost column; the words follow it.
    assert!(sentence.starts_with("The next 1 damage"), "{sentence}");
    assert!(sentence.contains("owner may activate"), "{sentence}");
}

#[test]
fn clockwork_beasts_refill_keeps_its_full_sentence_and_symbol_cost() {
    let mut beast = baylee_client_core::test_support::printed(1, 0, "Clockwork Beast", 1);
    beast.card.as_mut().unwrap().index = baylee_cards::decks::by_name("Clockwork Beast").unwrap();
    beast.rules = beast.card.map(baylee_view::RulesFace::from);
    let id = beast.id;
    let view = ViewBuilder::new(2).with_battlefield(0, [beast]).build();
    let rows = options(Lang::En, &view, &offering(vec![(id, 1)], vec![]), id);
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert!(!row.mana);
    assert_eq!(row.cost.as_deref(), Some("{X}, {T}"));
    let cut = printed_words(None, &view, id, row).expect("the refill's Oracle sentence");
    assert_eq!(cut.head.as_deref(), Some("{X}, {T}"));
    let words = cut
        .blocks
        .iter()
        .map(baylee_client_core::card_face::TextBlock::text)
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        words,
        "Put up to X +1/+0 counters on this creature. This ability can't cause the total number of +1/+0 counters on this creature to be greater than seven. Activate only during your upkeep."
    );
    let symbols = baylee_client_core::manapip::segments(cut.head.as_deref().unwrap());
    assert_eq!(
        symbols
            .iter()
            .filter(|part| matches!(part, baylee_client_core::manapip::Segment::Symbol(_)))
            .count(),
        2,
        "X and tap must reach the shared Mana-font renderer as symbols"
    );
}

/// A copy's row is the copied card's: its cost, its sentence, read off
/// the card `rules` names and not the card the permanent is.
///
/// A Glasspool Mimic that became a Werefox Bodyguard is offered the
/// Fox's index 1, "{1}{W}, Sacrifice this creature: You gain 2 life."
/// Read against the Mimic's own one-entry list, index 1 is nothing: the
/// row said "Ability 2" and had no sentence, which is the sheet the owner
/// reported as needing special treatment for every card.
#[test]
fn a_copy_offers_the_row_of_the_card_it_copied() {
    let id = ObjectId::new(1, 0);
    let mimic = baylee_cards::decks::by_name("Glasspool Mimic").expect("in the pool");
    let fox = baylee_cards::decks::by_name("Werefox Bodyguard").expect("in the pool");
    let mut copy = token(1, 0, "Werefox Bodyguard", 2, 2);
    copy.card = Some(baylee_view::CardIdentity {
        index: mimic,
        print: baylee_core::ids::PrintRef::new(7),
        face: 0,
    });
    copy.rules = Some(baylee_view::RulesFace { card: fox, face: 0 });
    let view = ViewBuilder::new(2).with_battlefield(0, [copy]).build();

    let rows = options(Lang::En, &view, &offering(vec![(id, 1)], vec![]), id);
    let row = rows.first().expect("the offered ability is a row");
    assert_eq!(row.cost.as_deref(), Some("{1}{W}"), "the Fox's cost");
    let printed = row.printed.expect("the Fox prints this ability");
    let line = baylee_cards::lines::ability_line(fox, 0, 1).expect("a line for it");
    assert_eq!(
        (printed.face, printed.line, printed.of),
        (0, line.line, line.of)
    );

    let words: Vec<String> = crate::cardtext::sentence(None, fox, printed)
        .expect("the Fox's English Oracle")
        .iter()
        .map(|b| b.text().to_string())
        .collect();
    assert_eq!(
        words.join(" "),
        "{1}{W}, Sacrifice this creature: You gain 2 life."
    );
    let cut = printed_words(None, &view, id, row).expect("the Fox's row");
    assert_eq!(
        cut.head.as_deref(),
        Some("{1}{W}, Sacrifice this creature"),
        "the column is the copied card's printed cost"
    );

    // And in German, which is what keying text by card bought: the
    // copy holds no printing of the Fox, and the Fox's German is filed
    // under the Fox. Text of woe #39, read from the catalog 2026-09-24.
    let texts = crate::cardtext::CardTexts::filed(CardTextEntry {
        scryfall_id: String::new(),
        oracle_id: baylee_cards::by_index(fox)
            .expect("in the pool")
            .oracle_id
            .to_string(),
        lang: "de".to_string(),
        layout: "normal".to_string(),
        faces: vec![FaceText {
            name: "Werfuchs-Leibwächterin".to_string(),
            english_name: "Werefox Bodyguard".to_string(),
            printed: Some(FOX_DE.to_string()),
            oracle_text: FOX_DE.to_string(),
            ..FaceText::default()
        }],
    });
    let cut = printed_words(Some(&texts), &view, id, row).expect("the Fox's row");
    assert_eq!(
        cut.head.as_deref(),
        Some("{1}{W}, opfere die Werfuchs-Leibwächterin"),
        "the copy's row is the copied card's German"
    );
}

/// Werefox Bodyguard's German, woe #39.
const FOX_DE: &str = "Aufblitzen\nWenn die Werfuchs-Leibwächterin ins Spiel kommt, schicke bis zu eine andere Nicht-Fuchs-Kreatur deiner Wahl ins Exil, bis die Werfuchs-Leibwächterin das Spiel verlässt.\n{1}{W}, opfere die Werfuchs-Leibwächterin: Du erhältst 2 Lebenspunkte dazu.";

/// The two synthetic indices are offered like any other ability and have
/// to be labelled without asking the card about them.
///
/// `GRANTED_ABILITY` is `u32::MAX`, and the fallback label counts an
/// ability out as `index + 1` — so a permanent under a Chromatic Lantern
/// killed the client outright in a debug build, on the frame the chooser
/// was built. Nothing in the duel-flow ledger reaches this: it drives
/// `Interaction`, not the list of buttons drawn from it.
#[test]
fn a_granted_ability_is_labelled_without_asking_the_card_it_is_not_on() {
    let id = ObjectId::new(1, 0);
    let mut ally = token(1, 0, "Ally", 2, 2);
    ally.grants = vec![baylee_view::GrantSource {
        slot: 0,
        ability: None,
        mana_ability: true,
        mana: None,
        source: None,
        rules: None,
        text: None,
    }];
    let view = ViewBuilder::new(2).with_battlefield(0, [ally]).build();

    // Production cannot be summarized, but the slot explicitly identifies
    // this as a mana ability, so it needs no arming.
    let i = offering(vec![(id, GRANTED_ABILITY)], vec![id]);
    let out = options(Lang::En, &view, &i, id);
    assert_eq!(out.len(), 1, "one granted ability, one button: {out:?}");
    assert_eq!(out[0].label, "Granted ability");
    assert!(out[0].mana, "the engine offered it as a mana ability");
    assert_eq!(
        out[0].action,
        PlayerAction::ActivateAbility {
            source: id,
            ability_index: GRANTED_ABILITY,
        }
    );

    // The same slot explicitly classified as a non-mana ability.
    let mut ally = view.object(id).unwrap().clone();
    ally.grants[0].mana_ability = false;
    let view = ViewBuilder::new(2).with_battlefield(0, [ally]).build();
    let i = offering(vec![(id, GRANTED_ABILITY)], vec![]);
    assert!(
        !options(Lang::En, &view, &i, id)[0].mana,
        "the explicit slot classification determines mana routing"
    );

    // A prepared cast is a cast: never a mana ability, and never
    // "Ability 4294967295". A token links no spell, so it is not named.
    let i = offering(vec![(id, PREPARED_CAST)], vec![]);
    let out = options(Lang::En, &view, &i, id);
    assert_eq!(out[0].label, "");
    assert!(!out[0].mana);
}

/// A Room's unlock is a row of its own (CR 709.5e): named by the door it
/// opens, priced at that half's mana cost, and neither a mana ability
/// nor a position in the card's ability list. Offered for the right half
/// of a Walk-In Closet, it reads "Unlock Forgotten Cellar" for
/// {3}{G}{G}, and the left half's would be the Closet's {2}{G}.
#[test]
fn a_rooms_unlock_is_named_by_its_door_and_costs_that_half() {
    let mut room = crate::registry_printed(1, 0, "Walk-In Closet");
    room.unlocked_doors = Some([false, false]);
    let id = room.id;
    let view = ViewBuilder::new(2).with_battlefield(0, [room]).build();
    let i = offering(
        vec![
            (id, baylee_engine::choice::unlock_door(0)),
            (id, baylee_engine::choice::unlock_door(1)),
        ],
        vec![],
    );
    let out = options(Lang::En, &view, &i, id);
    let rows: Vec<(&str, Option<&str>, bool, Option<u32>)> = out
        .iter()
        .map(|o| {
            (
                o.label.as_str(),
                o.cost.as_deref(),
                o.mana,
                o.printed_index(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("Unlock Walk-In Closet", Some("{2}{G}"), false, None),
            ("Unlock Forgotten Cellar", Some("{3}{G}{G}"), false, None),
        ]
    );
    assert_eq!(
        options(Lang::De, &view, &i, id)[1].label,
        "Forgotten Cellar entsperren",
        "the door by its English name, as a named card is"
    );
}

/// Chromatic Lantern, msc #428, read from the catalog 2026-09-24.
const LANTERN_DE: &str = "Länder, die du kontrollierst, haben „{T}: Erzeuge ein Mana beliebiger Farbe.\"\n{T}: Erzeuge ein Mana beliebiger Farbe.";

/// A granted ability's row is its grantor's name over the sentence that
/// grants it, in the player's language where the grantor's text has
/// arrived and in English before (#212). It said "Granted ability",
/// which is this client's phrase and not the card's.
///
/// The Lantern's line is the one the host's own test pins
/// (`baylee_gamehost` view, line 0 of 2). Offered here as a grant that
/// is not a mana ability, so the row is a row and not a pip: the words
/// are what is under test, not the mana routing.
#[test]
fn a_granted_ability_is_the_sentence_of_the_card_that_granted_it() {
    let id = ObjectId::new(1, 0);
    let lantern = baylee_cards::decks::by_name("Chromatic Lantern").expect("in the pool");
    let mut land = crate::registry_printed(1, 0, "Island");
    land.grants = vec![baylee_view::GrantSource {
        slot: 0,
        ability: None,
        mana_ability: false,
        mana: None,
        source: Some(ObjectId::new(2, 0)),
        rules: Some(baylee_view::RulesFace {
            card: lantern,
            face: 0,
        }),
        text: Some(baylee_view::StackText {
            face: 0,
            line: 0,
            of: 2,
        }),
    }];
    let view = ViewBuilder::new(2).with_battlefield(0, [land]).build();

    let out = options(
        Lang::En,
        &view,
        &offering(vec![(id, GRANTED_ABILITY)], vec![]),
        id,
    );
    assert_eq!(out[0].label, "Chromatic Lantern");
    assert_eq!(out[0].printed_index(), None, "the Island prints no grant");

    let english = crate::cardtext::CardTexts::default();
    let (blocks, said) = grant_words(&english, &view, id, &out[0]).expect("words");
    assert_eq!(said, crate::cardtext::Said::Oracle);
    assert_eq!(
        blocks,
        vec![
            TextBlock::Rules("Chromatic Lantern".to_string()),
            TextBlock::Rules(
                "Lands you control have \"{T}: Add one mana of any color.\"".to_string()
            ),
        ]
    );

    let german = crate::cardtext::CardTexts::filed(CardTextEntry {
        scryfall_id: String::new(),
        oracle_id: baylee_cards::by_index(lantern)
            .expect("in the pool")
            .oracle_id
            .to_string(),
        lang: "de".to_string(),
        layout: "normal".to_string(),
        faces: vec![FaceText {
            name: "Chromatische Laterne".to_string(),
            english_name: "Chromatic Lantern".to_string(),
            printed: Some(LANTERN_DE.to_string()),
            oracle_text: LANTERN_DE.to_string(),
            ..FaceText::default()
        }],
    });
    let (blocks, said) = grant_words(&german, &view, id, &out[0]).expect("words");
    assert_eq!(said, crate::cardtext::Said::Localized);
    assert_eq!(
        blocks,
        vec![
            TextBlock::Rules("Chromatische Laterne".to_string()),
            TextBlock::Rules(
                "Länder, die du kontrollierst, haben „{T}: Erzeuge ein Mana beliebiger Farbe.\""
                    .to_string()
            ),
        ]
    );

    // The words are the grant's row alone, and the slot's own: a second
    // slot the view says nothing about is the client's label again.
    let printed = AbilityOption {
        action: PlayerAction::ActivateAbility {
            source: id,
            ability_index: 0,
        },
        ..out[0].clone()
    };
    assert_eq!(grant_words(&german, &view, id, &printed), None);
    let second = baylee_engine::choice::granted_ability(1);
    let out = options(Lang::En, &view, &offering(vec![(id, second)], vec![]), id);
    assert_eq!(out[0].label, "Granted ability");
    assert_eq!(grant_words(&german, &view, id, &out[0]), None);
}

/// Where the view names the grantor and no sentence, the row is the
/// grantor's name alone; where it names no grantor, the row is the
/// client's label, and nothing is read off a card the seat may not see.
#[test]
fn a_grant_the_view_cannot_place_draws_no_sentence() {
    let id = ObjectId::new(1, 0);
    let opt = baylee_cards::decks::by_name("Opt").expect("in the pool");
    let mut land = crate::registry_printed(1, 0, "Island");
    land.grants = vec![baylee_view::GrantSource {
        slot: 0,
        ability: None,
        mana_ability: false,
        mana: None,
        source: Some(ObjectId::new(2, 0)),
        rules: Some(baylee_view::RulesFace { card: opt, face: 0 }),
        text: None,
    }];
    let view = ViewBuilder::new(2)
        .with_battlefield(0, [land.clone()])
        .build();
    let i = offering(vec![(id, GRANTED_ABILITY)], vec![]);
    let out = options(Lang::En, &view, &i, id);
    assert_eq!(out[0].label, "Opt");
    let english = crate::cardtext::CardTexts::default();
    assert_eq!(
        grant_words(&english, &view, id, &out[0]),
        Some((
            vec![TextBlock::Rules("Opt".to_string())],
            crate::cardtext::Said::Oracle
        ))
    );

    land.grants = vec![baylee_view::GrantSource::default()];
    let view = ViewBuilder::new(2).with_battlefield(0, [land]).build();
    let out = options(Lang::En, &view, &i, id);
    assert_eq!(out[0].label, "Granted ability");
    assert_eq!(grant_words(&english, &view, id, &out[0]), None);
}

/// Demonic Tutor, cmm #150, read from the catalog 2026-09-24.
const TUTOR_DE: &str =
    "Durchsuche deine Bibliothek nach einer Karte, nimm sie auf deine Hand und mische danach.";

/// A prepared cast is named by the spell it casts, in the player's
/// language where its text has arrived and in English before, and costs
/// what that spell prints. It said "Cast the prepared spell", which is
/// this client's sentence and not the card's.
#[test]
fn a_prepared_cast_is_the_spell_it_casts() {
    let id = ObjectId::new(1, 0);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, [crate::registry_printed(1, 0, "Emeritus of Woe")])
        .build();
    let tutor = baylee_cards::decks::by_name("Demonic Tutor").expect("in the pool");
    assert_eq!(
        prepared_of(&view, id),
        Some(baylee_view::RulesFace {
            card: tutor,
            face: 0
        })
    );

    let i = offering(vec![(id, PREPARED_CAST)], vec![]);
    let out = options(Lang::En, &view, &i, id);
    assert_eq!(out[0].label, "Demonic Tutor");
    assert_eq!(out[0].cost.as_deref(), Some("{1}{B}"));
    assert_eq!(out[0].printed_index(), None, "a cast prints no ability");

    let english = crate::cardtext::CardTexts::default();
    let (blocks, said) = prepared_words(&english, &view, id, &out[0]).expect("words");
    assert_eq!(said, crate::cardtext::Said::Oracle);
    assert_eq!(
        blocks,
        vec![
            TextBlock::Rules("Demonic Tutor".to_string()),
            TextBlock::Rules(
                "Search your library for a card, put that card into your hand, then shuffle."
                    .to_string()
            ),
        ]
    );

    let german = crate::cardtext::CardTexts::filed(CardTextEntry {
        scryfall_id: String::new(),
        oracle_id: baylee_cards::by_index(tutor)
            .expect("in the pool")
            .oracle_id
            .to_string(),
        lang: "de".to_string(),
        layout: "normal".to_string(),
        faces: vec![FaceText {
            name: "Dämonischer Lehrmeister".to_string(),
            english_name: "Demonic Tutor".to_string(),
            printed: Some(TUTOR_DE.to_string()),
            oracle_text: TUTOR_DE.to_string(),
            ..FaceText::default()
        }],
    });
    let (blocks, said) = prepared_words(&german, &view, id, &out[0]).expect("words");
    assert_eq!(said, crate::cardtext::Said::Localized);
    assert_eq!(
        blocks,
        vec![
            TextBlock::Rules("Dämonischer Lehrmeister".to_string()),
            TextBlock::Rules(TUTOR_DE.to_string()),
        ]
    );

    // The words are the prepared row's alone.
    let other = AbilityOption {
        action: PlayerAction::ActivateAbility {
            source: id,
            ability_index: 0,
        },
        ..out[0].clone()
    };
    assert_eq!(prepared_words(&german, &view, id, &other), None);
}
#[test]
fn an_inset_prepare_spell_uses_its_own_name_price_and_frozen_text() {
    let id = ObjectId::new(1, 0);
    let chronologist = baylee_core::generated::index::FATEHOLD_CHRONOLOGIST;
    let mut creature = token(1, 0, "Frog copy", 1, 1);
    creature.prepared_spell = Some(baylee_view::RulesFace {
        card: chronologist,
        face: 1,
    });
    // No backing card or printed rules are needed: the host describes the
    // existing exiled copy even after this permanent copies something else.
    let view = ViewBuilder::new(2).with_battlefield(0, [creature]).build();
    let interaction = offering(vec![(id, PREPARED_CAST)], vec![]);
    let out = options(Lang::En, &view, &interaction, id);
    assert_eq!(out[0].label, "Peer Review");
    assert_eq!(out[0].cost.as_deref(), Some("{2}{W/U}"));
    let (blocks, _) =
        prepared_words(&crate::cardtext::CardTexts::default(), &view, id, &out[0]).unwrap();
    assert_eq!(blocks[0], TextBlock::Rules("Peer Review".into()));
    assert!(view.cards().any(|card| card == chronologist));
}

/// A Chromatic Lantern's land, as it actually arrives: the engine offers
/// the grant *and* the view says what it makes. Both halves of the
/// chooser then have something to say about the same tap, and for one
/// commit they both said it — the player got two buttons that did the
/// same thing, one of them labelled "Granted ability".
#[test]
fn a_granted_mana_ability_is_one_button_and_not_two() {
    let id = ObjectId::new(1, 0);
    let mut land = token(1, 0, "Mountain", 0, 0);
    land.types = baylee_core::types::TypeSet::LAND;
    land.power = None;
    land.toughness = None;
    land.granted_mana = Some(baylee_view::GrantedMana {
        slot: 0,
        colors: vec![ManaColor::Red],
        amount: 1,
    });
    let view = ViewBuilder::new(2).with_battlefield(0, [land]).build();

    let i = offering(vec![(id, GRANTED_ABILITY)], vec![id]);
    let out = options(Lang::En, &view, &i, id);
    assert_eq!(out.len(), 1, "one tap, one button: {out:?}");
    assert!(out[0].mana);
}

/// Urza's Saga is granted two abilities and only one of them makes mana.
/// Reading "is this a mana ability" off `legal.mana_abilities` — which
/// holds the *permanent*, not the slot — marked the Construct ability as
/// one tap, which would have sent it with no arming and no confirmation.
#[test]
fn a_second_grant_is_not_a_mana_ability_because_the_first_one_is() {
    let id = ObjectId::new(1, 0);
    let mut saga = token(1, 0, "Urza's Saga", 0, 0);
    saga.types = baylee_core::types::TypeSet::LAND;
    saga.power = None;
    saga.toughness = None;
    saga.granted_mana = Some(baylee_view::GrantedMana {
        slot: 0,
        colors: vec![ManaColor::Colorless],
        amount: 1,
    });
    let view = ViewBuilder::new(2).with_battlefield(0, [saga]).build();

    let i = offering(
        vec![
            (id, baylee_engine::choice::granted_ability(0)),
            (id, baylee_engine::choice::granted_ability(1)),
        ],
        vec![id],
    );
    let out = options(Lang::En, &view, &i, id);
    assert_eq!(out.len(), 2, "two grants, two buttons: {out:?}");
    let construct = out
        .iter()
        .find(|o| {
            o.action
                == PlayerAction::ActivateAbility {
                    source: id,
                    ability_index: baylee_engine::choice::granted_ability(1),
                }
        })
        .expect("chapter II is offered");
    assert!(
        !construct.mana,
        "the permanent has a granted mana ability; this is not it"
    );
}

/// The same fallback with two grants on the permanent, which is where it
/// stops being sound: `legal.mana_abilities` names the permanent, so it
/// cannot say *which* of them makes the mana. Reading it anyway would
/// have fired the other one on a single tap, with no arming and no way
/// back — and arming a mana ability by mistake only ever costs a tap.
#[test]
fn an_unreadable_grant_is_not_a_mana_ability_when_there_are_two_of_them() {
    let id = ObjectId::new(1, 0);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, [token(1, 0, "Ally", 2, 2)])
        .build();
    let i = offering(
        vec![
            (id, baylee_engine::choice::granted_ability(0)),
            (id, baylee_engine::choice::granted_ability(1)),
        ],
        vec![id],
    );
    let out = options(Lang::En, &view, &i, id);
    assert_eq!(out.len(), 2, "two grants, two buttons: {out:?}");
    assert!(
        out.iter().all(|o| !o.mana),
        "neither can be claimed as the mana one: {out:?}"
    );
}

/// The other way that fallback goes wrong, and the one a real card
/// reaches first: a *basic land* is named in `legal.mana_abilities` for
/// its own intrinsic mana (CR 305.6), whatever it was granted. Counting
/// that entry as evidence about the grant marks a non-mana grant as one
/// tap, and it fires with no arming.
///
/// No card in the pool grants a land a non-mana ability today — Lantern,
/// Saga and Guide all grant mana — so this guards the reading rather
/// than a card, which is why it is written from both sides.
#[test]
fn a_basics_own_mana_says_nothing_about_a_grant_that_cannot_be_read() {
    use baylee_core::generated::subtypes::land;
    let id = ObjectId::new(1, 0);
    let mut mountain = token(1, 0, "Mountain", 0, 0);
    mountain.types = baylee_core::types::TypeSet::LAND;
    mountain.subtypes = baylee_core::types::SubtypeSet::from_slice(&[land::MOUNTAIN]);
    mountain.power = None;
    mountain.toughness = None;
    // The view could not reduce the grant to "n mana of these colours".
    mountain.granted_mana = None;
    mountain.grants = vec![baylee_view::GrantSource {
        slot: 0,
        ability: None,
        mana_ability: false,
        mana: None,
        source: None,
        rules: None,
        text: None,
    }];

    // The land really does have two things to do — its own `{R}` and the
    // grant — so both are drawn. What matters is which of them is one
    // tap: the intrinsic is, the unreadable grant is not.
    let granted = |legal: Vec<ObjectId>, mana_ability| {
        let mut mountain = mountain.clone();
        mountain.grants[0].mana_ability = mana_ability;
        let view = ViewBuilder::new(2).with_battlefield(0, [mountain]).build();
        let i = offering(vec![(id, GRANTED_ABILITY)], legal);
        let out = options(Lang::En, &view, &i, id);
        assert_eq!(out.len(), 2, "the intrinsic and the grant: {out:?}");
        assert!(
            out.iter()
                .any(|o| o.mana && o.action == PlayerAction::ActivateManaAbility { source: id }),
            "the Mountain still taps for `{{R}}`: {out:?}"
        );
        out.into_iter()
            .find(|o| {
                o.action
                    == PlayerAction::ActivateAbility {
                        source: id,
                        ability_index: GRANTED_ABILITY,
                    }
            })
            .expect("the grant is offered")
    };

    // Named once, and its own `{R}` is what that entry is.
    assert!(
        !granted(vec![id], false).mana,
        "the Mountain's own mana is not evidence about the grant"
    );

    // The grant is explicitly classified as mana, independently of the
    // land's intrinsic mana offer.
    assert!(
        granted(vec![id, id], true).mana,
        "the grant has its own explicit mana classification"
    );
}

/// The card the whole restricted-mana path was reported broken on.
///
/// The three shapes a mana choice is written in, and why there are three.
///
/// The owner reported "Tap for WUBRG" as ugly, and it was two faults in
/// one string: bare letters where the interface draws symbols everywhere
/// else, and five of them in a row where the card itself says three
/// words. A short choice is the case that keeps its symbols — two discs
/// and a conjunction read at a glance.
#[test]
fn a_mana_choice_is_written_as_symbols_until_it_is_every_colour() {
    use baylee_core::mana::ManaColor::{Black, Blue, Green, Red, White};
    assert_eq!(mana_choice(Lang::En, &[Green], 1), "{G}");
    assert_eq!(
        mana_choice(Lang::En, &[Green], 3),
        "{G}{G}{G}",
        "a source that makes three of one colour says so three times",
    );
    assert_eq!(mana_choice(Lang::En, &[Blue, Black], 1), "{U} or {B}");
    assert_eq!(
        mana_choice(Lang::En, &[White, Blue, Black], 1),
        "{W}, {U} or {B}",
    );
    assert_eq!(
        mana_choice(Lang::En, &[White, Blue, Black, Red, Green], 1),
        "any color",
        "five discs is not a symbol, it is arithmetic",
    );
    assert_eq!(
        mana_choice(Lang::De, &[White, Blue, Black, Red, Green], 1),
        "beliebige Farbe",
    );
    assert_eq!(mana_choice(Lang::De, &[Blue, Black], 1), "{U} oder {B}");
}

/// Jasmine Dragon Tea Shop prints two mana abilities: `{T}: Add {C}` and
/// `{T}: Add one mana of any color`, the second spendable only on Allies.
/// `manasources` reduces a permanent to the one tap it can read and
/// `simple_mana` refuses restricted mana on purpose, so the first won the
/// mana slot and the second fell through to the cost label — which is
/// `{T}`, the same tap, saying nothing about what it makes. Two buttons,
/// one of them unreadable, and the unreadable one is the reason the land
/// is in the deck.
///
/// The partition in [`pour_out`] changed the *shape* of the answer and
/// not the claim: the `{C}` is a header pip now, because a pip is exactly
/// what a colour with a number behind it wants, and the restricted tap
/// keeps its sentence because a pip cannot say "and only on Ally spells".
/// Still two offers, still tellable apart.
#[test]
fn a_restricted_mana_ability_says_what_it_makes() {
    let id = ObjectId::new(1, 0);
    let mut land = crate::registry_printed(1, 0, "Jasmine Dragon Tea Shop");
    land.types = baylee_core::types::TypeSet::LAND;
    land.power = None;
    land.toughness = None;
    let view = ViewBuilder::new(2).with_battlefield(0, [land]).build();

    let i = offering(vec![(id, 0), (id, 1)], vec![]);
    let out = options(Lang::En, &view, &i, id);
    let labels: Vec<&str> = out.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(
        labels,
        vec!["{C}", "Tap for any color (restricted)"],
        "both taps have to be tellable apart: {out:?}"
    );
    assert!(
        out[0].pour.is_some(),
        "the colourless tap is the header pip"
    );
    assert!(
        out[1].pour.is_none(),
        "and the restricted one is a written row, because a pip cannot \
             carry what it may be spent on"
    );
    assert_eq!(Split::of(&out), Split { pips: 1, rows: 1 });
    assert!(
        out.iter().all(|o| o.mana),
        "both are mana abilities (CR 605.1), so both stay one tap"
    );
    assert_eq!(
        out[1].action,
        PlayerAction::ActivateAbility {
            source: id,
            ability_index: 1,
        },
        "and the second button sends the second ability"
    );
}
