//! Resolving a planned step against the current offer.

use super::super::Act;
use super::{
    Ask, CardIndex, Decision, Menu, ObjectId, Pending, PlayerAction, Request, Step, Stop, Table,
    asked, is_id, is_player, normal, object,
};
use baylee_engine::choice::YesNoPrompt;

impl Menu {
    /// The answer `step` gives this question, read against its offer as a
    /// model's own answer is: the kind of question first, then every id
    /// and name. `Err` is why the plan stops here.
    ///
    /// # Errors
    /// The step does not answer this question exactly.
    pub fn plan_decision(&self, step: &Step, request: &Request) -> Result<Decision, Stop> {
        let table = Table::new(&request.view, &request.context);
        let kind = || Stop::Kind {
            asked: asked(&request.pending),
        };
        // A queued yes/no describes an expected card choice. An opponent's
        // offer to end the game needs the model's own, current consent.
        if matches!(
            request.pending,
            Pending::YesNo {
                prompt: YesNoPrompt::DrawOffer { .. },
                ..
            }
        ) {
            return Err(kind());
        }
        let decision = |pick: Vec<String>| Decision {
            ask: Some(format!("q{}", self.question)),
            pick,
            ..Decision::default()
        };
        match (step, &request.pending) {
            (Step::Play(card), Pending::Priority { .. }) => {
                let id = self.option_for(card, |act| match act {
                    PlayerAction::PlayLand { card } => Some(*card),
                    _ => None,
                })?;
                Ok(decision(vec![id]))
            }
            (Step::Cast { card, targets }, Pending::Priority { .. }) => {
                let id = self.option_for(card, |act| match act {
                    PlayerAction::CastSpell { card } => Some(*card),
                    _ => None,
                })?;
                let mut decision = decision(vec![id]);
                decision.then_targets.clone_from(targets);
                Ok(decision)
            }
            (
                Step::Activate {
                    source,
                    ability,
                    targets,
                },
                Pending::Priority { .. },
            ) => {
                let id = self.ability_option(source, *ability)?;
                let mut decision = decision(vec![id]);
                decision.then_targets.clone_from(targets);
                Ok(decision)
            }
            (Step::Pass, Pending::Priority { .. }) => self
                .options
                .iter()
                .find(|c| c.alias.as_deref() == Some("pass"))
                .map(|c| decision(vec![c.id.clone()]))
                .ok_or_else(|| Stop::NotOffered("passing is not offered".into())),
            (Step::Yes, Pending::YesNo { .. }) => Ok(decision(vec!["yes".into()])),
            (Step::No, Pending::YesNo { .. }) => Ok(decision(vec!["no".into()])),
            (Step::Color(color), Pending::ChooseColor { .. })
            | (Step::Mode(color), Pending::ChooseCastMode { .. }) => {
                Ok(decision(vec![color.clone()]))
            }
            (Step::Number(n), Pending::ChooseNumber { .. }) => Ok(Decision {
                number: Some(n.to_string()),
                ..decision(Vec::new())
            }),
            (Step::Attack(pairs), Pending::ChooseAttackers { .. }) => Ok(Decision {
                attacks: pairs.clone(),
                ..decision(Vec::new())
            }),
            (
                Step::Choose(items),
                Pending::ChooseCards { .. }
                | Pending::ChooseTargets { .. }
                | Pending::LegendChoice { .. }
                | Pending::MulliganBottom { .. }
                | Pending::DiscardChoice { .. },
            ) => Ok(decision(self.chosen_by_name(items, &table)?)),
            _ => Err(kind()),
        }
    }

    /// The option whose action `of` reads as acting on the object `written`
    /// names.
    fn option_for(
        &self,
        written: &str,
        of: impl Fn(&PlayerAction) -> Option<ObjectId>,
    ) -> Result<String, Stop> {
        let offered: Vec<(ObjectId, &str)> = self
            .options
            .iter()
            .filter_map(|c| {
                let action = match &c.act {
                    Act::Now(action) | Act::Taps { then: action, .. } => action,
                };
                of(action).map(|id| (id, c.id.as_str()))
            })
            .collect();
        let ids: Vec<ObjectId> = offered.iter().map(|(id, _)| *id).collect();
        let id = object(written, &ids).map_err(Stop::NotOffered)?;
        Ok(offered
            .iter()
            .find(|(o, _)| *o == id)
            .map(|(_, option)| (*option).to_string())
            .expect("found among them"))
    }

    /// The option that activates `source`'s ability: its only one offered,
    /// or the one numbered `ability`.
    fn ability_option(&self, source: &str, ability: Option<u32>) -> Result<String, Stop> {
        let offered: Vec<(ObjectId, u32, &str)> = self
            .options
            .iter()
            .filter_map(|c| match &c.act {
                Act::Now(PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                }) => Some((*source, *ability_index, c.id.as_str())),
                _ => None,
            })
            .collect();
        let sources: Vec<ObjectId> = offered.iter().map(|(id, ..)| *id).collect();
        let id = object(source, &sources).map_err(Stop::NotOffered)?;
        let mine: Vec<&(ObjectId, u32, &str)> = offered
            .iter()
            .filter(|(o, index, _)| *o == id && ability.is_none_or(|n| index + 1 == n))
            .collect();
        match mine.as_slice() {
            [(.., option)] => Ok((*option).to_string()),
            [] => Err(Stop::NotOffered(format!(
                "{source} offers no ability numbered {}",
                ability.unwrap_or(0)
            ))),
            _ => Err(Stop::NotOffered(format!(
                "{source} offers more than one ability; name it as activate {source}:<n>"
            ))),
        }
    }

    /// Ids for `items`: an id among the offer as it is; a name for the
    /// options of that name, which must all be one card (two Swamps a
    /// library search shows are one card; Swamp and Snow-Covered Swamp are
    /// not), each name taking the next of them.
    fn chosen_by_name(&self, items: &[String], table: &Table<'_>) -> Result<Vec<String>, Stop> {
        let offered: &[ObjectId] = match &self.ask {
            Ask::Objects { from, .. } => from,
            Ask::Targets { objects, .. } => objects,
            _ => &[],
        };
        let mut chosen: Vec<ObjectId> = Vec::new();
        let mut out = Vec::new();
        for item in items {
            if is_id(item) || is_player(item) {
                out.push(item.clone());
                continue;
            }
            let wanted = normal(item);
            let named: Vec<ObjectId> = offered
                .iter()
                .copied()
                .filter(|&id| table.name(id).is_some_and(|name| normal(&name) == wanted))
                .collect();
            let mut cards: Vec<CardIndex> =
                named.iter().filter_map(|&id| card_of(table, id)).collect();
            cards.sort_unstable();
            cards.dedup();
            if cards.len() > 1 {
                return Err(Stop::NotOffered(format!(
                    "«{item}» names more than one card"
                )));
            }
            let next = named
                .iter()
                .find(|id| !chosen.contains(id))
                .copied()
                .ok_or_else(|| Stop::NotOffered(format!("no {item} is among the choices")))?;
            chosen.push(next);
            out.push(crate::narrator::tag(next));
        }
        Ok(out)
    }
}

/// The card behind an object the view shows.
fn card_of(table: &Table<'_>, id: ObjectId) -> Option<CardIndex> {
    let view = table.view;
    view.object(id)
        .and_then(|o| o.card)
        .or_else(|| view.hand.iter().find(|c| c.id == id).map(|c| c.card))
        .map(|c| c.index)
}
