use super::*;

/// `docs/legal.md` §3: Scryfall's terms ask a client for the words
/// "data and images provided by Scryfall" (#325). The English is those
/// words; every language names Scryfall.
#[test]
fn scryfall_is_credited_in_its_own_words() {
    assert!(
        Phrase::ScryfallCredit
            .text(Lang::En)
            .contains("data and images provided by Scryfall"),
        "{:?}",
        Phrase::ScryfallCredit.text(Lang::En)
    );
    for lang in Lang::ALL {
        assert!(
            Phrase::ScryfallCredit.text(lang).contains("Scryfall"),
            "{lang:?}"
        );
    }
}

/// The whole point of the macro: there is no such thing as a phrase with
/// no German. This test cannot fail — it would not compile — and is here
/// so that the guarantee is written down where a reader looks for it.
#[test]
fn every_phrase_answers_in_every_language() {
    for phrase in Phrase::ALL {
        for lang in Lang::ALL {
            assert!(
                !phrase.text(lang).is_empty(),
                "{phrase:?} says nothing in {lang:?}"
            );
        }
    }
}

/// The tab over the local seat says the pronoun once.
#[test]
fn a_seat_named_for_the_pronoun_is_not_named_twice() {
    assert_eq!(own_seat_name(Lang::En, "You"), "You");
    assert_eq!(own_seat_name(Lang::De, "Du"), "Du");
    // A real name still gets the pronoun in front of it: at a table of
    // six, "You" alone does not say which chair.
    assert_eq!(own_seat_name(Lang::En, "Viktor"), "You (Viktor)");
    assert_eq!(own_seat_name(Lang::De, "Viktor"), "Du (Viktor)");
    // The languages do not borrow each other's pronoun — a German table
    // whose host still names the seat in English is two claims, not one.
    assert_eq!(own_seat_name(Lang::De, "You"), "Du (You)");
    assert_eq!(own_seat_name(Lang::En, "you"), "You");
}

/// `card(s)` is not a plural in any language, and the sheet greys what a
/// sentence says in brackets — so the broken form was drawn as an
/// editorial aside, in grey, right beside the number it disagreed with.
/// A selector on `$count` is the way to say this; a bracketed suffix is
/// not. Both forms of a counted phrase are read.
#[test]
fn no_phrase_fakes_a_plural_with_a_bracket() {
    for phrase in Phrase::ALL {
        for lang in Lang::ALL {
            let forms = if Phrase::ARGS[*phrase as usize].contains(&"count") {
                vec![phrase.text_count(lang, 1), phrase.text_count(lang, 2)]
            } else {
                vec![phrase.text(lang).to_owned()]
            };
            for text in forms {
                for fake in ["(s)", "(n)", "(e)", "(en)", "(er)"] {
                    assert!(
                        !text.contains(fake),
                        "{phrase:?} says {fake} in {lang:?} instead of choosing a form"
                    );
                }
            }
        }
    }
}

/// A counted phrase is one message, and its number picks the form:
/// exactly one takes `[one]`, and **zero takes the plural** in both
/// languages ("Discard 0 cards"). A number that is shown and also picks is
/// passed once, as a number.
#[test]
fn a_counted_phrase_takes_the_form_its_number_asks_for() {
    let discard = |lang, n| Phrase::DiscardCards.fill_count(lang, n, &[]);
    assert_eq!(discard(Lang::En, 1), "Discard 1 card");
    assert_eq!(discard(Lang::En, 3), "Discard 3 cards");
    assert_eq!(discard(Lang::En, 0), "Discard 0 cards");
    assert_eq!(discard(Lang::De, 1), "Wirf 1 Karte ab");
    assert_eq!(discard(Lang::De, 3), "Wirf 3 Karten ab");
    // A noun whose number is said by another phrase.
    assert_eq!(Phrase::NounCards.fill_count(Lang::En, 1, &[]), "card");
    assert_eq!(Phrase::NounCards.fill_count(Lang::En, 2, &[]), "cards");
    // With slots: the count is written, every other argument stays a slot.
    assert_eq!(
        Phrase::LogCountersPlus.text_count(Lang::En, 1),
        "+{0}/+{1} counter"
    );
    // `text` alone (no number) reads as the plural, as a label does.
    assert_eq!(Phrase::NounCards.text(Lang::En), "cards");
}

