//! Which line of a printing stands for which line of the Oracle.

use crate::text::{fold, reminder_only, repair_braces, sentences};

/// Whether a printing's face says nothing the English Oracle does not.
///
/// True when the face has no printed text at all (`printed_text` is NULL —
/// Elven Palisade's only German printing, exo) and when the text it has is
/// the Oracle's words: a German-language row whose text Scryfall never
/// translated (Cascade Bluffs, eoc; Kavaron, eoe) or an English one. The
/// comparison is made after repairing braces, dropping reminders and folding
/// spacing and case, because those rows differ from the Oracle in exactly
/// that — `{UU}` for `{U}{U}`, a borderless frame's missing reminder — and
/// are still English.
///
/// That is a language-neutral rule on purpose. Over the German printings of
/// this pool it finds all 110 untranslated faces, where byte equality finds
/// 87, and leaves no picked face reading English (measured 2026-09-24 over
/// the local catalog). A list of German words would have been one language's
/// special case.
///
/// A face whose whole text is a reminder folds to nothing in any language,
/// so for it the reminders are kept: a German Forest's `({T}: Erzeuge {G}.)`
/// is a translation.
#[must_use]
pub fn untranslated(oracle: &str, printed: Option<&str>) -> bool {
    let Some(printed) = printed else {
        return true;
    };
    let folded = fold(oracle, false);
    if folded.is_empty() {
        fold(printed, true) == fold(oracle, true)
    } else {
        fold(printed, false) == folded
    }
}

/// How a face's lines were brought level with the Oracle's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    /// As printed: the face has as many lines as the Oracle.
    Raw,
    /// After removing lines that are not rules text: Scryfall's `//…//`
    /// markers (a Class's `//Level_2//`), an Adventure's second half from
    /// `//ADV//` on, and — on a modal double-faced card only — the other
    /// face's type and ability printed as two hint lines at the bottom.
    Markers,
    /// After also setting reminder-only lines aside on both sides: a
    /// borderless frame that drops `({T}: Add {G} or {W}.)` still lines up
    /// with the regular one that prints it.
    Reminders,
}

/// A face's lines, placed against the Oracle's.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Aligned {
    /// Which stage brought the counts level.
    pub stage: Stage,
    /// For each Oracle sentence, the printed sentence in its position, with
    /// its braces repaired ([`repair_braces`]) so that everything reading
    /// it — a key, a cost cut, a renderer drawing pips — sees Scryfall's
    /// symbols.
    ///
    /// `None` only at [`Stage::Reminders`], for an Oracle sentence that is
    /// itself a reminder and was set aside: it has no counterpart.
    pub lines: Vec<Option<String>>,
}

/// Places a printed face's sentences against the Oracle's, or refuses.
///
/// The stages are tried in order, and the order is the rule: each one is
/// only reached when the one before it did not already line the face up.
/// Removing reminder-only lines first would have been simpler and is wrong —
/// pre-Defender Walls print `(Mauern können nicht angreifen)` on a line of
/// its own where the Oracle prints `Defender (…)` on one, so the raw face
/// lines up (Flowstone Wall, nem) and the reminder stage would un-align it.
///
/// The double-faced hint is removed only when `layout` is `modal_dfc`.
/// Ungated, the same test damaged 1724 German faces that already lined up
/// (measured 2026-09-24): plenty of ordinary cards end in a line with `{T}`
/// after a line with no braces.
///
/// `None` is a refusal — Spawning Pool's 10e printing glues its first two
/// lines together — and every row on the face then falls to the Oracle.
#[must_use]
pub fn align(oracle: &str, printed: &str, layout: &str) -> Option<Aligned> {
    let want: Vec<&str> = sentences(oracle).collect();
    let raw: Vec<&str> = sentences(printed).collect();
    if raw.len() == want.len() {
        return Some(one_to_one(Stage::Raw, &raw));
    }
    let stripped = strip_markers(&raw, layout);
    if stripped.len() == want.len() {
        return Some(one_to_one(Stage::Markers, &stripped));
    }
    let mut kept = stripped.iter().filter(|l| !reminder_only(l));
    if kept.clone().count() != want.iter().filter(|l| !reminder_only(l)).count() {
        return None;
    }
    let lines = want
        .iter()
        .map(|l| {
            if reminder_only(l) {
                None
            } else {
                kept.next().map(|&l| repair_braces(l).into_owned())
            }
        })
        .collect();
    Some(Aligned {
        stage: Stage::Reminders,
        lines,
    })
}

