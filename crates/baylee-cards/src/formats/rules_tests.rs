//! The validation function per problem kind and format, with real cards.

use super::*;
use crate::decks::LoadedDeck;
use baylee_cardtext::{CardLine, PoolFactsBody, PoolSet};

fn card(name: &str) -> CardIndex {
    crate::decks::by_name(name).unwrap_or_else(|| panic!("{name} is in the pool"))
}

fn many(name: &str, n: usize) -> Vec<CardIndex> {
    vec![card(name); n]
}

/// A table holding these verdicts for these cards in one format.
fn table(format: &str, verdicts: &[(&str, &str)]) -> PoolFacts {
    let lines = verdicts
        .iter()
        .map(|(name, word)| CardLine {
            index: card(name).get(),
            legalities: [(format.to_owned(), (*word).to_owned())].into(),
            printed: Vec::new(),
        })
        .collect();
    let set = PoolSet {
        code: "tst".to_owned(),
        name: "Test".to_owned(),
    };
    PoolFactsBody::assemble(vec![set], lines).into()
}

fn rules<'a>(format: &'a str, facts: &'a PoolFacts) -> Rules<'a> {
    Rules {
        format,
        facts: Some(facts),
        list: None,
    }
}

fn list(entries: &[(&str, ListStatus)]) -> ListRules {
    entries
        .iter()
        .map(|(name, status)| {
            let def = crate::by_index(card(name)).expect("in the registry");
            (def.oracle_id.to_owned(), *status)
        })
        .collect()
}

fn deck<'a>(main: &'a [CardIndex]) -> DeckCards<'a> {
    DeckCards {
        main,
        side: &[],
        leaders: &[],
    }
}

/// Sixty cards a constructed table says nothing against.
fn sixty() -> (Vec<CardIndex>, PoolFacts) {
    let mut main = many("Island", 56);
    main.extend(many("Counterspell", 4));
    let facts = table("modern", &[("Island", "legal"), ("Counterspell", "legal")]);
    (main, facts)
}

#[test]
fn a_deck_the_format_has_nothing_against_is_legal() {
    let (main, facts) = sixty();
    let report = validate(deck(&main), rules("modern", &facts));
    assert_eq!(report.problems, []);
    assert_eq!(report.standing(), Standing::Legal);
}

#[test]
fn a_banned_card_is_out_of_the_format_and_of_its_sideboard() {
    let (mut main, _) = sixty();
    main.pop();
    main.push(card("Sol Ring"));
    let facts = table(
        "modern",
        &[
            ("Island", "legal"),
            ("Counterspell", "legal"),
            ("Sol Ring", "banned"),
        ],
    );
    let report = validate(deck(&main), rules("modern", &facts));
    assert_eq!(
        report.problems,
        [Problem::Banned {
            card: card("Sol Ring").get()
        }]
    );
    assert_eq!(report.standing(), Standing::Illegal);

    let (main, _) = sixty();
    let side = many("Sol Ring", 1);
    let report = validate(
        DeckCards {
            main: &main,
            side: &side,
            leaders: &[],
        },
        rules("modern", &facts),
    );
    assert_eq!(
        report.problems,
        [Problem::Banned {
            card: card("Sol Ring").get()
        }],
        "a sideboard is part of the deck"
    );
}

#[test]
fn restricted_means_one_copy_counted_with_the_sideboard() {
    let facts = table(
        "vintage",
        &[
            ("Island", "legal"),
            ("Counterspell", "legal"),
            ("Sol Ring", "restricted"),
        ],
    );
    let mut main = many("Island", 59);
    main.push(card("Sol Ring"));
    assert_eq!(
        validate(deck(&main), rules("vintage", &facts)).problems,
        [],
        "one copy is the point of being restricted"
    );
    let side = many("Sol Ring", 1);
    let report = validate(
        DeckCards {
            main: &main,
            side: &side,
            leaders: &[],
        },
        rules("vintage", &facts),
    );
    assert_eq!(
        report.problems,
        [Problem::Restricted {
            card: card("Sol Ring").get(),
            have: 2
        }]
    );
}

