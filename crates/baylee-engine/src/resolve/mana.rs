//! Mana production effects: flat/dynamic adds, land-color choices, and
//! delayed mana triggers.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

/// Executes one mana effect.
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::AddMana { color, amount } => {
            state.players[you.get() as usize]
                .mana_pool
                .add(color, amount);
            state.journal.record(GameEvent::ManaProduced {
                player: you,
                color,
                amount,
                source: Some(res.source),
            });
            None
        }
        Effect::AddManaDynamic { color, amount } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as u16;
            state.players[you.get() as usize].mana_pool.add(color, n);
            state.journal.record(GameEvent::ManaProduced {
                player: you,
                color,
                amount: n,
                source: Some(res.source),
            });
            None
        }
        Effect::AddManaLandColor { mine } => {
            // Union the producible mana of all lands of the chosen side.
            let mut colors = ColorSet::EMPTY;
            let mut colorless = false;
            for id in state.zones.list(ZoneLocation::Battlefield) {
                let Some(obj) = state.object(*id) else {
                    continue;
                };
                if !obj
                    .characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::LAND)
                {
                    continue;
                }
                let side_matches = if mine {
                    obj.controller == you
                } else {
                    obj.controller != you
                };
                if side_matches {
                    let c = obj.characteristics();
                    colors = colors.union(c.produced_colors);
                    colorless |= c.produced_colorless;
                }
            }
            let mut options: Vec<ManaColor> = [
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ]
            .into_iter()
            .filter(|c| {
                colors.contains(match c {
                    ManaColor::White => baylee_core::color::Color::White,
                    ManaColor::Blue => baylee_core::color::Color::Blue,
                    ManaColor::Black => baylee_core::color::Color::Black,
                    ManaColor::Red => baylee_core::color::Color::Red,
                    ManaColor::Green => baylee_core::color::Color::Green,
                    ManaColor::Colorless => return false,
                })
            })
            .collect();
            if colorless {
                options.push(ManaColor::Colorless);
            }
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::CommanderMana);
            Some(Pending::ChooseColor {
                player: you,
                options,
            })
        }
        Effect::DelayedManaAtNextFirstMain { color } => {
            let cmc = res
                .targets
                .first()
                .and_then(|t| state.object(*t))
                .map_or(0, |o| o.characteristics().mana_cost.cmc());
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::NextFirstMain,
                action: crate::state::DelayedAction::AddMana {
                    color,
                    amount: cmc as u16,
                },
            });
            None
        }
        _ => unreachable!("not a mana effect"),
    }
}
