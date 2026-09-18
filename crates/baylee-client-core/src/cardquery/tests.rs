//! The language, held against itself.
//!
//! The bound that matters here is the round trip, and it is asserted over a
//! **corpus** rather than over a case: `parse(render(q)) == q` has to hold for
//! every shape the grammar can make, and a test that names three of them is a
//! test that passes the day a fourth is added wrongly.

use super::{
    Colors, Facts, Flag, Key, Match, Op, Query, Surface, Term, Value, matches, parse, render,
};

/// Every shape the grammar makes, written the way a player writes it.
///
/// Each line is one property of the language and is named in the comment
/// beside it, so a failure of [`every_written_query_survives_being_written`]
/// says which one broke.
pub(crate) const CORPUS: &[&str] = &[
    "",                                   // the empty box
    "lightning",                          // a loose word
    "lightning bolt",                     // two loose words, which is an and
    "\"sift through sands\"",             // a quoted phrase
    "!fire",                              // an exact name
    "!\"sift through sands\"",            // an exact name with spaces
    "-fire",                              // a negated loose word
    "name:bolt",                          // a name and nothing else
    "o:draw",                             // rules text
    "o:\"draw a card\"",                  // rules text with spaces
    "o:~",                                // the card's own name
    "t:creature",                         // a type
    "t:creature t:legendary",             // two types
    "-t:creature",                        // not a type
    "c:rg",                               // at least red and green
    "c>=uw -c:red",                       // the page's own example
    "c=2",                                // exactly two colours
    "c:m",                                // more than one colour
    "c:c",                                // colourless
    "id<=esper",                          // a nickname nothing here knows
    "id:c t:land",                        // the page's own example
    "m:{2}{W}{W}",                        // a cost, braced
    "m>{3}{W}{U}",                        // a cost, compared
    "mv:3",                               // a mana value
    "mv>=3 mv<=5",                        // a range
    "mv:even",                            // a parity
    "mv!=0",                              // not equal
    "pow>tou",                            // unreadable, and kept whole
    "pow>=8",                             // a printed number
    "loy=3",                              // a printed loyalty
    "is:playable",                        // a flag
    "-is:stub",                           // not a flag
    "t:fish or t:bird",                   // an or
    "t:legendary (t:goblin or t:elf)",    // a group inside an and
    "(a or b) or c",                      // a group that flattens
    "t:creature c:r or t:land mv:0",      // or binds looser than and
    "-(t:creature c:r)",                  // a negated group
    "through (depths or sands or mists)", // the page's own example
    "frobnicate:yes",                     // a key nothing here knows
    "t:creature frobnicate:yes",          // ... beside one it does
    "\"or\"",                             // the joint word, as a name
    "t:\"legendary creature\"",           // a value with a space in it
];

fn q(text: &str) -> Query {
    parse(text)
}

#[test]
fn every_written_query_survives_being_written() {
    for line in CORPUS {
        let once = q(line);
        let written = render(&once);
        let twice = parse(&written);
        assert_eq!(
            once, twice,
            "`{line}` was written as `{written}` and read back as something else"
        );
    }
}

#[test]
fn writing_a_query_twice_writes_the_same_string() {
    for line in CORPUS {
        let written = render(&q(line));
        let again = render(&parse(&written));
        assert_eq!(written, again, "`{line}` is not written canonically");
    }
}

#[test]
fn an_or_binds_looser_than_the_space_between_two_terms() {
    let Query::Any(parts) = q("t:creature c:r or t:land mv:0") else {
        panic!("the top of `a b or c d` is the or");
    };
    assert_eq!(parts.len(), 2);
    assert!(
        matches!(&parts[0], Query::All(inner) if inner.len() == 2),
        "each branch of the or is the conjunction beside it"
    );
    assert!(matches!(&parts[1], Query::All(inner) if inner.len() == 2));
}

#[test]
fn a_group_of_ors_flattens_into_the_or_it_stands_in() {
    assert_eq!(q("(a or b) or c"), q("a or b or c"));
    assert_eq!(render(&q("(a or b) or c")), "a or b or c");
}