#[test]
fn a_card_the_format_does_not_have_is_named() {
    let mut main = many("Island", 59);
    main.push(card("Sol Ring"));
    let facts = table(
        "standard",
        &[("Island", "legal"), ("Sol Ring", "not_legal")],
    );
    assert_eq!(
        validate(deck(&main), rules("standard", &facts)).problems,
        [Problem::NotLegal {
            card: card("Sol Ring").get()
        }]
    );
}

#[test]
fn the_copy_limit_runs_over_deck_and_sideboard() {
    let side = many("Counterspell", 4);
    let (main, facts) = sixty();
    let report = validate(
        DeckCards {
            main: &main,
            side: &side,
            leaders: &[],
        },
        rules("modern", &facts),
    );
    assert_eq!(
        report.problems,
        [Problem::TooManyCopies {
            card: card("Counterspell").get(),
            have: 8,
            limit: 4
        }]
    );
}

#[test]
fn a_sideboard_is_fifteen_and_a_commander_deck_has_none() {
    let (main, facts) = sixty();
    let side = many("Island", 16);
    let report = validate(
        DeckCards {
            main: &main,
            side: &side,
            leaders: &[],
        },
        rules("modern", &facts),
    );
    assert_eq!(
        report.problems,
        [Problem::SideboardTooBig {
            have: 16,
            limit: 15
        }]
    );

    let tazri = card("General Tazri");
    let main = many("Plains", 99);
    let side = many("Plains", 1);
    let facts = table(
        "commander",
        &[("Plains", "legal"), ("General Tazri", "legal")],
    );
    let report = validate(
        DeckCards {
            main: &main,
            side: &side,
            leaders: &[tazri],
        },
        rules("commander", &facts),
    );
    assert_eq!(
        report.problems,
        [Problem::SideboardTooBig { have: 1, limit: 0 }]
    );
}

/// A leader of white alone, for a deck an Island falls outside of.
fn white_leader() -> &'static baylee_cards_dsl::CardDef {
    crate::all()
        .find(|def| {
            crate::decks::leader_of(def.index).is_some_and(|l| l.eligible)
                && def.color_identity == ColorSet::from_slice(&[baylee_core::color::Color::White])
        })
        .expect("a mono-white leader in the pool")
}

#[test]
fn a_commander_deck_is_held_to_its_leader_in_every_commander_format() {
    let leader = white_leader();
    let mut main = many("Plains", 98);
    main.push(card("Sol Ring"));
    for format in ["commander", "duel", "predh", "paupercommander"] {
        let facts = table(
            format,
            &[
                ("Plains", "legal"),
                ("Sol Ring", "legal"),
                (leader.name(), "legal"),
            ],
        );
        let report = validate(
            DeckCards {
                main: &main,
                side: &[],
                leaders: &[leader.index],
            },
            rules(format, &facts),
        );
        assert_eq!(report.standing(), Standing::Legal, "{format}");
        // One card more of what the leader's identity does not hold.
        let mut outside = main.clone();
        outside[0] = card("Island");
        let report = validate(
            DeckCards {
                main: &outside,
                side: &[],
                leaders: &[leader.index],
            },
            rules(format, &facts),
        );
        assert!(
            report.problems.contains(&Problem::OutsideIdentity {
                card: card("Island").get()
            }),
            "{format}: {:?}",
            report.problems
        );
    }
}

#[test]
fn a_card_that_cannot_lead_and_a_missing_leader_are_named() {
    let main = many("Plains", 99);
    let facts = table("commander", &[("Plains", "legal"), ("Sol Ring", "legal")]);
    let ring = card("Sol Ring");
    let report = validate(
        DeckCards {
            main: &main,
            side: &[],
            leaders: &[ring],
        },
        rules("commander", &facts),
    );
    assert!(
        report
            .problems
            .contains(&Problem::NotALeader { card: ring.get() })
    );
    let report = validate(deck(&main), rules("commander", &facts));
    assert!(report.problems.contains(&Problem::Leaders { have: 0 }));
}

