use super::*;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{ArrangePrompt, CastModeDesc, CastModeKind};

#[test]
fn a_colour_row_is_a_symbol_and_no_word() {
    let rows = options(
        &Prompt::ChooseColor {
            options: vec![ManaColor::Blue, ManaColor::Black],
        },
        Lang::En,
        None,
        "",
        FaceNames::default(),
    )
    .expect("a colour choice has rows");
    assert_eq!(rows.len(), 2, "one row per offered colour, in engine order");
    assert!(rows.iter().all(|r| r.pip.is_some() && r.label.is_empty()));
    assert_ne!(rows[0].pip, rows[1].pip, "two colours, two symbols");
}

#[test]
fn a_seat_row_says_the_name_the_table_shows() {
    let statics = GameStatic {
        decision_secs: None,
        reconnect_secs: None,
        view_version: baylee_view::VIEW_VERSION,
        game_id: "g".into(),
        your_seat: PlayerId::new(0),
        seats: vec![baylee_view::SeatIdentity {
            player: PlayerId::new(1),
            display_name: "House AI".into(),
            is_ai: true,
            away: false,
            team: None,
        }],
        prints: vec![],
    };
    let rows = options(
        &Prompt::ChoosePlayer {
            options: vec![PlayerId::new(1), PlayerId::new(7)],
        },
        Lang::En,
        Some(&statics),
        "",
        FaceNames::default(),
    )
    .expect("a player choice has rows");
    assert_eq!(rows[0].label, "House AI");
    // A seat the statics do not describe still gets a row: a chooser that
    // dropped it would offer fewer answers than the engine did, and the
    // index of everything after it would name the wrong seat. It is
    // numbered in the words the rest of the interface numbers a seat in,
    // which this row used to spell `#7`.
    assert_eq!(rows[1].label, "Seat 7");
}

#[test]
fn player_choices_use_the_same_localized_house_name_as_the_table() {
    let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    let mut statics = GameStatic {
        decision_secs: None,
        reconnect_secs: None,
        view_version: baylee_view::VIEW_VERSION,
        game_id: "g".into(),
        your_seat: PlayerId::new(0),
        seats: vec![baylee_view::SeatIdentity {
            player: PlayerId::new(1),
            display_name: String::new(),
            is_ai: true,
            away: false,
            team: None,
        }],
        prints: vec![],
    };
    for (name, expected) in [
        ("House AI", "Haus-KI"),
        ("Solide 1", "Solide 1"),
        ("Ada", "Ada"),
    ] {
        statics.seats[0].display_name = name.into();
        let rows = options(
            &Prompt::ChoosePlayer {
                options: vec![statics.seats[0].player],
            },
            Lang::De,
            Some(&statics),
            "",
            FaceNames {
                view: Some(&view),
                texts: None,
            },
        )
        .unwrap();
        assert_eq!(rows[0].label, expected);
        assert_eq!(
            rows[0].label,
            crate::hud::seatbar::called(
                Lang::De,
                &view,
                Some(&statics),
                statics.seats[0].player,
                baylee_client_core::board::SeatRole::House
            )
        );
    }
}

/// Raging River's label is a choice between the defender's "left" and
/// "right" piles, and the rows say so; Fact or Fiction's keep numbers.
#[test]
fn a_river_label_s_rows_are_left_and_right() {
    let piles = vec![vec![], vec![ObjectId::new(7, 0)]];
    let rows = |label, lang| {
        options(
            &Prompt::ChoosePile {
                piles: piles.clone(),
                label,
            },
            lang,
            None,
            "",
            FaceNames::default(),
        )
        .expect("a pile choice has rows")
        .into_iter()
        .map(|row| row.label)
        .collect::<Vec<_>>()
    };
    let attacker = Some(ObjectId::new(2, 0));
    assert_eq!(rows(attacker, Lang::En), ["Left: no cards", "Right: ?"]);
    assert_eq!(
        rows(attacker, Lang::De),
        ["Links: keine Karten", "Rechts: ?"]
    );
    assert_eq!(rows(None, Lang::En), ["Pile 1: no cards", "Pile 2: ?"]);
}

