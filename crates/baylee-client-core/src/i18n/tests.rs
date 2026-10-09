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

/// Word order is the translator's; the values are not. A `{0}` that is
/// dropped in one language is a name or a count the player never sees.
#[test]
fn a_translation_keeps_every_placeholder_it_was_given() {
    for phrase in Phrase::ALL {
        let english = phrase.slots(Lang::En);
        for lang in Lang::ALL {
            assert_eq!(
                phrase.slots(lang),
                english,
                "{phrase:?} loses or invents a placeholder in {lang:?}"
            );
        }
    }
}

/// `card(s)` is not a plural in any language, and the sheet greys what a
/// sentence says in brackets — so the broken form was drawn as an
/// editorial aside, in grey, right beside the number it disagreed with.
/// [`Phrase::counted`] is the way to say this; a bracketed suffix is not.
#[test]
fn no_phrase_fakes_a_plural_with_a_bracket() {
    for phrase in Phrase::ALL {
        for lang in Lang::ALL {
            let text = phrase.text(lang);
            for fake in ["(s)", "(n)", "(e)", "(en)", "(er)"] {
                assert!(
                    !text.contains(fake),
                    "{phrase:?} says {fake} in {lang:?} instead of being written twice"
                );
            }
        }
    }
}

/// A counted phrase is two literals, and the pair has to stay one
/// sentence: the same values in the same slots, or the singular quietly
/// drops the number it is counting.
#[test]
fn both_forms_of_a_counted_phrase_carry_the_same_values() {
    let pairs = [
        (Phrase::ShakyCard, Phrase::ShakyCards),
        (Phrase::PutCardOnBottom, Phrase::PutOnBottom),
        (Phrase::DiscardCard, Phrase::DiscardCards),
        (Phrase::NounCard, Phrase::NounCards),
        (Phrase::ReportConfirmCardRef, Phrase::ReportConfirmCardRefs),
        (
            Phrase::ReportConfirmPlayerName,
            Phrase::ReportConfirmPlayerNames,
        ),
        (Phrase::NounTarget, Phrase::NounTargets),
        (Phrase::NounCardFromLibrary, Phrase::NounCardsFromLibrary),
        (Phrase::NounCardToTop, Phrase::NounCardsToTop),
        (Phrase::NounCardOutside, Phrase::NounCardsOutside),
        (
            Phrase::NounPermanentToSacrifice,
            Phrase::NounPermanentsToSacrifice,
        ),
        (Phrase::NounCardToKeep, Phrase::NounCardsToKeep),
        (Phrase::NounLandToKeep, Phrase::NounLandsToKeep),
        (Phrase::NounCreatureToKeep, Phrase::NounCreaturesToKeep),
        (Phrase::NounPermanentToKeep, Phrase::NounPermanentsToKeep),
        (Phrase::NounCardToDiscard, Phrase::NounCardsToDiscard),
        (Phrase::NounPermanentToTap, Phrase::NounPermanentsToTap),
        (
            Phrase::NounPermanentToReturn,
            Phrase::NounPermanentsToReturn,
        ),
        (Phrase::NounCardToExile, Phrase::NounCardsToExile),
        (
            Phrase::NounPermanentToLeaveTapped,
            Phrase::NounPermanentsToLeaveTapped,
        ),
        (Phrase::NounPermanentToUntap, Phrase::NounPermanentsToUntap),
        (Phrase::NounCardToReveal, Phrase::NounCardsToReveal),
        (Phrase::NounAttackerToBand, Phrase::NounAttackersToBand),
        (
            Phrase::NounAttackerToBandWith,
            Phrase::NounAttackersToBandWith,
        ),
        (Phrase::NounCardToHand, Phrase::NounCardsToHand),
        (Phrase::NounCardToBottom, Phrase::NounCardsToBottom),
        (Phrase::NounCardToPlay, Phrase::NounCardsToPlay),
        (
            Phrase::NounCardToBattlefield,
            Phrase::NounCardsToBattlefield,
        ),
        (Phrase::NounCardToGraveyard, Phrase::NounCardsToGraveyard),
        (Phrase::NounCardForFirstPile, Phrase::NounCardsForFirstPile),
        (
            Phrase::NounCardFromGraveyard,
            Phrase::NounCardsFromGraveyard,
        ),
        (Phrase::LogKeptCardYou, Phrase::LogKeptCardsYou),
        (Phrase::LogKeptCard, Phrase::LogKeptCards),
        (Phrase::LogDrewCardYou, Phrase::LogDrewCardsYou),
        (Phrase::LogDrewCard, Phrase::LogDrewCards),
        (Phrase::LogLifePointYou, Phrase::LogLifePointsYou),
        (Phrase::LogLifePoint, Phrase::LogLifePoints),
        (Phrase::LogCounterPlus, Phrase::LogCountersPlus),
        (Phrase::LogCounterMinus, Phrase::LogCountersMinus),
        (Phrase::LogCounterLoyalty, Phrase::LogCountersLoyalty),
        (Phrase::LogCounterLore, Phrase::LogCountersLore),
        (Phrase::LogCounterTime, Phrase::LogCountersTime),
        (Phrase::LogCounterCharge, Phrase::LogCountersCharge),
        (Phrase::LogCounterPoison, Phrase::LogCountersPoison),
        (Phrase::LogCounterEnergy, Phrase::LogCountersEnergy),
        (Phrase::LogCounterRad, Phrase::LogCountersRad),
        (Phrase::LogCounterLifelink, Phrase::LogCountersLifelink),
        (Phrase::LogCounterLevel, Phrase::LogCountersLevel),
        (Phrase::LogCounterOther, Phrase::LogCountersOther),
    ];
    for (one, many) in pairs {
        for lang in Lang::ALL {
            assert_eq!(
                one.slots(lang),
                many.slots(lang),
                "{one:?} and {many:?} disagree about their values in {lang:?}"
            );
        }
        assert_eq!(Phrase::counted(1, one, many), one);
        for n in [0, 2, 7] {
            assert_eq!(Phrase::counted(n, one, many), many);
        }
    }
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
