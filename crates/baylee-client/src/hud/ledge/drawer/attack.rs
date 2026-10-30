//! Categorised defender buttons and bounded, named reversible assignments.
use super::{Duel, HINT_PT, Lang, Line, Phrase, glyph, palette, target_seat_name};
use baylee_client_core::{
    interaction::{AttackOption, CombatFocus},
    targeting,
};
use baylee_core::ids::Defender;

#[allow(clippy::too_many_lines)] // one reading in visual order
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
            || Phrase::NounCards.fill_count(lang, 1, &[]),
            |o| crate::face::name_of(o, view, texts),
        )
    };
    let defender = |d| match d {
        Defender::Player(p) => target_seat_name(duel, lang, view, p),
        Defender::Planeswalker(id) | Defender::Battle(id) => card(id),
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
    if duel.attack_list_folded {
        *rows = vec![fold_row(lang, true, options.len())];
        return;
    }

    let page_count = crate::choices::attack_page_count(&options);
    let page = duel.target_page.min(page_count - 1);
    if page_count > 1 {
        lines.push(Line {
            text: Phrase::AttackPage
                .fill(lang, &[&(page + 1).to_string(), &page_count.to_string()]),
            size: HINT_PT,
            ink: palette::DOCK_INK,
        });
    }
    let defenders: Vec<(usize, Defender)> = options
        .iter()
        .enumerate()
        .filter_map(|(index, option)| match *option {
            AttackOption::Aim(defender) => Some((index, defender)),
            _ => None,
        })
        .collect();
    // Each group clamps to its own last page (`attack_page_count`).
    let shown = crate::choices::Page::of(defenders.len(), page, targeting::PAGE_SIZE);
    let mut groups: Vec<(String, char, Vec<crate::choices::ChoiceOption>)> = Vec::new();
    for &(index, d) in &defenders[shown.start..shown.end] {
        let count = i
            .assignments()
            .iter()
            .filter(|(_, f)| *f == CombatFocus::Defender(d))
            .count();
        let option = crate::choices::ChoiceOption::text(
            index,
            Phrase::AttackAim.fill(lang, &[&defender(d), &count.to_string()]),
        )
        .chosen(d == aim);
        let (category, icon) = match d {
            Defender::Player(_) => (Phrase::SheetPlayers.text(lang).to_string(), glyph::FRIENDS),
            Defender::Planeswalker(id) | Defender::Battle(id) => {
                let kind = if matches!(d, Defender::Planeswalker(_)) {
                    Phrase::GroupPlaneswalkers.text(lang)
                } else {
                    Phrase::GroupBattles.text(lang)
                };
                let icon = if matches!(d, Defender::Planeswalker(_)) {
                    glyph::CROWN
                } else {
                    glyph::COMMANDER_DAMAGE
                };
                let owner = view
                    .object(id)
                    .map(|object| target_seat_name(duel, lang, view, object.controller));
                (
                    owner.map_or_else(|| kind.to_string(), |owner| format!("{kind} · {owner}")),
                    icon,
                )
            }
        };
        if let Some((_, _, group)) = groups.iter_mut().find(|(heading, group_icon, _)| {
            heading.as_str() == category.as_str() && *group_icon == icon
        }) {
            group.push(option);
        } else {
            groups.push((category, icon, vec![option]));
        }
    }
    rows.clear();
    for (category, icon, mut group) in groups {
        group[0].heading = Some(crate::choices::ChoiceHeading {
            label: category,
            icon,
        });
        rows.extend(group);
    }

    for (index, option) in options.iter().enumerate().filter(|(_, option)| {
        matches!(
            option,
            AttackOption::AddRemaining | AttackOption::WithdrawAll
        )
    }) {
        let label = match option {
            AttackOption::AddRemaining => Phrase::AttackRemaining.fill(lang, &[&aim_name]),
            AttackOption::WithdrawAll => Phrase::AttackWithdrawAll.text(lang).into(),
            AttackOption::Aim(_) | AttackOption::Toggle(_) => unreachable!(),
        };
        rows.push(crate::choices::ChoiceOption::text(index, label));
    }

    let creatures: Vec<(usize, baylee_core::ids::ObjectId)> = options
        .iter()
        .enumerate()
        .filter_map(|(index, option)| match *option {
            AttackOption::Toggle(id) => Some((index, id)),
            _ => None,
        })
        .collect();
    let sent = crate::choices::Page::of(creatures.len(), page, targeting::PAGE_SIZE);
    for (offset, &(index, id)) in creatures[sent.start..sent.end].iter().enumerate() {
        let label = if let Some(CombatFocus::Defender(d)) = i.assignment(id) {
            Phrase::AttackWithdraw.fill(lang, &[&card(id), &defender(d)])
        } else {
            Phrase::AttackSend.fill(lang, &[&card(id), &aim_name])
        };
        let row = crate::choices::ChoiceOption::text(index, label);
        rows.push(if offset == 0 {
            row.headed(crate::choices::ChoiceHeading {
                label: if sent.count > 1 {
                    format!(
                        "{} · {}–{}/{}",
                        Phrase::GroupCreatures.text(lang),
                        sent.start + 1,
                        sent.end,
                        creatures.len()
                    )
                } else {
                    Phrase::GroupCreatures.text(lang).to_string()
                },
                icon: glyph::SEND,
            })
        } else {
            row
        });
    }
    crate::choices::page_arrows(rows, lang, page > 0, page + 1 < page_count);
    rows.push(fold_row(lang, false, options.len()));
}

/// The row that folds the attack chooser's list away, or opens it again.
/// Its index is in no engine's namespace, like the page arrows'.
pub(crate) const FOLD: usize = usize::MAX - 64;

