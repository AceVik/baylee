use super::{
    Action, Book, Screen, api,
    draw::{label, pages, tile_action},
};
use crate::shellkit::{controls::Kit, tokens};
use api::puzzle::{Goal, Keyword, Move, Phase, Side, Spell, Target};
use baylee_client_core::{Lang, Phrase};
use bevy::prelude::*;

pub(super) fn act(book: &mut Book, action: Action) {
    let Screen::Door(door) = book.screen else {
        return;
    };
    let year = book.year();
    let concerns_door = |r: &api::Request| match r {
        api::Request::Puzzle { door: d, .. }
        | api::Request::Claim { door: d, .. }
        | api::Request::Hint { door: d, .. }
        | api::Request::Submit { door: d, .. } => *d == door,
        _ => false,
    };
    if book
        .pending
        .as_ref()
        .is_some_and(|p| concerns_door(&p.request))
        || book.work.iter().any(concerns_door)
    {
        return;
    }
    if matches!(action, Action::Claim)
        && book
            .door(door)
            .is_some_and(|d| matches!(d.state, api::DoorState::Solved | api::DoorState::Claimed))
    {
        book.push(api::Request::Claim { year, door });
        return;
    }
    let Some(p) = book.puzzle.as_mut().filter(|p| p.door == door) else {
        return;
    };
    match action {
        Action::Choose(m)
            if p.submitted.is_none()
                && p.puzzle
                    .choices(&p.line)
                    .is_ok_and(|choices| choices.contains(&m)) =>
        {
            p.line.0.push(m);
            book.inventory_page = 0;
        }
        Action::Reset => {
            p.line.0.clear();
            p.submitted = None;
            book.inventory_page = 0;
        }
        Action::Hint(number)
            if matches!(number, 1 | 2) && p.hints[usize::from(number - 1)].is_none() =>
        {
            book.push(api::Request::Hint { year, door, number })
        }
        Action::Submit
            if p.submitted.is_none()
                && p.puzzle
                    .position(&p.line)
                    .is_ok_and(|pos| pos.result.is_some()) =>
        {
            let line = p.line.clone();
            book.push(api::Request::Submit { year, door, line });
        }
        _ => {}
    }
}
fn spell(s: Spell) -> (Phrase, Phrase) {
    match s {
        Spell::Might => (Phrase::AdventMight, Phrase::AdventMightRule),
        Spell::Spark => (Phrase::AdventSpark, Phrase::AdventSparkRule),
        Spell::Bolt => (Phrase::AdventBolt, Phrase::AdventBoltRule),
        Spell::Frost => (Phrase::AdventFrost, Phrase::AdventFrostRule),
        Spell::Wings => (Phrase::AdventWings, Phrase::AdventWingsRule),
        Spell::Edge => (Phrase::AdventEdge, Phrase::AdventEdgeRule),
        Spell::Rally => (Phrase::AdventRally, Phrase::AdventRallyRule),
        Spell::Mend => (Phrase::AdventMend, Phrase::AdventMendRule),
    }
}
fn word(k: Keyword) -> Phrase {
    match k {
        Keyword::Flying => Phrase::AdventFlying,
        Keyword::Reach => Phrase::AdventReach,
        Keyword::FirstStrike => Phrase::AdventFirstStrike,
        Keyword::Deathtouch => Phrase::AdventDeathtouch,
        Keyword::Trample => Phrase::AdventTrample,
        Keyword::Lifelink => Phrase::AdventLifelink,
        Keyword::Vigilance => Phrase::AdventVigilance,
    }
}
fn side(s: Side) -> Phrase {
    if s == Side::You {
        Phrase::AdventYou
    } else {
        Phrase::AdventOpponent
    }
}
fn slot(n: u8) -> char {
    char::from(b'A' + n.min(2))
}
fn selected(mask: u8) -> String {
    (0..3)
        .filter(|&i| (mask & (1 << i)) != 0)
        .map(|i| slot(i).to_string())
        .collect::<Vec<_>>()
        .join(" + ")
}
fn pairs(assignments: [Option<u8>; 3]) -> String {
    assignments
        .iter()
        .enumerate()
        .filter_map(|(i, a)| Some(format!("{} → {}", slot(i as u8), slot((*a)?))))
        .collect::<Vec<_>>()
        .join(" · ")
}
fn move_name(m: Move, lang: Lang) -> String {
    match m {
        Move::Pass => Phrase::AdventPass.text(lang).to_owned(),
        Move::Cast { spell: s, target } => {
            let name = spell(s).0.text(lang);
            match target {
                None => name.to_owned(),
                Some(Target::Player(s)) => format!("{name} → {}", side(s).text(lang)),
                Some(Target::Unit { side: s, slot: i }) => {
                    format!("{name} → {} {}", side(s).text(lang), slot(i))
                }
            }
        }
        Move::Attack { attackers: 0 } => Phrase::AdventNoAttack.text(lang).to_owned(),
        Move::Attack { attackers } => Phrase::AdventAttackWith.fill(lang, &[&selected(attackers)]),
        Move::Block { assignments } if assignments == [None; 3] => {
            Phrase::AdventNoBlocks.text(lang).to_owned()
        }
        Move::Block { assignments } => pairs(assignments),
    }
}
fn plan(phase: Phase, lang: Lang) -> String {
    match phase {
        Phase::Cast => Phrase::AdventCast.text(lang).to_owned(),
        Phase::Attack {
            guards: [None, None, None],
        } => Phrase::AdventUnguarded.text(lang).to_owned(),
        Phase::Attack { guards } => Phrase::AdventGuards.fill(lang, &[&pairs(guards)]),
        Phase::Block { attackers } => Phrase::AdventAttackers.fill(lang, &[&selected(attackers)]),
    }
}

