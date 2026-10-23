//! Possible production is a monotone six-bit graph, never recursive land lookup.

use super::{ManaSource, colors_of};
use crate::eval;
use crate::object::{GameObject, Rider};
use crate::state::GameState;
use crate::text_changes::RuleContext;
use crate::zone::Zone;
use baylee_cards_dsl::{AbilityDef, Amount, Effect, Filter};
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use baylee_core::types::TypeSet;
use std::collections::VecDeque;

const COLORS: u8 = 0b01_1111;
const TYPES: u8 = 0b11_1111;

#[derive(Clone, Copy)]
struct Dependency {
    filter: &'static Filter,
    mine: bool,
    mask: u8,
    context: RuleContext,
    tapped: bool,
}

#[derive(Default)]
struct Production {
    types: u8,
    dependencies: Vec<Dependency>,
}

pub(super) fn land_colors(
    state: &GameState,
    you: PlayerId,
    from: ObjectId,
    filter: &Filter,
    mine: bool,
    any_type: bool,
) -> Vec<ManaColor> {
    let lands: Vec<_> = state
        .battlefield_seen()
        .filter_map(|id| {
            state
                .object(id)
                .filter(|object| object.characteristics().types.contains(TypeSet::LAND))
        })
        .collect();
    let mut production: Vec<_> = lands.iter().map(|land| read(state, land)).collect();
    let mut dependants = vec![Vec::new(); lands.len()];
    for (recipient, rules) in production.iter().enumerate() {
        for dependency in &rules.dependencies {
            for (provider, land) in lands.iter().enumerate() {
                if (land.controller == lands[recipient].controller) == dependency.mine
                    && eval::matches_with_context(
                        dependency.filter,
                        state,
                        land,
                        lands[recipient].controller,
                        dependency.context,
                    )
                {
                    dependants[provider].push((recipient, dependency.mask, dependency.tapped));
                }
            }
        }
    }
    // Each of six bits can enter a vertex only once. A cycle with no
    // producing instruction has no seed and therefore invents no mana.
    let mut pending: VecDeque<_> = production
        .iter()
        .enumerate()
        .filter_map(|(index, item)| (item.types != 0).then_some(index))
        .collect();
    let mut queued: Vec<_> = production.iter().map(|item| item.types != 0).collect();
    while let Some(provider) = pending.pop_front() {
        queued[provider] = false;
        let available = production[provider].types;
        for &(recipient, mask, tapped) in &dependants[provider] {
            let output = if tapped {
                crate::mana_rules::replace_types(state, lands[recipient], available & mask)
            } else {
                available & mask
            };
            let extra = output & !production[recipient].types;
            if extra == 0 {
                continue;
            }
            production[recipient].types |= extra;
            if !queued[recipient] {
                queued[recipient] = true;
                pending.push_back(recipient);
            }
        }
    }
    let context = eval::live_context(state, from);
    let types = lands
        .iter()
        .zip(&production)
        .filter(|(land, _)| {
            (land.controller == you) == mine
                && eval::matches_with_context(filter, state, land, you, context)
        })
        .fold(0, |types, (_, produced)| types | produced.types);
    colors(types & if any_type { TYPES } else { COLORS })
}

fn read(state: &GameState, land: &GameObject) -> Production {
    let mut production = Production {
        types: crate::mana_rules::replace_types(
            state,
            land,
            mask(crate::casting::intrinsic_mana_colors(state, land.id)),
        ),
        ..Production::default()
    };
    let current = land.characteristics();
    if current.abilities_lost.is_none()
        && !current.rules_text_lost
        && let Some(list) = state.printed_ability_list(land.id)
    {
        for (index, ability) in list.abilities.iter().enumerate() {
            let context = RuleContext {
                source: land.id,
                text: state.ability_text(land.id, index as u32),
                resolved_name: None,
                reference: None,
            };
            read_ability(
                state,
                land,
                ability,
                context,
                land.rules_acquisition(),
                &mut production,
            );
        }
    }
    for grant in crate::effects::granted_activated(state, land.id) {
        let context = RuleContext {
            source: land.id,
            text: grant.text,
            resolved_name: None,
            reference: None,
        };
        read_ability(
            state,
            land,
            &grant.definition,
            context,
            grant
                .acquisition
                .unwrap_or_else(|| land.rules_acquisition()),
            &mut production,
        );
    }
    production
}

fn read_ability(
    state: &GameState,
    land: &GameObject,
    ability: &AbilityDef,
    context: RuleContext,
    acquisition: u64,
    production: &mut Production,
) {
    // Activation restrictions and costs do not prevent hypothetical resolution
    // (CR 106.7). Conditions inside the instructions still control production.
    match ability {
        AbilityDef::Activated { cost, effects, .. }
        | AbilityDef::ActivatedConditional { cost, effects, .. } => {
            read_effects(
                state,
                land,
                effects,
                ProductionContext {
                    rules: context,
                    acquisition,
                    tapped: ability.is_mana_ability()
                        && cost.parts.contains(&baylee_cards_dsl::CostPart::TapSelf),
                },
                production,
            );
        }
        AbilityDef::ModalActivated { cost, modes, .. } => {
            for mode in *modes {
                read_effects(
                    state,
                    land,
                    mode.effects,
                    ProductionContext {
                        rules: context,
                        acquisition,
                        tapped: ability.is_mana_ability()
                            && cost.parts.contains(&baylee_cards_dsl::CostPart::TapSelf),
                    },
                    production,
                );
            }
        }
        AbilityDef::Triggered { effects, .. } => {
            read_effects(
                state,
                land,
                effects,
                ProductionContext {
                    rules: context,
                    acquisition,
                    tapped: false,
                },
                production,
            );
        }
        _ => {}
    }
}