fn fold_row(lang: Lang, folded: bool, count: usize) -> crate::choices::ChoiceOption {
    crate::choices::ChoiceOption::text(
        FOLD,
        if folded {
            Phrase::AttackListShow.fill(lang, &[&count.to_string()])
        } else {
            Phrase::AttackListHide.text(lang).into()
        },
    )
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
            alone: Vec::new(),
            minimum_required: None,
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
        assert!(first[0].chosen && first[0].label.starts_with("Aim:"));
        assert!(!first[1].chosen, "one defender is aimed at");
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
        assert!(after[1].chosen && !after[0].chosen);
        assert!(after[1].label.starts_with("Aim:"));
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
            // Four controls, a page of creatures, two arrows and the fold.
            assert!(page.len() <= targeting::PAGE_SIZE + 7);
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

    /// The attack report's mixed defenders: categories retain the engine's
    /// indices, planeswalkers and battles name their controllers, and defender
    /// pagination does not depend on there being a page of creatures.
    #[test]
    fn mixed_defenders_and_creatures_reach_every_original_index() {
        let own: Vec<_> = (10..12).map(|id| token(id, 0, "Soldier", 1, 1)).collect();
        let other: Vec<_> = (80..92)
            .map(|id| token(id, 1, "Walker", 0, 0))
            .chain([token(95, 2, "Siege", 0, 0)])
            .collect();
        let view = ViewBuilder::new(3)
            .with_battlefield(0, own)
            .with_battlefield(1, other)
            .build();
        let mut defenders = vec![
            Defender::Player(PlayerId::new(1)),
            Defender::Planeswalker(ObjectId::new(80, 0)),
            Defender::Battle(ObjectId::new(95, 0)),
            Defender::Player(PlayerId::new(2)),
        ];
        defenders.extend((81..92).map(|id| Defender::Planeswalker(ObjectId::new(id, 0))));
        let pending = Pending::ChooseAttackers {
            alone: Vec::new(),
            minimum_required: None,
            player: view.seat,
            attackers: view.battlefield.iter().map(|o| o.id).take(2).collect(),
            defenders,
            required: Vec::new(),
            limits: Vec::new(),
        };
        let mut duel = Duel {
            interaction: Some(Interaction::new(pending, view.seat)),
            view: Some(view),
            ..Duel::default()
        };
        let option_count = duel.interaction.as_ref().unwrap().attack_options().len();
        let mut seen = Vec::new();
        let mut pages = 0;
        for _ in 0..4 {
            let page = rows(&duel, Lang::En);
            pages += 1;
            if pages == 1 {
                let headings: Vec<_> = page.iter().filter_map(|row| row.heading.as_ref()).collect();
                assert!(headings.iter().any(|h| h.label == "Players"));
                assert!(
                    headings
                        .iter()
                        .any(|h| h.label.starts_with("Planeswalkers · "))
                );
                assert!(headings.iter().any(|h| h.label.starts_with("Battles · ")));
                assert!(headings.iter().any(|h| h.label.starts_with("Creatures")));
            }
            seen.extend(
                page.iter()
                    .map(|row| row.index)
                    .filter(|index| *index < option_count),
            );
            if !page.iter().any(|row| row.index == targeting::NEXT) {
                break;
            }
            crate::input::pick_choice(&mut duel, targeting::NEXT);
        }
        assert_eq!(pages, 2, "fifteen defenders need two pages");
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen, (0..option_count).collect::<Vec<_>>());
        crate::input::pick_choice(&mut duel, option_count - 1);
        assert_eq!(duel.interaction.as_ref().unwrap().declared(), 1);
        assert!(duel.outbox.is_empty());
    }

    /// Report 25: the list folds away to one row and opens again. The last
    /// row of an open list is the fold, "Hide the list"; picking it leaves
    /// one row, "Show the list (N)" with N the options it hides; picking
    /// that opens the list as it was. Folding is the client's own: nothing is
    /// sent, and the assignments made stay. On the old code there was no
    /// such row, so the first assertion fails.
    #[test]
    fn the_list_folds_to_one_row_and_opens_again_without_sending_or_forgetting() {
        let mut duel = duel();
        crate::input::pick_choice(&mut duel, 4); // first creature at the aimed defender
        let open = rows(&duel, Lang::En);
        let last = open.last().expect("rows");
        assert_eq!(last.index, FOLD, "the fold is the last row");
        assert_eq!(last.label, "Hide the list \u{25be}");
        let count = duel.interaction.as_ref().unwrap().attack_options().len();
        assert_eq!(count, 54, "four controls and fifty creatures");

        crate::input::pick_choice(&mut duel, FOLD);
        assert!(duel.attack_list_folded);
        let folded = rows(&duel, Lang::En);
        assert_eq!(folded.len(), 1, "one row stands for the list");
        assert_eq!(folded[0].index, FOLD);
        assert_eq!(folded[0].label, "Show the list (54) \u{25b8}");
        assert_eq!(rows(&duel, Lang::De)[0].label, "Liste zeigen (54) \u{25b8}");
        assert!(duel.outbox.is_empty(), "folding sends nothing");
        assert_eq!(
            duel.interaction.as_ref().unwrap().declared(),
            1,
            "and keeps what was assigned"
        );

        crate::input::pick_choice(&mut duel, FOLD);
        assert!(!duel.attack_list_folded);
        assert_eq!(rows(&duel, Lang::En), open, "opened again as it was");
        assert!(duel.outbox.is_empty());
        assert_eq!(duel.interaction.as_ref().unwrap().declared(), 1);
    }
}
