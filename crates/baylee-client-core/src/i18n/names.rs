use super::Phrase;
use super::lang::Lang;
use baylee_core::ids::PlayerId;
use baylee_view::GameStatic;

/// What the local seat's own tab is headed.
///
/// [`Phrase::YouNamed`] is "You ({0})" — the pronoun, and then whatever the
/// table calls this seat, because at a room of six "You" alone does not say
/// which chair is yours to anybody reading over your shoulder. The offline
/// `LocalHost` names seat 0 `"You"`, which is the pronoun itself, and the tab
/// then read **"You (You)"**.
///
/// So a name that already *is* this language's pronoun is drawn once. The
/// comparison is against [`Phrase::You`] rather than against a literal, or a
/// German table would go on saying "Du (Du)"; it ignores case, because the
/// name comes from a host and "you" is the same claim.
#[must_use]
pub fn own_seat_name(lang: Lang, name: &str) -> String {
    let pronoun = Phrase::You.text(lang);
    if name.trim().eq_ignore_ascii_case(pronoun) {
        pronoun.to_string()
    } else {
        Phrase::YouNamed.fill(lang, &[name])
    }
}

/// What a house AI's difficulty is called, from its wire spelling.
///
/// The wire value is an identifier — `"sharp"` is what a `GamePreset` holds
/// and what an HTTP route takes — and the root contract keeps it: *values
/// that are also identifiers keep their wire spelling and translate only the
/// label*. This is the label half, and it had only one caller, so the table
/// went on printing the identifier. A chair arranged in the lobby as
/// "Solide" sat down at the table called `steady 1`.
///
/// Unknown spellings are [`None`] rather than the middle difficulty, because
/// a caller that has a value the client does not know is in a different
/// situation from one that has none — and a wrong difficulty drawn
/// confidently is worse than no difficulty at all.
#[must_use]
pub fn ai_name(lang: Lang, wire: &str) -> Option<&'static str> {
    let phrase = match wire {
        "novice" => Phrase::AiNovice,
        "casual" => Phrase::AiCasual,
        "steady" => Phrase::AiSteady,
        "sharp" => Phrase::AiSharp,
        "expert" => Phrase::AiExpert,
        _ => return None,
    };
    Some(phrase.text(lang))
}

/// What to call a seat inside a sentence.
///
/// Every line that talks *about* another chair needs this, and four of them
/// were writing it out: a zone browser's tab, the player chooser's rows, the
/// bar's "waiting for" line and now a draw offer. Three spellings had grown
/// between them — `Phrase::SeatNumbered` in one, a developer's `#1` in
/// another, and a bare `PlayerId` in the third, which is how "Warte auf Platz
/// 1" was the best the prompt bar could say at a table where everyone has a
/// name.
///
/// The roster is [`Option`] because a seat is sent [`GameStatic`] once and a
/// client draws frames before it arrives; a seat it does not describe is
/// **numbered, not dropped**, because the sentence is about a chair that
/// exists either way.
///
/// This is for *another* seat. The viewing seat's own name is
/// [`own_seat_name`], which draws the pronoun instead — and no caller here
/// has to choose between them: a line that says "waiting for" or "offers a
/// draw" is never about the seat reading it.
#[must_use]
pub fn seat_name(lang: Lang, statics: Option<&GameStatic>, player: PlayerId) -> String {
    statics
        .and_then(|s| s.seats.iter().find(|seat| seat.player == player))
        .map_or_else(
            || Phrase::SeatNumbered.fill(lang, &[&player.get().to_string()]),
            |seat| seat.display_name.clone(),
        )
}