#[test]
fn a_cast_option_of_an_unnamed_card_is_its_cost() {
    let desc = |kind, cost: &str| CastModeDesc {
        index: 0,
        kind,
        cost: ManaCost::try_parse(cost).expect("a cost"),
    };
    let rows = options(
        &Prompt::CastMode {
            object: ObjectId::new(1, 0),
            options: vec![
                desc(CastModeKind::Normal, "{2}{U}"),
                desc(CastModeKind::Mode(1), "{U}"),
            ],
        },
        Lang::En,
        None,
        "",
        FaceNames::default(),
    )
    .expect("a cast choice has rows");
    // No view names the card, so there is no sentence to draw: each
    // row is its cost, and nothing this client composed stands in.
    assert_eq!(rows[0].label, "", "the normal way is its cost");
    assert_eq!(
        rows[1].label, "Mode 2",
        "a mode with no sentence keeps its number, one-based as printed"
    );
    assert_eq!(rows[0].cost, ManaCost::try_parse("{2}{U}").ok());
    assert_eq!(rows[1].cost, ManaCost::try_parse("{U}").ok());
}

/// A normal cast for nothing is a `{0}`, not an empty row: the cost is
/// all the normal way draws. An alternative cost for no mana keeps no
/// pips, because its sentence says what it charges.
#[test]
fn a_free_normal_cast_says_zero() {
    let rows = options(
        &Prompt::CastMode {
            object: ObjectId::new(1, 0),
            options: vec![
                CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Normal,
                    cost: ManaCost::ZERO,
                },
                CastModeDesc {
                    index: 1,
                    kind: CastModeKind::Alternative(0),
                    cost: ManaCost::ZERO,
                },
            ],
        },
        Lang::En,
        None,
        "",
        FaceNames::default(),
    )
    .expect("a cast choice has rows");
    let zero = rows[0].cost.expect("a free normal cast still draws a cost");
    assert!(!zero.is_empty());
    assert_eq!(zero.to_string(), "{0}");
    assert_eq!(rows[1].cost, None);
}

/// Brightclimb Pathway, in the hand, as one object with two land faces.
///
/// The hand and not the battlefield because that is where the question is
/// asked from — and because `PlayerView::object` does not look in the
/// hand, which is the whole reason [`crate::face::face_name`] searches it
/// first.
fn pathway_in_hand() -> (baylee_view::PlayerView, ObjectId) {
    let def = baylee_cards::by_oracle_id("1c633e02-95ef-445e-b4e0-fbfbc5ed9cc9")
        .expect("Brightclimb Pathway is in the pool");
    let id = ObjectId::new(11, 0);
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.hand = vec![baylee_view::HandObject {
        id,
        card: baylee_view::CardIdentity {
            index: def.index,
            print: baylee_core::ids::PrintRef::new(0),
            face: 0,
        },
        name: def.faces[0].name.to_string(),
        mana_value: 0,
        colors: baylee_core::color::ColorSet::default(),
        types: baylee_core::types::TypeSet::LAND,
        commander: false,
    }];
    (view, id)
}

/// The owner's AE11: two buttons that said the same thing.
///
/// A pathway's two options are the same kind at the same empty cost, so
/// every column a row draws was identical — the label, the missing pip,
/// the missing cost — and the choice was made blind.
#[test]
fn a_pathways_two_land_faces_are_two_different_buttons() {
    let (view, object) = pathway_in_hand();
    let desc = |face: usize| CastModeDesc {
        index: u32::try_from(face).expect("a face index"),
        kind: CastModeKind::PlayLandFace(face),
        cost: ManaCost::ZERO,
    };
    let rows = options(
        &Prompt::CastMode {
            object,
            options: vec![desc(0), desc(1)],
        },
        Lang::En,
        None,
        "",
        FaceNames {
            view: Some(&view),
            texts: None,
        },
    )
    .expect("a cast choice has rows");
    assert_eq!(rows[0].label, "Brightclimb Pathway");
    assert_eq!(rows[1].label, "Grimclimb Pathway");
    assert!(
        rows.iter().all(|r| r.cost.is_none()),
        "a land face costs nothing, which is why the label has to carry it"
    );
}

/// The rung under it: nothing to look a name up with is not a blank row.
///
/// A view that has not arrived, an object the seat cannot place, a card
/// the registry does not have — each leaves the phrase that says what kind
/// of option this is, which is what every row said before.
#[test]
fn a_face_that_cannot_be_named_keeps_the_words_it_had() {
    let rows = options(
        &Prompt::CastMode {
            object: ObjectId::new(11, 0),
            options: vec![CastModeDesc {
                index: 0,
                kind: CastModeKind::PlayLandFace(1),
                cost: ManaCost::ZERO,
            }],
        },
        Lang::En,
        None,
        "",
        FaceNames::default(),
    )
    .expect("a cast choice has rows");
    assert_eq!(rows[0].label, Phrase::CastLandFace.text(Lang::En));
}

