//! The block declaration as a named list (report 31): every attacker, and the
//! creatures that may block the one in aim, each a row the table's own click
//! answers ([`baylee_client_core::interaction::BlockRow`]).
use super::{Duel, HINT_PT, Lang, Line, Phrase, palette};
use baylee_client_core::{interaction::BlockRow, targeting};

pub(super) fn reading(
    duel: &Duel,
    lang: Lang,
    texts: &crate::cardtext::CardTexts,
    lines: &mut Vec<Line>,
    rows: &mut Vec<crate::choices::ChoiceOption>,
) {
    let (Some(i), Some(view)) = (&duel.interaction, &duel.view) else {
        return;
    };
    let options = i.block_rows();
    let Some(aim) = i.block_aim() else {
        return;
    };
    if options.is_empty() {
        return;
    }
    let card = |id| {
        view.object(id).map_or_else(
            || Phrase::NounCard.text(lang).to_string(),
            |o| crate::face::name_of(o, view, texts),
        )
    };
    lines.push(Line {
        text: Phrase::BlockDraftHint.text(lang).into(),
        size: HINT_PT,
        ink: palette::DOCK_INK,
    });
    // The attackers stay on every page; only the blockers page.
    let controls = options
        .iter()
        .take_while(|o| matches!(o, BlockRow::Aim(_)))
        .count();
    let creatures = options.len() - controls;
    let start = duel
        .target_page
        .min(creatures.saturating_sub(1) / targeting::PAGE_SIZE)
        * targeting::PAGE_SIZE;
    let end = (start + targeting::PAGE_SIZE).min(creatures);
    *rows = options
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            *index < controls || (controls + start..controls + end).contains(index)
        })
        .map(|(index, row)| {
            let label = match *row {
                BlockRow::Aim(attacker) => {
                    let count = options
                        .iter()
                        .filter(|o| {
                            matches!(o, BlockRow::Toggle(b) if i.blocking(*b) == Some(attacker))
                        })
                        .count();
                    format!(
                        "{}{}",
                        if attacker == aim { "[x] " } else { "" },
                        Phrase::BlockAim.fill(lang, &[&card(attacker), &count.to_string()])
                    )
                }
                BlockRow::Toggle(blocker) => match i.blocking(blocker) {
                    Some(attacker) => {
                        Phrase::BlockWithdraw.fill(lang, &[&card(blocker), &card(attacker)])
                    }
                    None => Phrase::BlockSend.fill(lang, &[&card(blocker), &card(aim)]),
                },
            };
            crate::choices::ChoiceOption {
                index,
                label,
                pip: None,
                cost: None,
            }
        })
        .collect();
    for (show, index, label) in [
        (start > 0, targeting::PREVIOUS, "←"),
        (end < creatures, targeting::NEXT, "→"),
    ] {
        if show {
            rows.push(crate::choices::ChoiceOption {
                index,
                label: label.into(),
                pip: None,
                cost: None,
            });
        }
    }
}
