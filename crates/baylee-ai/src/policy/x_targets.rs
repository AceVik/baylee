//! Reject a positive-X graveyard target whose public mana value cannot fit
//! the X this casting plan can pay. No hidden state or guessed card identity.
use super::{AbilityDef, CardIdentity, Filter, HeuristicAgent, ManaCost, ObjectId, PlayerView};
use baylee_cards_dsl::{TargetSpec, ZoneRef};
use baylee_view::PublicObject;

fn positive_x_bound(filter: &Filter) -> bool {
    match filter {
        Filter::CmcAtMostX => true,
        Filter::And(parts) => parts.iter().any(positive_x_bound),
        _ => false,
    }
}

impl HeuristicAgent {
    /// Unknown filter clauses remain possible. Only a proved lack of enough
    /// targets prevents announcing the cast; the engine still validates it.
    pub(super) fn unreachable_x_graveyard_target(
        &self,
        view: &PlayerView,
        source: ObjectId,
        card: CardIdentity,
        printed: &ManaCost,
        planned: &ManaCost,
    ) -> bool {
        if !printed.has_variable() {
            return false;
        }
        let base = printed.with_x(0).cmc();
        let per_x = printed.with_x(1).cmc().saturating_sub(base);
        if per_x == 0 {
            return false;
        }
        let x = planned.cmc().saturating_sub(base) / per_x;
        let Some(def) = baylee_cards::by_index(card.index) else {
            return false;
        };
        def.abilities_for_face(usize::from(card.face))
            .iter()
            .any(|ability| {
                let AbilityDef::Spell {
                    targets: Some(req), ..
                } = ability
                else {
                    return false;
                };
                if req.min == 0 || req.count_is_x || req.distinct_mana_values {
                    return false;
                }
                let TargetSpec::CardInGraveyard(filter, relation) = req.spec else {
                    return false;
                };
                if !positive_x_bound(filter) {
                    return false;
                }
                let Some(owners) = self.seats(relation, view) else {
                    return false;
                };
                view.graveyards
                    .iter()
                    .flatten()
                    .filter(|object| owners.contains(&object.owner))
                    .filter(|object| {
                        self.x_bound_matches(filter, view, object, source, x) != Some(false)
                    })
                    .take(usize::from(req.min))
                    .count()
                    < usize::from(req.min)
            })
    }

    fn x_bound_matches(
        &self,
        filter: &Filter,
        view: &PlayerView,
        object: &PublicObject,
        source: ObjectId,
        x: u32,
    ) -> Option<bool> {
        match filter {
            Filter::CmcAtMostX => Some(object.mana_value <= x),
            Filter::And(parts) => {
                let mut unknown = false;
                for part in *parts {
                    match self.x_bound_matches(part, view, object, source, x) {
                        Some(false) => return Some(false),
                        None => unknown = true,
                        Some(true) => {}
                    }
                }
                if unknown { None } else { Some(true) }
            }
            _ => self.filter_matches(filter, view, object, ZoneRef::Graveyard, Some(source)),
        }
    }
}