/// The prompts this module does not answer: they are clicks on the board
/// or the table, and a chooser row drawn for one would be a second, wrong
/// way to answer.
#[test]
fn prompts_that_are_not_indexed_choices_have_no_rows() {
    assert!(
        options(
            &Prompt::Arrange {
                reason: ArrangePrompt::Order,
                onto: None,
            },
            Lang::En,
            None,
            "",
            FaceNames::default(),
        )
        .is_none()
    );
    assert!(
        options(
            &Prompt::ChooseTargets {
                min: 1,
                max: 1,
                reason: baylee_engine::choice::TargetPrompt::Targets,
            },
            Lang::En,
            None,
            "",
            FaceNames::default(),
        )
        .is_none()
    );
}

/// Every subtype the generated table knows, which is what the engine
/// hands over for a Cavern of Souls.
fn all_subtypes() -> Vec<SubtypeId> {
    (0..400).map(SubtypeId::new).collect()
}

#[test]
fn an_unfiltered_type_list_is_cut_to_what_the_bar_holds() {
    let rows = options(
        &Prompt::ChooseSubtype {
            options: all_subtypes(),
        },
        Lang::En,
        None,
        "",
        FaceNames::default(),
    )
    .expect("a subtype choice has rows");
    assert_eq!(
        rows.len(),
        SUBTYPE_ROWS,
        "the bar holds twelve, not four hundred"
    );
    assert!(
        rows.windows(2)
            .all(|pair| pair[0].label.to_lowercase() <= pair[1].label.to_lowercase())
    );
    for row in rows {
        assert_eq!(
            subtypes::name(all_subtypes()[row.index]),
            Some(row.label.as_str())
        );
    }
}

/// The property the whole `index` field exists for: once a filter has
/// hidden part of the list, a row's position is no longer its answer.
#[test]
fn a_filtered_row_still_carries_the_engine_s_own_index() {
    let all = all_subtypes();
    let elf = all
        .iter()
        .position(|id| subtypes::name(*id) == Some("Elf"))
        .expect("the generated table knows about elves");
    let rows = options(
        &Prompt::ChooseSubtype { options: all },
        Lang::En,
        None,
        "elf",
        FaceNames::default(),
    )
    .expect("a subtype choice has rows");
    assert_eq!(rows[0].label, "Elf", "a prefix match, case-insensitively");
    assert_ne!(rows[0].index, 0, "an elf is not the first creature type");
    assert_eq!(rows[0].index, elf, "the row answers the engine's option");
}

#[test]
fn creature_types_search_in_both_languages_and_keep_the_original_answer() {
    let offered = vec![
        subtypes::creature::ELF,
        subtypes::creature::ALLY,
        subtypes::creature::WIZARD,
    ];
    for needle in ["verb", "VERBÜ", "ally"] {
        let rows = subtype_rows(&offered, needle, Lang::De);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].index, 1);
        assert_eq!(rows[0].label, "Verbündeter");
    }
    let rows = subtype_rows(&offered, "zaub", Lang::De);
    assert_eq!((rows[0].index, rows[0].label.as_str()), (2, "Zauberer"));
    assert!(subtype_rows(&offered, "verb", Lang::En).is_empty());
    assert_eq!(subtype_rows(&offered, "ally", Lang::En)[0].label, "Ally");
}

/// A card name is answered out of the pool: typing a word of the name
/// offers it, the row stands for its card and face, and picking it is
/// what the model sends.
#[test]
fn a_card_name_is_found_by_any_word_of_it_and_sent_as_its_card() {
    let needle = baylee_cards::decks::by_name("Pithing Needle").expect("in the pool");
    let rows = options(
        &Prompt::ChooseCardName {
            types: baylee_core::types::TypeSet::EMPTY,
        },
        Lang::En,
        None,
        "needle",
        FaceNames::default(),
    )
    .expect("a card name is a chooser");
    let row = rows
        .iter()
        .find(|row| row.label == "Pithing Needle")
        .expect("a word of the name finds it, not only its start");
    assert_eq!(card_name_at(row.index), Some((needle, 0)));
    assert!(rows.len() <= SUBTYPE_ROWS);

    let pending = baylee_engine::choice::Pending::ChooseCardName {
        types: baylee_core::types::TypeSet::EMPTY,
        player: baylee_core::ids::PlayerId::new(0),
    };
    let mut interaction = Interaction::new(pending, baylee_core::ids::PlayerId::new(0));
    assert!(pick(&mut interaction, row.index));
    assert_eq!(picked(&interaction), Some(row.index));
    assert_eq!(
        interaction.confirm(),
        Some(baylee_engine::choice::PlayerAction::ChooseCardName {
            card: needle,
            face: 0
        })
    );
    assert!(
        !pick(&mut interaction, usize::MAX),
        "a row past the pool answers nothing"
    );
}