/// Most callers pass a count as text (`n.to_string()`); a whole number
/// reaches Fluent as a number all the same, so every selector picks, and it
/// reads back as the same digits. Text that only looks like a number stays
/// text (`03` keeps its zero).
#[test]
fn a_count_passed_as_text_still_picks_its_form() {
    assert_eq!(
        Phrase::DiscardCards.fill(Lang::En, &["1"]),
        "Discard 1 card"
    );
    assert_eq!(
        Phrase::DiscardCards.fill(Lang::En, &["12"]),
        "Discard 12 cards"
    );
    assert_eq!(
        Phrase::DiscardCards.fill(Lang::De, &["1"]),
        "Wirf 1 Karte ab"
    );
    assert_eq!(
        Phrase::DiscardCards.fill(Lang::En, &["03"]),
        "Discard 03 cards"
    );
    assert_eq!(
        Phrase::DiscardCards.fill(Lang::En, &["-0"]),
        "Discard -0 cards"
    );
    assert_eq!(
        Phrase::DiscardCards.fill(Lang::En, &["-4"]),
        "Discard -4 cards"
    );
    assert_eq!(
        Phrase::DiscardCards.fill(Lang::En, &["123456789012345"]),
        "Discard 123456789012345 cards"
    );
}

/// Two languages that say exactly the same thing everywhere would mean
/// the second one was never written. A handful of phrases genuinely are
/// the same word (`baylee`, `E-MAIL`), so this asks for most, not all.
#[test]
fn german_is_actually_german() {
    let same = Phrase::ALL
        .iter()
        .filter(|p| p.text(Lang::En) == p.text(Lang::De))
        .count();
    assert!(
        same * 5 < Phrase::ALL.len(),
        "{same} of {} phrases are untranslated",
        Phrase::ALL.len()
    );
}

#[test]
fn a_stored_code_names_a_language() {
    assert_eq!(Lang::of("de"), Lang::De);
    assert_eq!(Lang::of("DE"), Lang::De);
    // A regional code is the language it is a region of…
    assert_eq!(Lang::of("de-AT"), Lang::De);
    assert_eq!(Lang::of("en_GB"), Lang::En);
    // …and anything else is English rather than a refusal to start.
    assert_eq!(Lang::of("kl"), Lang::En);
    assert_eq!(Lang::of(""), Lang::En);
    assert_eq!(Lang::of("de").code(), "de");
}

#[test]
fn the_picker_walks_every_language_and_comes_back() {
    let mut lang = Lang::default();
    for _ in Lang::ALL {
        lang = lang.next();
    }
    assert_eq!(lang, Lang::default(), "the ring is not a ring");
}

#[test]
fn filling_a_phrase_puts_the_arguments_where_the_language_wants_them() {
    assert_eq!(
        Phrase::PageOf.fill(Lang::En, &["1", "8", "12"]),
        "1–8 of 12"
    );
    assert_eq!(
        Phrase::PageOf.fill(Lang::De, &["1", "8", "12"]),
        "1–8 von 12"
    );
    // An argument that was not supplied leaves its placeholder showing.
    assert!(Phrase::PageOf.fill(Lang::En, &["1"]).contains("{1}"));
}

/// The attachment boxes of the report form are a choice the player can
/// undo before sending, not a consent: the form, its tour step and its
/// settings row say so in plain words, in both languages. (Crash reports
/// are another matter and keep their own wording.)
#[test]
fn the_report_attachments_are_a_choice_and_not_a_consent() {
    let lower = |phrase: Phrase, lang: Lang| phrase.text(lang).to_lowercase();
    let claims_consent = |text: &str| text.contains("consent") || text.contains("einwilligung");
    // The control: the check finds the words it looks for.
    assert!(claims_consent("your consent"));
    assert!(claims_consent("deine einwilligung"));
    assert!(!claims_consent("a choice"));

    for phrase in [
        Phrase::TourT32Body,
        Phrase::HelpReportConsent,
        Phrase::RowReportConsent,
    ] {
        for lang in Lang::ALL {
            let text = lower(phrase, lang);
            assert!(!claims_consent(&text), "{phrase:?} {lang:?}: {text}");
        }
    }
    let en = lower(Phrase::TourT32Body, Lang::En);
    assert!(en.contains("a choice"), "{en}");
    assert!(
        en.contains("untick") && en.contains("before you send"),
        "{en}"
    );
    let de = lower(Phrase::TourT32Body, Lang::De);
    assert!(de.contains("eine wahl"), "{de}");
    assert!(
        de.contains("heraus") && de.contains("vor dem senden"),
        "{de}"
    );
}