/// A modal double-faced card counts every face (CR 903.4d's "both faces",
/// as the registry records it): a leader of the front's colours alone does
/// not admit it.
#[test]
fn a_back_face_counts_towards_the_identity() {
    let double = crate::all()
        .find(|def| {
            def.faces.len() == 2
                && def.faces[1].castable_from_hand
                && !def.color_identity.difference(front_colors(def)).is_empty()
        })
        .expect("a modal double-faced card whose back has a colour its front lacks");
    let leader = crate::all()
        .find(|def| {
            crate::decks::leader_of(def.index).is_some_and(|l| l.eligible)
                && !front_colors(double).is_empty()
                && def.color_identity == front_colors(double)
        })
        .expect("a leader of exactly the front's colours");
    let main = vec![double.index];
    let problems = check(Shape::Commander, &main, &[leader.index]);
    assert!(
        problems.contains(&Problem::OutsideIdentity {
            card: double.index.get()
        }),
        "{} under {}: {problems:?}",
        double.name(),
        leader.name()
    );
}

/// The colours the front face's mana cost shows.
fn front_colors(def: &'static baylee_cards_dsl::CardDef) -> ColorSet {
    def.faces[0].mana_cost.colors()
}

#[test]
fn a_list_stands_in_for_the_formats_bans_but_not_for_its_pool() {
    let mut main = many("Island", 57);
    main.push(card("Sol Ring"));
    main.push(card("Counterspell"));
    main.push(card("Lightning Bolt"));
    let facts = table(
        "modern",
        &[
            ("Island", "legal"),
            ("Sol Ring", "banned"),
            ("Counterspell", "legal"),
            ("Lightning Bolt", "not_legal"),
        ],
    );
    // The list took Sol Ring off and put Counterspell on; Lightning Bolt
    // stays out of the format's pool whatever the list says.
    let on_it = list(&[("Counterspell", ListStatus::Banned)]);
    let report = validate(
        deck(&main),
        Rules {
            list: Some(&on_it),
            ..rules("modern", &facts)
        },
    );
    assert_eq!(report.problems.len(), 2, "{:?}", report.problems);
    assert!(report.problems.contains(&Problem::ListBanned {
        card: card("Counterspell").get()
    }));
    assert!(report.problems.contains(&Problem::NotLegal {
        card: card("Lightning Bolt").get()
    }));
    assert!(
        !report
            .problems
            .iter()
            .any(|problem| problem.concerns(card("Sol Ring").get())),
        "off the list is off the ban: {:?}",
        report.problems
    );
}

#[test]
fn a_list_restricts_to_one_copy() {
    let mut main = many("Island", 58);
    main.extend(many("Counterspell", 2));
    let facts = table("modern", &[("Island", "legal"), ("Counterspell", "legal")]);
    let on_it = list(&[("Counterspell", ListStatus::Restricted)]);
    let report = validate(
        deck(&main),
        Rules {
            list: Some(&on_it),
            ..rules("modern", &facts)
        },
    );
    assert_eq!(
        report.problems,
        [Problem::ListRestricted {
            card: card("Counterspell").get(),
            have: 2
        }]
    );
}

/// In the deck it is fine; leading it is what the list refuses.
#[test]
fn banned_as_commander_is_only_about_the_leader() {
    let tazri = card("General Tazri");
    let on_it = list(&[("General Tazri", ListStatus::BannedAsCommander)]);
    let facts = table(
        "commander",
        &[("Plains", "legal"), ("General Tazri", "legal")],
    );
    let main = many("Plains", 99);
    let report = validate(
        DeckCards {
            main: &main,
            side: &[],
            leaders: &[tazri],
        },
        Rules {
            list: Some(&on_it),
            ..rules("commander", &facts)
        },
    );
    assert_eq!(
        report.problems,
        [Problem::ListBannedAsCommander { card: tazri.get() }]
    );

    let other = crate::all()
        .find(|def| {
            def.index != tazri
                && crate::decks::leader_of(def.index).is_some_and(|l| l.eligible)
                && def
                    .color_identity
                    .difference(ColorSet::from_slice(&[baylee_core::color::Color::White]))
                    .is_empty()
        })
        .expect("a leader who may play Tazri's Plains");
    let mut main = many("Plains", 98);
    main.push(tazri);
    let report = validate(
        DeckCards {
            main: &main,
            side: &[],
            leaders: &[other.index],
        },
        Rules {
            list: Some(&on_it),
            format: FREEFORM,
            facts: None,
        },
    );
    assert_eq!(report.problems, [], "in the deck it is any other card");
}