/// A back face's name is a name of its own (CR 201.4d), and a name that
/// begins with what was typed comes before one with a later word that does.
#[test]
fn a_back_face_is_named_and_a_leading_match_comes_first() {
    let rebirth = baylee_cards::decks::by_name("Malakir Rebirth").expect("in the pool");
    let rows = card_name_rows("malakir mire", baylee_core::types::TypeSet::EMPTY);
    assert_eq!(
        rows.first().and_then(|row| card_name_at(row.index)),
        Some((rebirth, 1)),
        "Malakir Mire is the back of Malakir Rebirth"
    );
    let rows = card_name_rows("pith", baylee_core::types::TypeSet::EMPTY);
    assert_eq!(
        rows.first().map(|row| row.label.as_str()),
        Some("Pithing Needle")
    );
    assert!(card_name_rows("zzzzqq", baylee_core::types::TypeSet::EMPTY).is_empty());
}

#[test]
fn a_filter_that_matches_nothing_offers_nothing() {
    let rows = options(
        &Prompt::ChooseSubtype {
            options: all_subtypes(),
        },
        Lang::En,
        None,
        "zzzz",
        FaceNames::default(),
    )
    .expect("a subtype choice still answers, with no rows");
    assert!(
        rows.is_empty(),
        "nothing may be pickable that does not match"
    );
}

#[test]
fn a_land_name_choice_preserves_the_catalog_index() {
    use baylee_core::types::TypeSet;
    assert!(card_name_rows("sol ring", TypeSet::LAND).is_empty());
    let rows = card_name_rows("wasteland", TypeSet::LAND);
    let row = rows.iter().find(|row| row.label == "Wasteland").unwrap();
    assert_eq!(
        card_name_at(row.index),
        Some((baylee_core::generated::index::WASTELAND, 0))
    );
}

#[test]
fn primal_clay_forms_have_descriptive_labels() {
    let (view, texts, object) = asking_about(
        "dccf2ca8-8c87-41c9-8373-351859396d05",
        "en",
        "As this creature enters, it becomes your choice of a 3/3 artifact creature, a 2/2 artifact creature with flying, or a 1/6 Wall artifact creature with defender in addition to its other types.",
    );
    let rows = cast_rows(
        object,
        &[
            CastModeKind::Mode(0),
            CastModeKind::Mode(1),
            CastModeKind::Mode(2),
        ],
        FaceNames {
            view: Some(&view),
            texts: Some(&texts),
        },
        Lang::En,
    );
    assert_eq!(
        rows.iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
        [
            "3/3 artifact creature",
            "2/2 artifact creature with flying",
            "1/6 Wall artifact creature with defender",
        ]
    );
}

/// One card in the hand with one printing's text filed against it.
///
/// The hand for the reason `pathway_in_hand` gives — it is where a cast
/// question is asked from, and the one zone `PlayerView::object` does
/// not answer for.
fn asking_about(
    oracle_id: &str,
    lang: &str,
    oracle: &str,
) -> (
    baylee_view::PlayerView,
    crate::cardtext::CardTexts,
    ObjectId,
) {
    use baylee_client_core::card_face::{CardTextEntry, FaceText};
    let def = baylee_cards::by_oracle_id(oracle_id).expect("the card is in the pool");
    let print = baylee_core::ids::PrintRef::new(3);
    let id = ObjectId::new(21, 0);
    let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
    view.hand = vec![baylee_view::HandObject {
        id,
        card: baylee_view::CardIdentity {
            index: def.index,
            print,
            face: 0,
        },
        name: def.faces[0].name.to_string(),
        mana_value: 0,
        colors: baylee_core::color::ColorSet::default(),
        types: def.faces[0].types,
        commander: false,
    }];
    // Served the way the gateway serves it: `printed` is the asked
    // language's text, and never English.
    let texts = crate::cardtext::CardTexts::filed(CardTextEntry {
        oracle_id: oracle_id.to_string(),
        layout: "normal".to_string(),
        scryfall_id: "x".to_string(),
        lang: lang.to_string(),
        faces: vec![FaceText {
            printed: (lang != "en").then(|| oracle.to_string()),
            name: def.faces[0].name.to_string(),
            english_name: def.faces[0].name.to_string(),
            type_line: String::new(),
            oracle_text: oracle.to_string(),
            mana_cost: String::new(),
        }],
    });
    (view, texts, id)
}