pub(super) fn draw(cmd: &mut Commands, parent: Entity, kit: Kit, b: &Book, door: u8) {
    let lang = kit.lang();
    label(
        cmd,
        parent,
        kit,
        &Phrase::AdventDoor.fill(lang, &[&door.to_string()]),
        tokens::GOLD,
    );
    match b.door(door).map(|d| d.state) {
        Some(api::DoorState::Solved | api::DoorState::Claimed) => {
            if b.puzzle
                .as_ref()
                .is_some_and(|p| p.door == door && p.submitted == Some(true))
                || b.door(door)
                    .is_some_and(|d| d.state == api::DoorState::Solved)
            {
                label(cmd, parent, kit, Phrase::AdventWon.text(lang), tokens::INK);
            }
            tile_action(
                cmd,
                parent,
                kit,
                Phrase::AdventReveal.text(lang),
                Action::Claim,
                "claim",
                0,
            );
            return;
        }
        Some(api::DoorState::Open) => {}
        _ => {
            label(
                cmd,
                parent,
                kit,
                Phrase::AdventLocked.text(lang),
                tokens::MUTED,
            );
            return;
        }
    }
    let Some(challenge) = b.puzzle.as_ref().filter(|p| p.door == door) else {
        label(
            cmd,
            parent,
            kit,
            Phrase::AdventLoading.text(lang),
            tokens::MUTED,
        );
        return;
    };
    let Ok(pos) = challenge.puzzle.position(&challenge.line) else {
        return;
    };
    label(
        cmd,
        parent,
        kit,
        match challenge.puzzle.goal {
            Goal::Survive => Phrase::AdventSurvive,
            Goal::Defeat => Phrase::AdventDefeat,
        }
        .text(lang),
        tokens::INK,
    );
    battlefield(cmd, parent, kit, &pos.board);
    timeline(cmd, parent, kit, &challenge.puzzle, pos.step);
    decisions(cmd, parent, kit, challenge, &pos, b.inventory_page);
    utilities(cmd, parent, kit, challenge);
}
fn battlefield(cmd: &mut Commands, parent: Entity, kit: Kit, board: &api::puzzle::Board) {
    let lang = kit.lang();
    let row = cmd
        .spawn(Node {
            width: percent(100),
            flex_wrap: FlexWrap::Wrap,
            column_gap: kit.m.px(10.),
            row_gap: kit.m.px(4.),
            ..default()
        })
        .id();
    cmd.entity(parent).add_child(row);
    for (s, side_id) in [Side::You, Side::Opponent].into_iter().enumerate() {
        let col = cmd
            .spawn((
                Node {
                    flex_grow: 1.,
                    min_width: kit.m.px(160.),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(kit.m.px(7.)),
                    ..default()
                },
                BackgroundColor(tokens::PANEL),
            ))
            .id();
        cmd.entity(row).add_child(col);
        let head = Phrase::AdventLife.fill(
            lang,
            &[side(side_id).text(lang), &board.life[s].to_string()],
        );
        label(cmd, col, kit, &head, tokens::ACCENT);
        for (i, piece) in board.units[s].iter().enumerate().filter(|(_, c)| c.alive) {
            let mut words = vec![format!(
                "{}  {}/{}",
                slot(i as u8),
                piece.power,
                piece.toughness
            )];
            words.extend(
                piece
                    .keywords
                    .iter()
                    .map(|&k| word(k).text(lang).to_owned()),
            );
            if piece.tapped {
                words.push(Phrase::AdventTapped.text(lang).to_owned());
            }
            if piece.damage > 0 {
                words.push(Phrase::AdventDamage.fill(lang, &[&piece.damage.to_string()]));
            }
            label(cmd, col, kit, &words.join(" · "), tokens::INK);
        }
    }
    if !board.hand.is_empty() {
        let mut seen = Vec::new();
        for &s in &board.hand {
            if seen.contains(&s) {
                continue;
            }
            seen.push(s);
            let (name, rule) = spell(s);
            label(
                cmd,
                parent,
                kit,
                &format!(
                    "{} ×{} · {} {} · {}",
                    name.text(lang),
                    board.hand.iter().filter(|&&v| v == s).count(),
                    s.cost(),
                    Phrase::AdventMana.text(lang),
                    rule.text(lang)
                ),
                tokens::MUTED,
            );
        }
    }
}
fn timeline(
    cmd: &mut Commands,
    parent: Entity,
    kit: Kit,
    puzzle: &api::puzzle::Puzzle,
    step: usize,
) {
    let lang = kit.lang();
    let step_info = puzzle
        .steps
        .iter()
        .skip(step)
        .enumerate()
        .map(|(offset, s)| {
            let mut words = vec![format!("{}. {}", step + offset + 1, plan(s.phase, lang))];
            if let Some(turn) = s.new_turn {
                words.push(format!(
                    "{} · {} {}",
                    side(turn.active).text(lang),
                    turn.mana,
                    Phrase::AdventMana.text(lang)
                ));
            }
            if let Some(card) = s.draw {
                words.push(Phrase::AdventDraw.fill(lang, &[spell(card).0.text(lang)]));
            }
            words.join(" · ")
        })
        .collect::<Vec<_>>();
    if !step_info.is_empty() {
        label(cmd, parent, kit, &step_info.join("   /   "), tokens::MUTED);
    }
}
fn decisions(
    cmd: &mut Commands,
    parent: Entity,
    kit: Kit,
    challenge: &super::Challenge,
    pos: &api::puzzle::Position,
    current_page: usize,
) {
    let lang = kit.lang();
    if pos.result.is_some() {
        if challenge.submitted == Some(false) {
            label(cmd, parent, kit, Phrase::AdventLost.text(lang), tokens::INK);
        } else {
            tile_action(
                cmd,
                parent,
                kit,
                Phrase::AdventSubmit.text(lang),
                Action::Submit,
                "submit",
                0,
            );
        }
    } else if let Ok(choices) = challenge.puzzle.choices(&challenge.line) {
        label(
            cmd,
            parent,
            kit,
            &Phrase::AdventChoose.fill(lang, &[&pos.board.mana.to_string()]),
            tokens::GOLD,
        );
        let page = current_page.min(choices.len().saturating_sub(1) / 8);
        let choices_row = cmd
            .spawn(Node {
                width: percent(100),
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.px(4.),
                row_gap: kit.m.px(4.),
                ..default()
            })
            .id();
        cmd.entity(parent).add_child(choices_row);
        for (i, &m) in choices.iter().skip(8 * page).take(8).enumerate() {
            tile_action(
                cmd,
                choices_row,
                kit,
                &move_name(m, lang),
                Action::Choose(m),
                "move",
                i as u8,
            );
        }
        pages(cmd, parent, kit, page, choices.len().div_ceil(8));
    }
}
fn utilities(cmd: &mut Commands, parent: Entity, kit: Kit, challenge: &super::Challenge) {
    let lang = kit.lang();
    let utility = cmd
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: kit.m.px(5.),
            ..default()
        })
        .id();
    cmd.entity(parent).add_child(utility);
    for n in 1..=2 {
        if let Some(t) = &challenge.hints[usize::from(n - 1)] {
            label(
                cmd,
                parent,
                kit,
                if lang == Lang::De { &t.de } else { &t.en },
                tokens::MUTED,
            );
        } else {
            tile_action(
                cmd,
                utility,
                kit,
                &Phrase::AdventHint.fill(lang, &[&n.to_string()]),
                Action::Hint(n),
                "hint",
                n,
            );
        }
    }
    if !challenge.line.0.is_empty() {
        tile_action(
            cmd,
            utility,
            kit,
            Phrase::AdventReset.text(lang),
            Action::Reset,
            "reset",
            0,
        );
    }
}
