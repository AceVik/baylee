//! Defender buttons and a bounded, named list of reversible assignments.
use super::{Duel, HINT_PT, Lang, Line, Phrase, palette, target_seat_name};
use baylee_client_core::{
    interaction::{AttackOption, CombatFocus},
    targeting,
};
use baylee_core::ids::Defender;

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
    let options = i.attack_options();
    if options.is_empty() {
        return;
    }
    let card = |id| {
        view.object(id).map_or_else(
            || Phrase::NounCard.text(lang).to_string(),
            |o| crate::face::name_of(o, view, texts),
        )
    };
    let defender = |d| match d {
        Defender::Player(p) => target_seat_name(duel, lang, view, p),
        Defender::Planeswalker(id) => card(id),
    };
    let CombatFocus::Defender(aim) = i.combat_focus() else {
        return;
    };
    let aim_name = defender(aim);
    lines.push(Line {
        text: Phrase::AttackDraftHint.text(lang).into(),
        size: HINT_PT,
        ink: palette::DOCK_INK,
    });
    // Defender and bulk controls stay on every page; only creatures page.
    let controls = options
        .iter()
        .take_while(|o| !matches!(o, AttackOption::Toggle(_)))
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
        .map(|(index, option)| {
            let label = match *option {
                AttackOption::Aim(d) => {
                    let count = i
                        .assignments()
                        .iter()
                        .filter(|(_, f)| *f == CombatFocus::Defender(d))
                        .count();
                    format!(
                        "{}{}",
                        if d == aim { "[x] " } else { "" },
                        Phrase::AttackAim.fill(lang, &[&defender(d), &count.to_string()])
                    )
                }
                AttackOption::AddRemaining => Phrase::AttackRemaining.fill(lang, &[&aim_name]),
                AttackOption::WithdrawAll => Phrase::AttackWithdrawAll.text(lang).into(),
                AttackOption::Toggle(id) => {
                    if let Some(CombatFocus::Defender(d)) = i.assignment(id) {
                        Phrase::AttackWithdraw.fill(lang, &[&card(id), &defender(d)])
                    } else {
                        Phrase::AttackSend.fill(lang, &[&card(id), &aim_name])
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::{
        Interaction,
        test_support::{ViewBuilder, token},
    };
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_engine::choice::{Pending, PlayerAction};

    fn duel() -> Duel {
        let view = ViewBuilder::new(3)
            .with_battlefield(
                0,
                (10..60)
                    .map(|id| token(id, 0, "Soldier", 1, 1))
                    .collect::<Vec<_>>(),
            )
            .build();
        let pending = Pending::ChooseAttackers {
            player: view.seat,
            attackers: view.battlefield.iter().map(|o| o.id).collect(),
            defenders: vec![
                Defender::Player(PlayerId::new(1)),
                Defender::Player(PlayerId::new(2)),
            ],
            required: Vec::new(),
            limits: Vec::new(),
        };
        Duel {
            interaction: Some(Interaction::new(pending, view.seat)),
            view: Some(view),
            ..Duel::default()
        }
    }
    fn rows(duel: &Duel, lang: Lang) -> Vec<crate::choices::ChoiceOption> {
        let mut rows = vec![];
        reading(
            duel,
            lang,
            &crate::cardtext::CardTexts::default(),
            &mut vec![],
            &mut rows,
        );
        rows
    }
    #[test]
    fn actual_choice_handler_edits_named_attacks_without_sending() {
        let mut duel = duel();
        let first = rows(&duel, Lang::En);
        assert!(first[0].label.starts_with("[x] Aim:"));
        crate::input::pick_choice(&mut duel, 4); // first creature, first defender
        crate::input::pick_choice(&mut duel, 1); // second defender
        crate::input::pick_choice(&mut duel, 2); // remaining creatures
        let i = duel.interaction.as_ref().unwrap();
        assert_eq!(i.declared(), 50);
        assert_eq!(
            i.assignment(ObjectId::new(10, 0)),
            Some(CombatFocus::Defender(Defender::Player(PlayerId::new(1))))
        );
        assert!(duel.outbox.is_empty());
        let after = rows(&duel, Lang::En);
        assert!(after[1].label.starts_with("[x] Aim:"));
        assert!(after[1].label.contains("49 assigned"));
        assert!(after[4].label.starts_with("Withdraw:"));
        assert!(rows(&duel, Lang::De)[4].label.starts_with("Zurückziehen:"));
        crate::input::pick_choice(&mut duel, 4); // withdraw first
        crate::input::pick_choice(&mut duel, 4); // send to selected second defender
        assert_eq!(
            duel.interaction
                .as_ref()
                .unwrap()
                .assignment(ObjectId::new(10, 0)),
            Some(CombatFocus::Defender(Defender::Player(PlayerId::new(2))))
        );
        crate::input::pick_choice(&mut duel, 3); // withdraw all
        assert_eq!(
            duel.interaction.as_ref().unwrap().confirm(),
            Some(PlayerAction::DeclareAttackers { attackers: vec![] })
        );
        assert!(duel.outbox.is_empty());
    }
    #[test]
    fn pagination_reaches_all_fifty_creatures_and_invalid_rows_do_nothing() {
        let mut duel = duel();
        let mut seen = Vec::new();
        loop {
            let page = rows(&duel, Lang::En);
            assert!(page.len() <= targeting::PAGE_SIZE + 6);
            assert_eq!(
                page.iter().take(4).map(|r| r.index).collect::<Vec<_>>(),
                vec![0, 1, 2, 3]
            );
            seen.extend(
                page.iter()
                    .filter(|r| (4..54).contains(&r.index))
                    .map(|r| r.index),
            );
            if !page.iter().any(|r| r.index == targeting::NEXT) {
                break;
            }
            crate::input::pick_choice(&mut duel, targeting::NEXT);
        }
        assert_eq!(seen, (4..54).collect::<Vec<_>>());
        crate::input::pick_choice(&mut duel, 2);
        assert_eq!(duel.interaction.as_ref().unwrap().declared(), 50);
        crate::input::pick_choice(&mut duel, 3);
        crate::input::pick_choice(&mut duel, 999);
        assert_eq!(duel.interaction.as_ref().unwrap().declared(), 0);
        assert!(duel.outbox.is_empty());
    }
}