/// The rows of a cast chooser over `kinds`, all at the same cost.
fn cast_rows(
    object: ObjectId,
    kinds: &[CastModeKind],
    names: FaceNames<'_>,
    lang: Lang,
) -> Vec<ChoiceOption> {
    options(
        &Prompt::CastMode {
            object,
            options: kinds
                .iter()
                .enumerate()
                .map(|(i, kind)| CastModeDesc {
                    index: u32::try_from(i).expect("a small list"),
                    kind: *kind,
                    cost: ManaCost::ZERO,
                })
                .collect(),
        },
        lang,
        None,
        "",
        names,
    )
    .expect("a cast choice has rows")
}

/// The owner's AM1: a mode row says what the mode *does*.
///
/// Sheoldred's Edict prints a header and three bullets, and the engine
/// offers three modes carrying nothing but their own index — so the
/// chooser drew "Mode 1", "Mode 2", "Mode 3" and the player picked
/// between three numbers. The bullet is the list's mark and not the
/// mode's words, so it is cut off the front.
#[test]
fn a_mode_row_says_the_bullet_the_card_prints() {
    let (view, texts, object) = asking_about(
        "217062f5-96f1-454c-9507-17f34ef37070",
        "en",
        "Choose one —\n\
             • Each opponent sacrifices a nontoken creature of their choice.\n\
             • Each opponent sacrifices a creature token of their choice.\n\
             • Each opponent sacrifices a planeswalker of their choice.",
    );
    let rows = cast_rows(
        object,
        &[
            CastModeKind::Mode(0),
            CastModeKind::Mode(1),
            CastModeKind::Mode(2),
        ],
        FaceNames {
            view: Some(&view),
            texts: Some(&texts),
        },
        Lang::En,
    );
    assert_eq!(
        rows[0].label,
        "Each opponent sacrifices a nontoken creature of their choice."
    );
    assert_eq!(
        rows[2].label,
        "Each opponent sacrifices a planeswalker of their choice."
    );
}

/// And it says it in the player's own language, because the sentence is
/// the *catalog's* and only the index is the registry's.
///
/// The text is the one the gateway actually serves for this printing,
/// **including its bullet**: a printing chooses its own mark, and the
/// German one is `*` where the English is `•`. A trim that knew only
/// the English mark left every German mode row starting with a star.
#[test]
fn a_mode_row_is_read_in_the_language_the_player_chose() {
    let (view, texts, object) = asking_about(
        "217062f5-96f1-454c-9507-17f34ef37070",
        "de",
        "Bestimme eines -\n\
             * Jeder Gegner opfert eine Nichtspielsteinkreatur, die er bestimmt.\n\
             * Jeder Gegner opfert einen Kreaturenspielstein, den er bestimmt.\n\
             * Jeder Gegner opfert einen Planeswalker, den er bestimmt.",
    );
    let rows = cast_rows(
        object,
        &[CastModeKind::Mode(1)],
        FaceNames {
            view: Some(&view),
            texts: Some(&texts),
        },
        Lang::De,
    );
    assert_eq!(
        rows[0].label,
        "Jeder Gegner opfert einen Kreaturenspielstein, den er bestimmt."
    );
}

/// A row for several modes says each one's words, in printed order, and
/// for a spree card without the plus sign and the mode's own cost in
/// front of them: the row draws beside it what the whole set costs.
#[test]
fn a_row_of_several_modes_says_each_without_its_own_cost() {
    let (view, texts, object) = asking_about(
        "7e7ec3d6-a84f-4cc3-93f4-4d181d41e126",
        "en",
        "Spree (Choose one or more additional costs.)\n\
             + {1} — All creatures lose all abilities until end of turn.\n\
             + {1} — Choose a creature you control. It gains indestructible \
             until end of turn.\n\
             + {3}{W}{W} — Destroy all creatures.",
    );
    let rows = cast_rows(
        object,
        &[CastModeKind::Modes(0b101)],
        FaceNames {
            view: Some(&view),
            texts: Some(&texts),
        },
        Lang::En,
    );
    assert_eq!(
        rows[0].label,
        "All creatures lose all abilities until end of turn. + Destroy all creatures."
    );
}