#[test]
fn a_group_inside_a_conjunction_keeps_its_brackets() {
    let written = render(&q("t:legendary (t:goblin or t:elf)"));
    assert_eq!(written, "t:legendary (t:goblin or t:elf)");
}

#[test]
fn an_unclosed_group_is_still_a_query() {
    // A player types the opening bracket before the closing one, and the box
    // filters as they type. The half-written form has to mean something.
    assert_eq!(q("t:creature (c:r or c:g"), q("t:creature (c:r or c:g)"));
}

#[test]
fn an_unclosed_quote_runs_to_the_end_of_the_line() {
    assert_eq!(q("o:\"draw a"), q("o:\"draw a\""));
}

// ------------------------------------------------------------ evaluation

/// Lightning Bolt, as the pool knows it, with a German name beside it.
fn bolt() -> (Vec<String>, Vec<String>, Vec<Flag>) {
    (
        vec!["\u{7a32}\u{59bb}".to_string()],
        vec!["Instant".to_string()],
        vec![Flag::Playable],
    )
}

fn facts<'a>(
    name: &'a str,
    alt: &'a [String],
    kinds: &'a [String],
    flags: &'a [Flag],
) -> Facts<'a> {
    Facts {
        name,
        english_name: "Lightning Bolt",
        alt_names: alt,
        type_line: "Instant",
        kinds,
        oracle: "Lightning Bolt deals 3 damage to any target.",
        colors: Colors::R,
        identity: Colors::R,
        mana_cost: "{R}",
        mana_value: 1,
        power: None,
        toughness: None,
        loyalty: None,
        flags,
    }
}

fn asks(text: &str, facts: &Facts<'_>, surface: Surface) -> Match {
    matches(&parse(text), facts, surface)
}

