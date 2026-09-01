//! Mana production, and delayed mana triggers.
//!
//! One effect covers all of it. Which colors are available, how much mana
//! there is, and what it may be spent on are three independent questions,
//! and a card that answers them in a new combination needs no new code
//! here — only the color list is game state (commander identity, the lands
//! on the battlefield), so that is the one thing resolved on the spot.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

use baylee_cards_dsl::effect::{ManaRestriction, ManaSource};

/// Executes one mana effect.
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    match op {
        Effect::AddMana {
            source,
            amount,
            combination,
            restriction,
        } => add_mana(state, res, source, &amount, combination, restriction),
        Effect::DelayedManaAtNextFirstMain { color } => {
            let cmc = res
                .targets
                .first()
                .and_then(|t| state.object(*t))
                .map_or(0, |o| o.characteristics().mana_cost.cmc());
            state.delayed.push(crate::state::DelayedTrigger {
                controller: res.controller,
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

/// Adds the mana, asking for a color first when there is more than one.
fn add_mana(
    state: &mut GameState,
    res: &mut Resolution,
    source: ManaSource,
    amount: &Amount,
    combination: bool,
    restriction: Option<ManaRestriction>,
) -> Option<Pending> {
    let you = res.controller;
    let n = amount2(amount, state, you, res.source, res.x, &res.targets) as u16;
    if n == 0 {
        return None;
    }
    let options = colors_of(state, you, source);
    match options[..] {
        [] => return None,
        [only] => {
            add(state, res, only, n, restriction);
            return None;
        }
        _ => {}
    }
    // "In any combination of colors" is one pick per mana; anything else
    // picks one color for the whole amount.
    let (picks, per_pick) = if combination { (n, 1) } else { (1, n) };
    res.awaiting = Some(AwaitingOp::ManaChoice {
        colors: options.clone(),
        remaining: picks,
        per_pick,
        restriction,
    });
    Some(Pending::ChooseColor {
        player: you,
        options,
    })
}

/// Adds mana of one settled color, restricted or not, and journals it.
pub(super) fn add(
    state: &mut GameState,
    res: &Resolution,
    color: ManaColor,
    amount: u16,
    restriction: Option<ManaRestriction>,
) {
    let you = res.controller;
    if let Some(ManaRestriction { filter, rider }) = restriction {
        let id = state.next_restriction_id;
        state.next_restriction_id += 1;
        state
            .restriction_info
            .insert(id, (res.source, filter, rider));
        state.players[you.get() as usize].mana_pool.add_restricted(
            baylee_core::mana::RestrictedMana {
                color,
                amount,
                flags: baylee_core::mana::ManaFlags::default(),
                restriction: baylee_core::mana::RestrictionId(id),
            },
        );
    } else {
        state.players[you.get() as usize]
            .mana_pool
            .add(color, amount);
    }
    state.journal.record(GameEvent::ManaProduced {
        player: you,
        color,
        amount,
        source: Some(res.source),
    });
}

/// The colors this source can produce right now.
fn colors_of(state: &GameState, you: PlayerId, source: ManaSource) -> Vec<ManaColor> {
    match source {
        ManaSource::Fixed(color) => vec![color],
        ManaSource::Choice(colors) => colors.to_vec(),
        ManaSource::CommanderIdentity => {
            let mut colors = ColorSet::EMPTY;
            for id in state.zones.list(ZoneLocation::Command(you)) {
                if let Some(obj) = state.object(*id) {
                    colors = colors.union(obj.characteristics().color_identity);
                }
            }
            let options = colored(colors);
            if options.is_empty() {
                // No commander at all (a non-commander game): the ability
                // still resolves, and colorless is what is left.
                vec![ManaColor::Colorless]
            } else {
                options
            }
        }
        ManaSource::LandColor { mine } => {
            // Union of what the lands of the chosen side could produce.
            let mut colors = ColorSet::EMPTY;
            let mut colorless = false;
            for id in state.zones.list(ZoneLocation::Battlefield) {
                let Some(obj) = state.object(*id) else {
                    continue;
                };
                let c = obj.characteristics();
                if !c.types.contains(baylee_core::types::TypeSet::LAND) {
                    continue;
                }
                if (obj.controller == you) != mine {
                    continue;
                }
                colors = colors.union(c.produced_colors);
                colorless |= c.produced_colorless;
            }
            let mut options = colored(colors);
            if colorless {
                options.push(ManaColor::Colorless);
            }
            options
        }
    }
}

/// The five colors of a [`ColorSet`], as mana.
fn colored(colors: ColorSet) -> Vec<ManaColor> {
    [
        (ManaColor::White, baylee_core::color::Color::White),
        (ManaColor::Blue, baylee_core::color::Color::Blue),
        (ManaColor::Black, baylee_core::color::Color::Black),
        (ManaColor::Red, baylee_core::color::Color::Red),
        (ManaColor::Green, baylee_core::color::Color::Green),
    ]
    .into_iter()
    .filter(|&(_, c)| colors.contains(c))
    .map(|(m, _)| m)
    .collect()
}