/// "Choose two": Cryptic Command's row for a pair says both modes' words,
/// in printed order, the way a spree row does.
#[test]
fn a_row_of_two_chosen_modes_says_both() {
    let (view, texts, object) = asking_about(
        "a3e51a35-09df-4189-b131-08a21e6a557d",
        "en",
        "Choose two —\n\
             • Counter target spell.\n\
             • Return target permanent to its owner's hand.\n\
             • Tap all creatures your opponents control.\n\
             • Draw a card.",
    );
    let rows = cast_rows(
        object,
        &[CastModeKind::Modes(0b1100)],
        FaceNames {
            view: Some(&view),
            texts: Some(&texts),
        },
        Lang::En,
    );
    assert_eq!(
        rows[0].label,
        "Tap all creatures your opponents control. + Draw a card."
    );
}

/// Ragavan's dash row names the keyword in the player's language, beside
/// the printed cost's row, which needs no words.
#[test]
fn a_dash_row_names_the_keyword() {
    for (lang, code, dash) in [(Lang::En, "en", "Dash"), (Lang::De, "de", "Sturmangriff")] {
        let (view, texts, object) =
            asking_about("37108cd4-bbab-4ce3-9ed6-f60e8422e703", code, "Dash {1}{R}");
        let rows = cast_rows(
            object,
            &[CastModeKind::Normal, CastModeKind::Dash],
            FaceNames {
                view: Some(&view),
                texts: Some(&texts),
            },
            lang,
        );
        assert_eq!(rows[1].label, dash);
    }
}

/// Uro's escape row names the keyword in the player's language.
#[test]
fn an_escape_row_names_the_keyword() {
    for (lang, code, escape) in [(Lang::En, "en", "Escape"), (Lang::De, "de", "Befreiung")] {
        let (view, texts, object) = asking_about(
            "ee302659-59ed-4eef-babe-451b9ccf7f14",
            code,
            "Escape—{G}{G}{U}{U}, Exile five other cards from your graveyard.",
        );
        let rows = cast_rows(
            object,
            &[CastModeKind::Escape],
            FaceNames {
                view: Some(&view),
                texts: Some(&texts),
            },
            lang,
        );
        assert_eq!(rows[0].label, escape);
    }
}

/// A printing whose own split is a different length is refused whole,
/// and the row falls to the card's English Oracle rather than to a number.
///
/// The `of` guard, at the one surface where being off by one is worst:
/// an index that is *in* range of a printing one mode short names the
/// mode beside the right one and draws it as the card's own words. So the
/// printing is not read at all, and the owner's rule for every row drawn
/// from card text — "Fallback ist immer englisch" — supplies the
/// sentence the index was counted against.
#[test]
fn a_printing_of_a_different_length_falls_to_the_english_oracle() {
    let (view, texts, object) = asking_about(
        "217062f5-96f1-454c-9507-17f34ef37070",
        "en",
        "Choose one —\n\
             • Each opponent sacrifices a nontoken creature of their choice.\n\
             • Each opponent sacrifices a creature token of their choice.",
    );
    let rows = cast_rows(
        object,
        &[CastModeKind::Mode(1)],
        FaceNames {
            view: Some(&view),
            texts: Some(&texts),
        },
        Lang::En,
    );
    assert_eq!(
        rows[0].label,
        "Each opponent sacrifices a creature token of their choice."
    );
}

/// The owner's other half of AM1: an alternative cost is a sentence too.
///
/// "Alternative cost" is a category and every card in the pool that has
/// one drew exactly that word. Solitude's is a keyword line, which is
/// the shape a sentence-shaped reader would have missed.
#[test]
fn an_alternative_cost_row_says_what_it_charges() {
    let (view, texts, object) = asking_about(
        "dcb9c2a7-ae54-4ddc-a567-640bf4bf4366",
        "en",
        "Flash\n\
             Lifelink\n\
             When this creature enters, exile up to one other target creature. \
             That creature's controller gains life equal to its power.\n\
             Evoke—Exile a white card from your hand.",
    );
    let rows = cast_rows(
        object,
        &[CastModeKind::Normal, CastModeKind::Alternative(0)],
        FaceNames {
            view: Some(&view),
            texts: Some(&texts),
        },
        Lang::En,
    );
    assert_eq!(rows[0].label, "", "the normal way has no line");
    assert_eq!(rows[1].label, "Evoke—Exile a white card from your hand.");
}

/// With no card text filed at all — the ordinary offline case — a row
/// says the card's English Oracle sentence, not the phrase it had.
#[test]
fn a_row_with_no_text_filed_says_the_english_oracle() {
    let (view, _, object) = asking_about(
        "dcb9c2a7-ae54-4ddc-a567-640bf4bf4366",
        "en",
        "Evoke—Exile a white card from your hand.",
    );
    let rows = cast_rows(
        object,
        &[CastModeKind::Alternative(0)],
        FaceNames {
            view: Some(&view),
            texts: None,
        },
        Lang::En,
    );
    assert_eq!(rows[0].label, "Evoke—Exile a white card from your hand.");
}

