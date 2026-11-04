//! The deck's standing on the screen: the line under the list, the sheet that
//! chooses what it is held to, and the word a row wears.
//!
//! `DeckBuilder::overview` decides; this draws. A standing is read by its
//! mark as well as its ink, because colour alone says nothing to a good part
//! of the people looking at it, and [`Standing::Unknown`] never wears what
//! legal wears: a deck nobody could check is not a deck that passed.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::shellkit::surfaces::{self, SheetWidth};
use baylee_cardtext::Standing;
use baylee_client_core::deckbuilder::legality::Line;
use baylee_client_core::deckcheck::format_name;

/// How many problems stand under the list before the sheet is asked for all
/// of them: the foot shares its height with the list.
const FOOT_PROBLEMS: usize = 3;

/// A standing's mark and ink.
fn sign(standing: Standing) -> (char, Color) {
    match standing {
        Standing::Legal => (mark::CHECK, tokens::ACCENT),
        Standing::Illegal => (mark::WARN, tokens::DANGER),
        Standing::Unknown => (mark::HELP, tokens::GOLD),
    }
}

/// The overview's line: the mark, the headline, the deck's size and the way
/// to the sheet; under it the first problems, each a press on its card.
pub(super) fn foot(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let overview = env.deck().overview(lang);
    let (mark, ink) = sign(overview.standing);
    let sign = glyph(commands, kit, mark, m.small, ink);
    let headline = cell(commands, kit, &overview.headline, m.small, ink, true);
    let gap = spring(commands);
    let count = cell(
        commands,
        kit,
        &overview.count,
        m.small,
        tokens::MUTED,
        false,
    );
    let open = link(
        commands,
        kit,
        Phrase::CheckOpen.text(lang),
        (Press::Build(BuildPress::OpenCheck), stop("check")),
    );
    let row = line(commands, kit, &[sign, headline, gap, count, open]);
    commands.entity(holder).add_child(row);
    for problem in overview.problems.iter().take(FOOT_PROBLEMS) {
        let said = problem_line(commands, kit, problem, "problem");
        commands.entity(holder).add_child(said);
    }
    if overview.problems.len() > FOOT_PROBLEMS {
        let more = Phrase::CheckMore.fill_count(lang, overview.problems.len() - FOOT_PROBLEMS, &[]);
        let more = link(
            commands,
            kit,
            &more,
            (Press::Build(BuildPress::OpenCheck), stop("check-more")),
        );
        commands.entity(holder).add_child(more);
    }
}

/// One problem: a press on its card where it has one, else its words.
fn problem_line(commands: &mut Commands, kit: Kit, problem: &Line, id: &'static str) -> Entity {
    match problem.slot {
        Some(slot) => link(
            commands,
            kit,
            &problem.text,
            (Press::Build(BuildPress::CheckCard(slot)), stop(id)),
        ),
        None => cell(
            commands,
            kit,
            &problem.text,
            kit.m.small,
            tokens::DANGER,
            false,
        ),
    }
}

/// The word a deck row wears, drawn as a small plate of danger ink.
pub(super) fn badge(commands: &mut Commands, kit: Kit, word: &str) -> Entity {
    let m = kit.m;
    let plate = commands
        .spawn((
            Node {
                padding: UiRect::axes(m.px(6.0), m.px(1.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(tokens::DANGER),
            Pickable::IGNORE,
        ))
        .id();
    let words = controls::label(commands, kit, word, m.small * 0.9, tokens::DANGER);
    commands.entity(plate).add_child(words);
    plate
}

/// The sheet: what the deck is held to, and everything the check found.
pub(super) fn sheet(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let overview = deck.overview(lang);
    let mut body = Vec::new();

    body.push(caption(commands, kit, Phrase::CheckFormat.text(lang)));
    let chosen = deck.format();
    let chips: Vec<Entity> = deck
        .format_choices()
        .iter()
        .enumerate()
        .map(|(at, key)| {
            controls::chip(
                commands,
                kit,
                format_name(key, lang),
                key == chosen,
                None,
                false,
                (
                    Press::Build(BuildPress::SetFormat(at)),
                    sheet_stop("format"),
                ),
            )
        })
        .collect();
    body.push(wrapped(commands, kit, &chips));
    super::lists::picker(commands, env, &mut body);

    let (mark, ink) = sign(overview.standing);
    let sign = glyph(commands, kit, mark, m.text, ink);
    let headline = cell(commands, kit, &overview.headline, m.text, ink, true);
    let gap = spring(commands);
    let count = cell(
        commands,
        kit,
        &overview.count,
        m.small,
        tokens::MUTED,
        false,
    );
    body.push(line(commands, kit, &[sign, headline, gap, count]));
    for problem in &overview.problems {
        body.push(problem_line(commands, kit, problem, "sheet-problem"));
    }
    for notice in &overview.notices {
        body.push(surfaces::prose(commands, kit, notice, false));
    }
    let close = controls::button(
        commands,
        kit,
        Phrase::ShellClose.text(lang),
        Weight::Primary,
        Live::Yes,
        None,
        (Press::Build(BuildPress::CloseCheck), sheet_stop("close")),
    );
    surfaces::sheet_box_listed(
        commands,
        kit,
        SheetWidth::Medium,
        Phrase::CheckTitle.text(lang),
        &body,
        &[close],
        crate::lobby::Scrollable(crate::lobby::List::Check),
    )
}

/// Chips in rows that wrap.
pub(super) fn wrapped(commands: &mut Commands, kit: Kit, chips: &[Entity]) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.px(6.0),
                row_gap: kit.m.px(6.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(chips);
    row
}