/// A list for no format asks nothing but the list: no shape, no verdicts,
/// and nothing unasked.
#[test]
fn a_freeform_list_asks_only_the_list() {
    let main = many("Sol Ring", 2);
    let on_it = list(&[("Sol Ring", ListStatus::Banned)]);
    let report = validate(
        deck(&main),
        Rules {
            format: FREEFORM,
            facts: None,
            list: Some(&on_it),
        },
    );
    assert_eq!(
        report.problems,
        [Problem::ListBanned {
            card: card("Sol Ring").get()
        }]
    );
    assert!(!report.unchecked.any());
    let report = validate(
        deck(&main),
        Rules {
            format: FREEFORM,
            facts: None,
            list: None,
        },
    );
    assert_eq!(report.standing(), Standing::Legal, "nothing to keep to");
}

/// Offline: what can be asked without the table is asked, and the deck is
/// never called legal.
#[test]
fn without_a_table_a_deck_is_not_called_legal() {
    let (main, _) = sixty();
    let offline = Rules {
        format: "modern",
        facts: None,
        list: None,
    };
    let report = validate(deck(&main), offline);
    assert_eq!(report.problems, []);
    assert!(report.unchecked.statuses);
    assert_eq!(report.standing(), Standing::Unknown);

    // A shape problem is still a problem offline.
    let report = validate(deck(&main[..10]), offline);
    assert_eq!(report.standing(), Standing::Illegal);
    assert!(report.unchecked.statuses);

    // An empty table is no table: a gateway with no catalog.
    let empty = PoolFacts::default();
    let report = validate(deck(&main), rules("modern", &empty));
    assert!(report.unchecked.statuses);
}

#[test]
fn a_card_the_table_lacks_is_unchecked_and_not_a_fault() {
    let (main, _) = sixty();
    let facts = table("modern", &[("Island", "legal")]);
    let report = validate(deck(&main), rules("modern", &facts));
    assert_eq!(report.problems, []);
    assert_eq!(report.unchecked.cards, [card("Counterspell").get()]);
    assert_eq!(report.standing(), Standing::Unknown);
}

#[test]
fn a_format_with_no_known_shape_says_so_and_still_reads_its_verdicts() {
    let main = many("Sol Ring", 2);
    let facts = table("oathbreaker", &[("Sol Ring", "banned")]);
    let report = validate(deck(&main), rules("oathbreaker", &facts));
    assert!(report.unchecked.shape);
    assert_eq!(
        report.problems,
        [Problem::Banned {
            card: card("Sol Ring").get()
        }]
    );
}

/// Coverage is its own notice: a format-legal deck may hold a card with no
/// script.
#[test]
fn a_card_without_a_script_is_a_notice_and_no_fault() {
    let stub = crate::all()
        .find(|def| def.coverage == baylee_cards_dsl::Coverage::Unimplemented)
        .map(|def| def.index);
    let Some(stub) = stub else {
        // Every card of the pool has a script today; nothing to tell apart.
        return;
    };
    let main = vec![stub];
    let report = validate(
        deck(&main),
        Rules {
            format: FREEFORM,
            facts: None,
            list: None,
        },
    );
    assert_eq!(report.unscripted, [stub.get()]);
    assert_eq!(report.standing(), Standing::Legal);
}

#[test]
fn the_same_deck_comes_to_the_same_report() {
    let (main, facts) = sixty();
    let mut shuffled = main.clone();
    shuffled.reverse();
    assert_eq!(
        validate(deck(&main), rules("modern", &facts)),
        validate(deck(&shuffled), rules("modern", &facts))
    );
}

#[test]
fn a_loaded_deck_is_read_like_any_other() {
    let main: Vec<_> = many("Plains", 99)
        .into_iter()
        .map(crate::decks::DeckCard::plain)
        .collect();
    let loaded = LoadedDeck {
        name: "Tazri".to_owned(),
        main,
        sideboard: Vec::new(),
        commanders: vec![crate::decks::DeckCard::plain(card("General Tazri"))],
    };
    let facts = table(
        "commander",
        &[("Plains", "legal"), ("General Tazri", "legal")],
    );
    assert_eq!(
        validate_loaded(&loaded, rules("commander", &facts)).standing(),
        Standing::Legal
    );
}