/// Where the card cannot be found, an alternative cost has no words:
/// there is nothing printed to say, and "Alternative cost" was this
/// client's wording. A mode with no sentence keeps its number, the one
/// thing that tells it from its neighbour.
///
/// Solitude has no modes, so its `Mode(1)` has no line in the table; and
/// with no view the card in hand cannot be named, so even its evoke cost
/// has no sentence to draw.
#[test]
fn a_row_with_no_printed_sentence_has_no_words_but_a_mode_its_number() {
    let (view, texts, object) = asking_about(
        "dcb9c2a7-ae54-4ddc-a567-640bf4bf4366",
        "en",
        "Evoke—Exile a white card from your hand.",
    );
    let unprinted = cast_rows(
        object,
        &[CastModeKind::Mode(1)],
        FaceNames {
            view: Some(&view),
            texts: Some(&texts),
        },
        Lang::En,
    );
    assert_eq!(
        unprinted[0].label, "Mode 2",
        "one-based, as a card numbers it"
    );
    let unfound = cast_rows(
        object,
        &[CastModeKind::Alternative(0)],
        FaceNames {
            view: None,
            texts: Some(&texts),
        },
        Lang::En,
    );
    assert_eq!(unfound[0].label, "");
}

/// [`mode_words`] is the one door the cast chooser's rows and the game
/// log's mode lines read (#23): a mode of Archdruid's Charm is its
/// printed sentence without the bullet, whether the catalog filed the
/// text or not (the compiled English Oracle stands in), and a card with
/// no modes has none to say. On the old code `mode_words` did not exist
/// and `mode_label` held the same chain privately, so the log (which has
/// no `FaceNames`) could name no mode at all.
#[test]
fn mode_words_are_the_printed_sentence_without_its_bullet() {
    let charm = baylee_cards::decks::by_name("Archdruid's Charm").expect("in the pool");
    let exile = "Exile target artifact or enchantment.";
    assert_eq!(
        mode_words(None, charm, 2).as_deref(),
        Some(exile),
        "no text filed: the compiled English Oracle"
    );

    let (_, texts, _) = asking_about(
        "3c1ef404-e2c6-486d-a5a2-d5779c71d498",
        "en",
        "Choose one \u{2014}\n\
             \u{2022} Search your library for a creature or land card and reveal it. \
             Put it onto the battlefield tapped if it's a land card. Otherwise, put it \
             into your hand. Then shuffle.\n\
             \u{2022} Put a +1/+1 counter on target creature you control. It deals \
             damage equal to its power to target creature you don't control.\n\
             \u{2022} Exile target artifact or enchantment.",
    );
    assert_eq!(
        mode_words(Some(&texts), charm, 2).as_deref(),
        Some(exile),
        "filed text: the same sentence, bullet cut"
    );
    assert!(
        mode_words(Some(&texts), charm, 0)
            .is_some_and(|words| words.starts_with("Search your library")),
        "and the first mode is the first bullet"
    );
    assert_eq!(mode_words(None, charm, 3), None, "there is no fourth mode");

    let bolt = baylee_cards::decks::by_name("Lightning Bolt").expect("in the pool");
    assert_eq!(mode_words(None, bolt, 0), None, "a card without modes");
}

/// The source of a modal trigger, on the battlefield as the view shows
/// it: the zone a trigger's question is asked from.
fn on_the_battlefield(name: &str) -> (baylee_view::PlayerView, ObjectId) {
    let object = crate::registry_printed(55, 0, name);
    let id = object.id;
    let view = baylee_client_core::test_support::ViewBuilder::new(2)
        .with_battlefield(0, [object])
        .build();
    (view, id)
}