#[derive(Clone, Copy)]
struct ProductionContext {
    rules: RuleContext,
    acquisition: u64,
    tapped: bool,
}

fn read_effects(
    state: &GameState,
    land: &GameObject,
    effects: &[Effect],
    context: ProductionContext,
    production: &mut Production,
) {
    for effect in effects {
        match effect {
            Effect::AddManaUnlessAnyPlayerPays { source, amount, .. } => {
                // Every player can decline; these colors are possible even
                // though another player may prevent actual production.
                read_effects(
                    state,
                    land,
                    &[Effect::AddMana {
                        source: *source,
                        amount: Amount::Fixed(*amount),
                        distribution: baylee_cards_dsl::effect::ManaDistribution::OneColor,
                        restriction: None,
                    }],
                    context,
                    production,
                );
            }
            Effect::AddMana { source, amount, .. } => {
                read_mana(state, land, source, amount, context, production);
            }
            Effect::ManaForColorsAmong(filter) => {
                let available = mask(super::colored(eval::colors_among(
                    state,
                    land.controller,
                    filter,
                    context.rules,
                )));
                production.types |= if context.tapped {
                    crate::mana_rules::replace_types(state, land, available)
                } else {
                    available
                };
            }
            Effect::IfCondition {
                condition,
                then,
                otherwise,
            } => {
                let effects = if eval::condition_holds_with_context(
                    state,
                    land.controller,
                    context.rules,
                    *condition,
                ) {
                    then
                } else {
                    otherwise
                };
                read_effects(state, land, effects, context, production);
            }
            Effect::Sequence(effects) | Effect::MayDo { effects } => {
                read_effects(state, land, effects, context, production);
            }
            _ => {}
        }
    }
}

fn read_mana(
    state: &GameState,
    land: &GameObject,
    source: &ManaSource,
    amount: &Amount,
    context: ProductionContext,
    production: &mut Production,
) {
    if *amount != Amount::DevotionToManaColor
        && eval::amount_with_context(
            amount,
            state,
            land.controller,
            context.rules,
            Some(u32::MAX),
        ) == 0
    {
        return;
    }
    if let ManaSource::LandColor {
        filter,
        mine,
        any_type,
    } = source
    {
        production.dependencies.push(Dependency {
            filter,
            mine: *mine,
            mask: if *any_type { TYPES } else { COLORS },
            context: context.rules,
            tapped: context.tapped,
        });
    } else {
        let options = if *source == ManaSource::ExiledWithSource {
            exiled_colors_for(
                state,
                DamageSourceRef {
                    object: land.id,
                    version: land.version,
                },
                context.acquisition,
            )
        } else {
            colors_of(state, land.controller, *source, land.id)
        };
        let available = if *amount == Amount::DevotionToManaColor {
            let devotion = super::devotion(state, land.controller).unwrap_or([0; 6]);
            mask(
                options
                    .into_iter()
                    .filter(|color| devotion[color.index()] != 0),
            )
        } else {
            mask(options)
        };
        production.types |= if context.tapped {
            crate::mana_rules::replace_types(state, land, available)
        } else {
            available
        };
    }
}

pub(super) fn exiled_colors(state: &GameState, source: ObjectId) -> Vec<ManaColor> {
    let Some(host) = state.object(source) else {
        return Vec::new();
    };
    let identity = DamageSourceRef {
        object: source,
        version: host.version,
    };
    exiled_colors_for(state, identity, host.rules_acquisition())
}

pub(super) fn exiled_colors_for(
    state: &GameState,
    identity: DamageSourceRef,
    acquisition: u64,
) -> Vec<ManaColor> {
    let mut types = 0;
    for player in &state.players {
        for &id in state
            .zones
            .list(crate::zone::ZoneLocation::Exile(player.id))
        {
            if let Some(card) = state.object(id)
                && card.zone == Zone::Exile
                && card.riders.iter().any(|rider| matches!(rider, Rider::LinkedExile { source, acquisition: linked } if *source == identity && *linked == acquisition))
            {
                types |= mask(super::colored(card.characteristics().colors));
            }
        }
    }
    colors(types)
}

fn mask(colors: impl IntoIterator<Item = ManaColor>) -> u8 {
    colors
        .into_iter()
        .fold(0, |types, color| types | (1 << color.index()))
}

fn colors(types: u8) -> Vec<ManaColor> {
    ManaColor::ALL
        .into_iter()
        .filter(|color| types & (1 << color.index()) != 0)
        .collect()
}