fn one_to_one(stage: Stage, lines: &[&str]) -> Aligned {
    Aligned {
        stage,
        lines: lines
            .iter()
            .map(|&l| Some(repair_braces(l).into_owned()))
            .collect(),
    }
}

/// The lines left once Scryfall's markers and a double-faced hint are gone.
fn strip_markers<'a>(lines: &[&'a str], layout: &str) -> Vec<&'a str> {
    let mut out: Vec<&str> = lines
        .iter()
        .take_while(|l| !l.contains("//ADV//"))
        .filter(|l| !is_marker(l))
        .copied()
        .collect();
    if layout == "modal_dfc"
        && let [.., before, last] = out[..]
        && (is_bare_cost(last) || last.contains("{T}"))
        && !before.contains('{')
    {
        out.truncate(out.len() - 2);
    }
    out
}

/// `//Level_2//`, `//PRT-…//`: a line Scryfall uses as a separator.
fn is_marker(line: &str) -> bool {
    line.len() >= 4 && line.starts_with("//") && line.ends_with("//")
}

/// `{3}{W}` and nothing else: the other face's mana cost, as a hint prints it.
fn is_bare_cost(line: &str) -> bool {
    let mut rest = line;
    if rest.is_empty() {
        return false;
    }
    while !rest.is_empty() {
        let Some(body) = rest.strip_prefix('{') else {
            return false;
        };
        let Some(close) = body.find('}') else {
            return false;
        };
        if close == 0 || body[..close].contains('{') {
            return false;
        }
        rest = &body[close + 1..];
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(a: &Aligned) -> Vec<Option<&str>> {
        a.lines.iter().map(Option::as_deref).collect()
    }

    #[test]
    fn a_face_with_the_oracle_s_count_lines_up_as_printed() {
        let got = align(
            "Flying\n{T}: Add {G}.",
            "Fliegend\n{T}: Erzeuge {G}.",
            "normal",
        )
        .unwrap();
        assert_eq!(got.stage, Stage::Raw);
        assert_eq!(lines(&got), [Some("Fliegend"), Some("{T}: Erzeuge {G}.")]);
    }

    #[test]
    fn markers_and_an_adventure_s_second_half_are_not_rules_lines() {
        let got = align("A\nB", "a\n//Level_2//\nb", "class").unwrap();
        assert_eq!(
            (got.stage, lines(&got)),
            (Stage::Markers, vec![Some("a"), Some("b")])
        );
        let got = align("A\nB", "a\nb\n//ADV//\nName\n{1}{U}", "adventure").unwrap();
        assert_eq!(
            (got.stage, lines(&got)),
            (Stage::Markers, vec![Some("a"), Some("b")])
        );
    }

    /// Both branches of the layout gate: the same two trailing lines are a
    /// hint on a modal double-faced card and rules text on anything else.
    #[test]
    fn the_double_faced_hint_is_removed_only_from_a_modal_double_faced_card() {
        let oracle = "{T}: Add {U}.";
        let printed = "{T}: Erzeuge {U}.\nLand\n{T}: Erzeuge {B}.";
        let got = align(oracle, printed, "modal_dfc").unwrap();
        assert_eq!(
            (got.stage, lines(&got)),
            (Stage::Markers, vec![Some("{T}: Erzeuge {U}.")])
        );
        assert_eq!(align(oracle, printed, "normal"), None);
    }

    #[test]
    fn a_hint_ending_in_a_mana_cost_is_removed_too() {
        let oracle = "As this land enters, you may pay 3 life.\n{T}: Add {W}.";
        let printed = "Sowie …\n{T}: Erzeuge {W}.\nMensch\n{3}{W}";
        assert_eq!(
            align(oracle, printed, "modal_dfc").unwrap().stage,
            Stage::Markers
        );
    }

    #[test]
    fn a_reminder_line_one_printing_drops_is_set_aside_on_both_sides() {
        let oracle = "({T}: Add {B} or {G}.)\nAs this land enters, you may pay 2 life.";
        let got = align(oracle, "Sowie die Grabstätte ins Spiel kommt …", "normal").unwrap();
        assert_eq!(got.stage, Stage::Reminders);
        assert_eq!(
            lines(&got),
            [None, Some("Sowie die Grabstätte ins Spiel kommt …")]
        );
    }

    /// The monotone half: a face that lines up raw is never handed to the
    /// reminder stage, which would un-align it.
    #[test]
    fn a_wall_that_lines_up_raw_stays_lined_up() {
        let oracle = "Defender (This creature can't attack.)\n{R}: This creature gets +1/-1 until end of turn.";
        let printed = "(Mauern können nicht angreifen)\n{R}: Die Schmelzsteinmauer erhält +1/-1.";
        assert_eq!(align(oracle, printed, "normal").unwrap().stage, Stage::Raw);
    }

    #[test]
    fn a_placed_line_is_spelled_in_scryfall_s_symbols() {
        let oracle = "{W/B}, {T}: Add {W}{W}.";
        let got = align(oracle, "{(}w/b)}, {T}: Erzeuge {WW}.", "normal").unwrap();
        assert_eq!(lines(&got), [Some("{W/B}, {T}: Erzeuge {W}{W}.")]);
    }

    #[test]
    fn a_face_that_cannot_be_brought_level_is_refused() {
        let oracle =
            "This land enters tapped.\n{T}: Add {B}.\n{1}{B}: This land becomes a creature.";
        let printed = "Das Laichbecken kommt getappt ins Spiel.{T}: Erhöhe um {B}.\n{1}{B}: Wird zur Kreatur.";
        assert_eq!(align(oracle, printed, "normal"), None);
    }

    #[test]
    fn untranslated_is_null_or_the_oracle_s_own_words() {
        let oracle = "{T}: Add {C}.\n{U/R}, {T}: Add {U}{U}, {U}{R}, or {R}{R}.";
        assert!(untranslated(oracle, None));
        assert!(untranslated(
            oracle,
            Some("{T}: Add {C}.\n{(u/r)}, {T}: Add {UU}, {UR}, or {RR}.")
        ));
        assert!(!untranslated(
            oracle,
            Some("{T}: Erzeuge {C}.\n{U/R}, {T}: Erzeuge {U}{U}, {U}{R} oder {R}{R}.")
        ));
    }

    /// The reminder-only branch: two texts that fold to nothing are compared
    /// with their reminders in.
    #[test]
    fn a_translated_reminder_only_text_is_a_translation() {
        assert!(!untranslated(
            "({T}: Add {G}.)",
            Some("({T}: Erzeuge {G}.)")
        ));
        assert!(untranslated("({T}: Add {G}.)", Some("({T}: Add {G}.)")));
    }

    #[test]
    fn a_bare_cost_is_braces_and_nothing_else() {
        assert!(is_bare_cost("{3}{W}"));
        assert!(is_bare_cost("{U}"));
        assert!(!is_bare_cost("{3}{W} Mensch"));
        assert!(!is_bare_cost("Mensch"));
        assert!(!is_bare_cost(""));
        assert!(!is_bare_cost("{}"));
    }
}