/// The owner's report 01a0e3ec (#319): Inspirit's combat trigger asked
/// "Modus 1" or "Modus 2".
///
/// Its choice is printed inside one sentence, so neither mode *is* a
/// sentence and the table of printed lines rightly knows none; the row
/// fell straight to a number this client had worded. The shape here is
/// the report's: the trigger's source on the battlefield, a German
/// interface, and the German printing's text as Scryfall files it — in
/// English, as it was served for this card.
#[test]
fn inspirit_s_two_modes_say_the_card_s_own_words() {
    let (view, object) = on_the_battlefield("Inspirit, Flagship Vessel");
    let (_, texts, _) = asking_about(
        "554df866-3dbb-4811-8573-6033481591aa",
        "de",
        "Station (Tap another creature you control: Put charge counters equal to its \
             power on this Spacecraft. Station only as a sorcery. It's an artifact creature \
             at 8+.)\n\
             1+ | At the beginning of combat on your turn, put your choice of a +1/+1 counter \
             or two charge counters on up to one other target artifact.\n\
             8+ | Flying\n\
             Other artifacts you control have hexproof and indestructible.",
    );
    for texts in [Some(&texts), None] {
        let rows = cast_rows(
            object,
            &[CastModeKind::Mode(0), CastModeKind::Mode(1)],
            FaceNames {
                view: Some(&view),
                texts,
            },
            Lang::De,
        );
        assert_eq!(rows[0].label, "a +1/+1 counter");
        assert_eq!(rows[1].label, "two charge counters");
    }
}

/// Every mode of every modal ability in the pool is a row that says
/// something the card prints, and no two modes of one ability say the
/// same thing.
///
/// Asked the way the report was asked: a German interface with no text
/// filed, so a row can only come from the compiled English Oracle —
/// through the printed sentence a mode is, or the words it is inside a
/// sentence. The one mode allowed its number is the effect-less mode a
/// player declines "choose up to one" with, which no card prints.
#[test]
fn every_mode_in_the_pool_is_a_row_in_the_card_s_words() {
    let mut modes_seen = 0usize;
    let mut faces_seen = 0usize;
    for def in baylee_cards::all() {
        let modes = baylee_cards::lines::face_modes(def.abilities_for_face(CAST_FACE));
        if modes.is_empty() {
            continue;
        }
        faces_seen += 1;
        let (view, object) = on_the_battlefield(def.name());
        let kinds: Vec<CastModeKind> = (0..modes.len()).map(CastModeKind::Mode).collect();
        let rows = cast_rows(
            object,
            &kinds,
            FaceNames {
                view: Some(&view),
                texts: None,
            },
            Lang::De,
        );
        let mut said = Vec::new();
        for (at, (mode, row)) in modes.iter().zip(&rows).enumerate() {
            let number = Phrase::CastModeNumber.fill(Lang::De, &[&(at + 1).to_string()]);
            if mode.effects.is_empty() {
                assert_eq!(
                    row.label,
                    number,
                    "{} mode {at} declines, and is printed nowhere",
                    def.name()
                );
                continue;
            }
            modes_seen += 1;
            assert!(
                !row.label.is_empty() && row.label != number,
                "{} mode {at} is drawn as {:?}, not in the card's words",
                def.name(),
                row.label
            );
            let oracle = baylee_cards::oracle::face(def.index, CAST_FACE)
                .expect("a pool card has its Oracle text");
            assert!(
                oracle.contains(row.label.as_str()),
                "{} mode {at}: {:?} is not what the card prints",
                def.name(),
                row.label
            );
            said.push(row.label.clone());
        }
        let count = said.len();
        said.sort_unstable();
        said.dedup();
        assert_eq!(
            said.len(),
            count,
            "{}: two of its modes are drawn alike",
            def.name()
        );
    }
    // The walk's own floor: a pool that lost its modal cards, or a
    // reading of modes that found none, would pass everything above.
    assert!(faces_seen >= 10, "only {faces_seen} modal faces walked");
    assert!(modes_seen >= 25, "only {modes_seen} modes walked");
}

/// Every paged chooser reads a page one way: an empty list is one empty
/// page, an asked page past the end is the last one, and the arrows stand
/// only where there is a page to go to, worded alike.
#[test]
fn a_page_clamps_and_its_arrows_are_worded_alike() {
    let empty = Page::of(0, 3, 8);
    assert_eq!(
        (empty.start, empty.end, empty.index, empty.count),
        (0, 0, 0, 1)
    );
    assert!(!empty.earlier() && !empty.later());

    let last = Page::of(20, 5, 8);
    assert_eq!(
        (last.start, last.end, last.index, last.count),
        (16, 20, 2, 3)
    );
    assert!(last.earlier() && !last.later());
    let first = Page::of(20, 0, 8);
    assert_eq!((first.start, first.end), (0, 8));
    assert!(!first.earlier() && first.later());

    let mut rows = Vec::new();
    page_arrows(&mut rows, Lang::En, true, true);
    let words: Vec<(usize, &str)> = rows.iter().map(|r| (r.index, r.label.as_str())).collect();
    assert_eq!(
        words,
        [
            (baylee_client_core::targeting::PREVIOUS, "‹ Back"),
            (baylee_client_core::targeting::NEXT, "More ›"),
        ]
    );
}