#[test]
fn a_card_is_found_by_the_name_it_is_drawn_under_and_by_its_english_one() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Blitzschlag", &alt, &kinds, &flags);
    assert_eq!(asks("blitz", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("lightning", &card, Surface::POOL), Match::Yes);
    assert_eq!(
        asks("\u{7a32}\u{59bb}", &card, Surface::POOL),
        Match::Yes,
        "and the Japanese printing beside it, which folds to itself"
    );
    assert_eq!(asks("!blitzschlag", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("!blitz", &card, Surface::POOL), Match::No);
}

#[test]
fn a_bare_word_looks_further_than_a_name_does() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Blitzschlag", &alt, &kinds, &flags);
    // The box a player types into is the only one there is, so a loose word
    // reaches the type line and the rules text as well.
    assert_eq!(asks("instant", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("damage", &card, Surface::POOL), Match::Yes);
    // Asking for a name on purpose gets a name.
    assert_eq!(asks("name:instant", &card, Surface::POOL), Match::No);
    assert_eq!(asks("name:blitz", &card, Surface::POOL), Match::Yes);
    assert_eq!(render(&q("name:blitz")), "name:blitz");
    assert_eq!(render(&q("blitz")), "blitz");
}

#[test]
fn a_type_answers_in_the_players_language_and_in_english() {
    let (alt, kinds, flags) = bolt();
    let mut card = facts("Blitzschlag", &alt, &kinds, &flags);
    card.type_line = "Spontanzauber";
    assert_eq!(asks("t:spontan", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("t:instant", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("t:creature", &card, Surface::POOL), Match::No);
}

#[test]
fn the_tilde_in_a_text_search_is_the_cards_own_name() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Lightning Bolt", &alt, &kinds, &flags);
    assert_eq!(asks("o:\"~ deals 3\"", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("o:\"~ draws\"", &card, Surface::POOL), Match::No);
}

#[test]
fn a_bare_colon_means_at_least_for_colour_and_at_most_for_identity() {
    let (alt, kinds, flags) = bolt();
    let mut card = facts("Lightning Bolt", &alt, &kinds, &flags);
    card.colors = Colors(Colors::R.0 | Colors::G.0);
    card.identity = Colors(Colors::R.0 | Colors::G.0);
    // At least red: a red-green card is.
    assert_eq!(asks("c:r", &card, Surface::POOL), Match::Yes);
    // At most red: a red-green card is not.
    assert_eq!(asks("id:r", &card, Surface::POOL), Match::No);
    assert_eq!(asks("id:rg", &card, Surface::POOL), Match::Yes);
}

#[test]
fn a_colour_nickname_matches_nothing_rather_than_the_letters_in_it() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Lightning Bolt", &alt, &kinds, &flags);
    // Read letter by letter, `esper` holds an `r` and would find every red
    // card; `boros` holds `b` and `r` and would find the black-red ones.
    // Both are wrong answers that look exactly like right ones.
    assert_eq!(asks("id<=esper", &card, Surface::POOL), Match::Unknown);
    assert_eq!(asks("c:boros", &card, Surface::POOL), Match::Unknown);
    assert_eq!(asks("c:r", &card, Surface::POOL), Match::Yes);
    assert_eq!(
        render(&q("c:boros")),
        "c:boros",
        "and it is still written back out, so a player can see what was refused"
    );
}

#[test]
fn more_than_one_colour_is_a_count_and_keeps_the_word_it_was_typed_with() {
    let (alt, kinds, flags) = bolt();
    let mut card = facts("Lightning Bolt", &alt, &kinds, &flags);
    assert_eq!(asks("c:m", &card, Surface::POOL), Match::No);
    card.colors = Colors(Colors::R.0 | Colors::G.0);
    assert_eq!(asks("c:m", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("c=2", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("c>2", &card, Surface::POOL), Match::No);
    assert_eq!(render(&q("c:m")), "c:m");
}

#[test]
fn a_colourless_identity_is_the_empty_set_and_not_every_set() {
    let (alt, kinds, flags) = bolt();
    let mut card = facts("Wastes", &alt, &kinds, &flags);
    card.colors = Colors::default();
    card.identity = Colors::default();
    assert_eq!(asks("id:c", &card, Surface::POOL), Match::Yes);
    card.identity = Colors::R;
    assert_eq!(
        asks("id:c", &card, Surface::POOL),
        Match::No,
        "a red card has more identity than colourless, so `at most` refuses it"
    );
}

#[test]
fn a_mana_cost_is_a_multiset_and_a_colon_means_it_contains_them() {
    let (alt, kinds, flags) = bolt();
    let mut card = facts("Wrath of God", &alt, &kinds, &flags);
    card.mana_cost = "{2}{W}{W}";
    assert_eq!(asks("m:{W}{W}", &card, Surface::POOL), Match::Yes);
    assert_eq!(
        asks("m:WWW", &card, Surface::POOL),
        Match::No,
        "two white symbols do not contain three"
    );
    assert_eq!(asks("m:2WW", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("m={2}{W}{W}", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("m>{2}{W}{W}", &card, Surface::POOL), Match::No);
}

#[test]
fn a_card_with_no_printed_number_is_refused_rather_than_unanswered() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Lightning Bolt", &alt, &kinds, &flags);
    assert_eq!(asks("pow>=4", &card, Surface::POOL), Match::No);
    assert_eq!(
        asks("-pow>=4", &card, Surface::POOL),
        Match::Yes,
        "an instant has no power, which is a fact and not a gap"
    );
}

#[test]
fn a_key_the_surface_cannot_answer_matches_nothing_either_way() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Lightning Bolt", &alt, &kinds, &flags);
    assert_eq!(asks("o:draw", &card, Surface::ZONE), Match::Unknown);
    assert_eq!(
        asks("-o:draw", &card, Surface::ZONE),
        Match::Unknown,
        "negating a question nobody can answer does not answer it"
    );
    assert!(!asks("-o:draw", &card, Surface::ZONE).shown());
}

#[test]
fn an_unanswerable_branch_of_an_or_leaves_the_other_one_alone() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Lightning Bolt", &alt, &kinds, &flags);
    assert_eq!(
        asks("o:draw or t:instant", &card, Surface::ZONE),
        Match::Yes,
        "one branch answering yes is enough, whatever the other one knows"
    );
    assert_eq!(
        asks("o:draw or t:creature", &card, Surface::ZONE),
        Match::Unknown
    );
}

#[test]
fn a_key_nothing_knows_narrows_the_search_to_nothing() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Lightning Bolt", &alt, &kinds, &flags);
    assert_eq!(asks("frobnicate:yes", &card, Surface::POOL), Match::Unknown);
    assert_eq!(
        asks("t:instant frobnicate:yes", &card, Surface::POOL),
        Match::Unknown,
        "a typo must not quietly widen what comes back"
    );
    assert_eq!(render(&q("frobnicate:yes")), "frobnicate:yes");
}

#[test]
fn a_flag_the_surface_does_not_know_is_not_the_same_as_a_flag_that_is_false() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Lightning Bolt", &alt, &kinds, &flags);
    assert_eq!(asks("is:playable", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("is:stub", &card, Surface::POOL), Match::No);
    assert_eq!(
        asks("is:token", &card, Surface::POOL),
        Match::Unknown,
        "the pool has no tokens to be asked about"
    );
}

#[test]
fn not_is_the_same_as_a_negated_is() {
    assert_eq!(q("not:stub"), q("-is:stub"));
    assert_eq!(render(&q("not:stub")), "-is:stub");
}

#[test]
fn the_empty_box_lets_everything_through() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Lightning Bolt", &alt, &kinds, &flags);
    assert!(parse("").is_anything());
    assert_eq!(asks("", &card, Surface::POOL), Match::Yes);
    assert_eq!(asks("   ", &card, Surface::POOL), Match::Yes);
}

// ------------------------------------------------------- reading one

#[test]
fn reading_a_query_finds_every_term_however_deep_it_sits() {
    let query = q("t:creature -(c:r or mv:3)");
    let keys: Vec<&Key> = query.terms().into_iter().map(|term| &term.key).collect();
    assert_eq!(
        keys,
        vec![&Key::Type, &Key::Color, &Key::ManaValue],
        "in written order, and none of them lost to the negation above it"
    );
}

#[test]
fn a_term_a_dialog_builds_is_written_the_way_a_player_would_have_typed_it() {
    let built = Query::All(vec![
        Query::Term(Term {
            key: Key::Type,
            op: Op::Colon,
            value: Value::Word("creature".to_string()),
        }),
        Query::Not(Box::new(Query::Term(Term {
            key: Key::Color,
            op: Op::Colon,
            value: Value::Colors(Colors::W),
        }))),
        Query::Term(Term {
            key: Key::ManaValue,
            op: Op::Le,
            value: Value::Number(3),
        }),
    ]);
    assert_eq!(render(&built), "t:creature -c:w mv<=3");
    assert_eq!(parse(&render(&built)), built);
}

/// The two surfaces, held against what their source actually carries.
///
/// A [`Surface`] is a hand-written list, which is the one shape that rots
/// without anything noticing: a key left out answers `Unknown` and so hides
/// every row, which reads exactly like a search that found nothing. It was
/// already wrong once — `ZONE` omitted the colours, the power, the toughness
/// and the loyalty, all four of which `baylee_view::PublicObject` projects.
/// So the list is asserted rather than trusted, in both directions.
#[test]
fn each_surface_answers_what_its_own_source_carries() {
    let (alt, kinds, flags) = bolt();
    let card = facts("Lightning Bolt", &alt, &kinds, &flags);
    // A printing has prose, a cost and a colour identity. A view has none of
    // the three, and no card in a zone is a stub.
    for key in ["o:bolt", "m:{R}", "id:r", "is:stub"] {
        assert_eq!(
            asks(key, &card, Surface::ZONE),
            Match::Unknown,
            "{key} is not a question a projected object can answer"
        );
        assert_ne!(
            asks(key, &card, Surface::POOL),
            Match::Unknown,
            "{key} is a question a compiled card row must answer"
        );
    }
    // And every characteristic a view projects is answerable on both.
    for key in ["t:instant", "c:r", "mv:3", "pow>=1", "tou>=1", "loy>=1"] {
        for surface in [Surface::POOL, Surface::ZONE] {
            assert_ne!(
                asks(key, &card, surface),
                Match::Unknown,
                "{key} is a projected characteristic and both surfaces have it"
            );
        }
    }
}
