//! The block declaration as a named list (report 31): every attacker, and the
//! creatures that may block the one in aim, each a row the table's own click
//! answers ([`baylee_client_core::interaction::BlockRow`]).
use super::{Duel, HINT_PT, Lang, Line, Phrase, glyph, palette};
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
            || Phrase::NounCards.fill_count(lang, 1, &[]),
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
    let page = crate::choices::Page::of(
        options.len() - controls,
        duel.target_page,
        targeting::PAGE_SIZE,
    );
    *rows = options
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            *index < controls || (controls + page.start..controls + page.end).contains(index)
        })
        .map(|(index, row)| match *row {
            BlockRow::Aim(attacker) => {
                let count = options
                    .iter()
                    .filter(
                        |o| matches!(o, BlockRow::Toggle(b) if i.blocking(*b) == Some(attacker)),
                    )
                    .count();
                crate::choices::ChoiceOption::text(
                    index,
                    Phrase::BlockAim.fill(lang, &[&card(attacker), &count.to_string()]),
                )
                .chosen(attacker == aim)
            }
            BlockRow::Toggle(blocker) => crate::choices::ChoiceOption::text(
                index,
                match i.blocking(blocker) {
                    Some(attacker) => {
                        Phrase::BlockWithdraw.fill(lang, &[&card(blocker), &card(attacker)])
                    }
                    None => Phrase::BlockSend.fill(lang, &[&card(blocker), &card(aim)]),
                },
            ),
        })
        .collect();
    // Grouped as the attack list is: the attackers, then the creatures that
    // may block the one in aim, each under its caption (they ran on in one
    // wrap, an attacker beside a blocker).
    if let Some(first) = rows.first_mut() {
        first.heading = Some(crate::choices::ChoiceHeading {
            label: Phrase::RailAttackers.text(lang).into(),
            icon: glyph::SEND,
        });
    }
    let blockers = options.len() - controls;
    if let Some(first) = rows.get_mut(controls) {
        first.heading = Some(crate::choices::ChoiceHeading {
            label: if page.count > 1 {
                format!(
                    "{} · {}–{}/{}",
                    Phrase::RailBlockers.text(lang),
                    page.start + 1,
                    page.end,
                    blockers
                )
            } else {
                Phrase::RailBlockers.text(lang).to_string()
            },
            icon: glyph::SHIELD,
        });
    }
    crate::choices::page_arrows(rows, lang, page.earlier(), page.later());
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::{
        Interaction,
        test_support::{ViewBuilder, token},
    };
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_engine::choice::{BlockOption, Pending, PlayerAction};

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// The opponent's Earth Elemental (1) and Sky Drake (2) attack; my Wall
    /// (10) may block only the first, my Archer (11) either, my Hawk (12)
    /// only the second.
    fn duel() -> Duel {
        duel_with(vec![
            BlockOption {
                blocker: obj(10),
                attackers: vec![obj(1)],
            },
            BlockOption {
                blocker: obj(11),
                attackers: vec![obj(1), obj(2)],
            },
            BlockOption {
                blocker: obj(12),
                attackers: vec![obj(2)],
            },
        ])
    }

    fn duel_with(blockers: Vec<BlockOption>) -> Duel {
        let view = ViewBuilder::new(2)
            .with_battlefield(
                0,
                vec![
                    token(10, 0, "Wall", 0, 4),
                    token(11, 0, "Archer", 1, 1),
                    token(12, 0, "Hawk", 1, 1),
                ],
            )
            .with_battlefield(
                1,
                vec![
                    token(1, 1, "Earth Elemental", 4, 4),
                    token(2, 1, "Sky Drake", 2, 2),
                ],
            )
            .build();
        let pending = Pending::ChooseBlockers {
            demands: Vec::new(),
            player: view.seat,
            attacker: PlayerId::new(1),
            blockers,
            capacity: Vec::new(),
            obeying: Vec::new(),
            bounds: Vec::new(),
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

    /// The rows as read, a chosen one marked `[x] ` here (it is drawn taken,
    /// never written so).
    fn labels(duel: &Duel) -> Vec<String> {
        rows(duel, Lang::En)
            .into_iter()
            .map(|r| {
                assert!(!r.label.starts_with("[x]"), "the mark is in the words");
                if r.chosen {
                    format!("[x] {}", r.label)
                } else {
                    r.label
                }
            })
            .collect()
    }

    /// The list is what the player reads: every attacker with the aimed one
    /// marked, then a "Block" row for each creature that may block it.
    #[test]
    fn the_block_list_names_the_attackers_and_who_may_block_the_aimed_one() {
        let duel = duel();
        assert_eq!(
            labels(&duel),
            vec![
                "[x] Attacker: Earth Elemental · 0 blocking",
                "Attacker: Sky Drake · 0 blocking",
                "Block: Wall → Earth Elemental",
                "Block: Archer → Earth Elemental",
            ],
            "Hawk may only block the Drake and has no row while the Elemental is aimed"
        );
        assert_eq!(
            rows(&duel, Lang::En)
                .iter()
                .map(|r| r.index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3],
            "a row's index is its place in block_rows()"
        );
        assert_eq!(
            rows(&duel, Lang::De)[2].label,
            "Blocken: Wall → Earth Elemental"
        );
        let captions: Vec<Option<String>> = rows(&duel, Lang::En)
            .into_iter()
            .map(|r| r.heading.map(|h| h.label))
            .collect();
        assert_eq!(
            captions,
            [
                Some("Attackers".into()),
                None,
                Some("Blockers".into()),
                None
            ],
            "the attackers and the blockers each open under their caption"
        );
    }

    /// Choosing a "Block" row blocks, the same row then reads "Take back",
    /// and neither sends anything: the confirmation is the only send.
    #[test]
    fn picking_a_block_row_blocks_then_reads_take_back_and_sends_nothing() {
        let mut duel = duel();
        crate::input::pick_choice(&mut duel, 2);
        assert_eq!(
            duel.interaction.as_ref().unwrap().blocking(obj(10)),
            Some(obj(1)),
            "the Wall should block the Elemental"
        );
        assert_eq!(
            labels(&duel),
            vec![
                "[x] Attacker: Earth Elemental · 1 blocking",
                "Attacker: Sky Drake · 0 blocking",
                "Take back: Wall → Earth Elemental",
                "Block: Archer → Earth Elemental",
            ]
        );
        assert!(duel.outbox.is_empty(), "a row must not send the block");

        crate::input::pick_choice(&mut duel, 2);
        assert_eq!(
            duel.interaction.as_ref().unwrap().blocking(obj(10)),
            None,
            "the Take back row should have undone the block"
        );
        assert_eq!(labels(&duel)[2], "Block: Wall → Earth Elemental");
        assert!(duel.outbox.is_empty());
        assert_eq!(
            duel.interaction.as_ref().unwrap().confirm(),
            Some(PlayerAction::DeclareBlockers { blockers: vec![] })
        );
    }

    /// Choosing an attacker row moves the aim: the marker, the blockers
    /// offered and the new block's target follow it, and a creature already
    /// blocking keeps its "Take back" row.
    #[test]
    fn picking_an_attacker_row_moves_the_aim_and_keeps_the_take_back_row() {
        let mut duel = duel();
        crate::input::pick_choice(&mut duel, 2); // Wall blocks the Elemental
        crate::input::pick_choice(&mut duel, 1); // aim at the Drake
        assert_eq!(
            labels(&duel),
            vec![
                "Attacker: Earth Elemental · 1 blocking",
                "[x] Attacker: Sky Drake · 0 blocking",
                "Take back: Wall → Earth Elemental",
                "Block: Archer → Sky Drake",
                "Block: Hawk → Sky Drake",
            ],
            "the Wall cannot block the Drake but must stay takeable"
        );
        crate::input::pick_choice(&mut duel, 4); // Hawk blocks the Drake
        assert_eq!(
            duel.interaction.as_ref().unwrap().blocking(obj(12)),
            Some(obj(2))
        );
        assert!(duel.outbox.is_empty());
        assert_eq!(
            duel.interaction.as_ref().unwrap().confirm(),
            Some(PlayerAction::DeclareBlockers {
                blockers: vec![(obj(10), obj(1)), (obj(12), obj(2))]
            })
        );
    }

    /// A pick past the end of the list is the click of a row that is not
    /// there any more (the aim moved under it): it changes nothing.
    #[test]
    fn picking_a_row_that_is_not_listed_changes_nothing() {
        let mut duel = duel();
        crate::input::pick_choice(&mut duel, 99);
        let i = duel.interaction.as_ref().unwrap();
        assert_eq!(i.declared(), 0, "nothing should be blocking");
        assert_eq!(i.block_aim(), Some(obj(1)), "the aim should not move");
        assert!(duel.outbox.is_empty());
    }

    /// Past a page of blockers the list pages, the attackers stay on every
    /// page, and a row on the second page still means its place in the whole
    /// list.
    #[test]
    fn a_long_list_pages_its_blockers_and_keeps_the_attackers_on_every_page() {
        let mut duel = duel_with(
            (100..120)
                .map(|id| BlockOption {
                    blocker: obj(id),
                    attackers: vec![obj(1)],
                })
                .collect(),
        );
        let first = rows(&duel, Lang::En);
        let page = baylee_client_core::targeting::PAGE_SIZE;
        assert_eq!(
            first.iter().filter(|r| r.index < 1).count(),
            1,
            "the one attacker on the first page"
        );
        assert_eq!(
            first.len(),
            1 + page + 1,
            "attackers, a page, and a next arrow"
        );
        assert!(
            first
                .iter()
                .any(|r| r.index == baylee_client_core::targeting::NEXT)
        );

        crate::input::pick_choice(&mut duel, baylee_client_core::targeting::NEXT);
        let second = rows(&duel, Lang::En);
        assert_eq!(
            second.iter().filter(|r| r.index < 1).count(),
            1,
            "the attackers stay on the second page"
        );
        let creature = second
            .iter()
            .find(|r| (1..usize::MAX - 1).contains(&r.index) && r.label.starts_with("Block:"))
            .expect("the second page lists blockers");
        assert!(
            creature.index > page,
            "a second-page row keeps its place in the whole list, not in the page: {}",
            creature.index
        );
        crate::input::pick_choice(&mut duel, creature.index);
        assert_eq!(
            duel.interaction.as_ref().unwrap().declared(),
            1,
            "the row picked should have blocked exactly one creature"
        );
        assert!(duel.outbox.is_empty());
    }
}