/// Every table's files: each parses whole, holds every English id once and
/// nothing else, and each message uses the same arguments as its English
/// one, all of them among those its `# Arguments` line names. This is the
/// guarantee the `messages!` macro gave as a compile error: a phrase with no
/// German, or a German that drops a number. (A selector's variants count
/// together: a `[one]` that says no number is fine while `*[other]` does.)
#[test]
fn every_language_says_every_phrase_with_the_same_arguments() {
    use fluent_syntax::ast;
    use std::collections::BTreeSet;

    fn variables<'a>(pattern: &'a ast::Pattern<&'a str>, into: &mut BTreeSet<&'a str>) {
        for element in &pattern.elements {
            if let ast::PatternElement::Placeable { expression } = element {
                expression_variables(expression, into);
            }
        }
    }
    fn expression_variables<'a>(
        expression: &'a ast::Expression<&'a str>,
        into: &mut BTreeSet<&'a str>,
    ) {
        match expression {
            ast::Expression::Select { selector, variants } => {
                inline_variables(selector, into);
                for variant in variants {
                    variables(&variant.value, into);
                }
            }
            ast::Expression::Inline(inline) => inline_variables(inline, into),
        }
    }
    fn inline_variables<'a>(
        inline: &'a ast::InlineExpression<&'a str>,
        into: &mut BTreeSet<&'a str>,
    ) {
        match inline {
            ast::InlineExpression::VariableReference { id } => {
                into.insert(id.name);
            }
            ast::InlineExpression::Placeable { expression } => {
                expression_variables(expression, into);
            }
            ast::InlineExpression::FunctionReference { arguments, .. } => {
                for argument in &arguments.positional {
                    inline_variables(argument, into);
                }
            }
            _ => {}
        }
    }

    for table in [&INTERFACE_TABLE, &REPLAY_TABLE, &CHAT_TABLE, &SERVER_TABLE] {
        let ids = table.ids();
        let mut by_language = Vec::new();
        for (code, source) in table.sources() {
            let resource = fluent_syntax::parser::parse(*source)
                .unwrap_or_else(|(_, errors)| panic!("{code}: {errors:?}"));
            let mut uses: Vec<Option<BTreeSet<String>>> = vec![None; ids.len()];
            for entry in &resource.body {
                let ast::Entry::Message(message) = entry else {
                    continue;
                };
                let id = message.id.name;
                let Some(index) = ids.iter().position(|e| *e == id) else {
                    panic!("{code}: {id} is no English phrase");
                };
                assert!(uses[index].is_none(), "{code}: {id} twice");
                let mut used = BTreeSet::new();
                if let Some(value) = &message.value {
                    variables(value, &mut used);
                }
                uses[index] = Some(used.into_iter().map(str::to_owned).collect());
            }
            by_language.push((*code, uses));
        }
        let (_, english) = by_language
            .iter()
            .find(|(code, _)| *code == "en")
            .expect("an English file");
        for (code, uses) in &by_language {
            for (index, used) in uses.iter().enumerate() {
                let id = ids[index];
                let Some(used) = used else {
                    panic!("{code} lacks {id}");
                };
                let named: BTreeSet<String> = table.args()[index]
                    .iter()
                    .map(|a| (*a).to_owned())
                    .collect();
                assert!(
                    used.is_subset(&named),
                    "{code}: {id} uses {used:?}, its `# Arguments` line names {named:?}"
                );
                assert_eq!(
                    Some(used),
                    english[index].as_ref(),
                    "{code}: {id} says other arguments than English"
                );
            }
        }
    }
}
