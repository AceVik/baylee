//! Exact mana-value targets are not monotone in X: more mana can remove
//! every legal target. Plan and announce the same visible payable value.
use super::{AbilityDef, CardIdentity, Filter, HeuristicAgent, ManaCost, ObjectId, PlayerView};
use baylee_cards_dsl::{TargetSpec, ZoneRef};
use baylee_core::ids::CardIndex;

fn exact(filter: &Filter) -> bool {
    match filter {
        Filter::CmcExactlyX => true,
        Filter::And(parts) => parts.iter().any(exact),
        _ => false,
    }
}

impl HeuristicAgent {
    pub(super) fn exact_x_values(
        &self,
        view: &PlayerView,
        source: ObjectId,
        card: CardIndex,
        face: u8,
    ) -> Option<Vec<u32>> {
        let def = baylee_cards::by_index(card)?;
        let filter = def
            .abilities_for_face(usize::from(face))
            .iter()
            .find_map(|ability| {
                let AbilityDef::Spell {
                    targets: Some(req), ..
                } = ability
                else {
                    return None;
                };
                let TargetSpec::Object(filter) = req.spec else {
                    return None;
                };
                (req.min == 1 && !req.count_is_x && !req.distinct_mana_values && exact(filter))
                    .then_some(filter)
            })?;
        let mut values = view
            .battlefield
            .iter()
            .filter(|object| {
                use baylee_cards_dsl::KeywordSet;
                !object
                    .status
                    .contains(baylee_view::ObjectStatus::PHASED_OUT)
                    && object.keywords & KeywordSet::SHROUD.bits() == 0
                    && !(self.hostile(object.controller, view.seat)
                        && object.keywords & KeywordSet::HEXPROOF.bits() != 0)
            })
            .filter(|object| self.exact_x_matches(filter, view, object, source))
            .map(|object| object.mana_value)
            .collect::<Vec<_>>();
        values.sort_unstable();
        values.dedup();
        Some(values)
    }

    fn exact_x_matches(
        &self,
        filter: &Filter,
        view: &PlayerView,
        object: &baylee_view::PublicObject,
        source: ObjectId,
    ) -> bool {
        match filter {
            Filter::CmcExactlyX => true, // X is this object's public mana value.
            Filter::And(parts) => parts
                .iter()
                .all(|part| self.exact_x_matches(part, view, object, source)),
            _ => {
                self.filter_matches(filter, view, object, ZoneRef::Battlefield, Some(source))
                    != Some(false)
            }
        }
    }

    pub(super) fn exact_x_cost(
        &self,
        view: &PlayerView,
        source: ObjectId,
        card: CardIdentity,
        printed: &ManaCost,
        planned: ManaCost,
    ) -> Option<ManaCost> {
        if !printed.has_variable() {
            return Some(planned);
        }
        let Some(values) = self.exact_x_values(view, source, card.index, card.face) else {
            return Some(planned);
        };
        let base = printed.with_x(0).cmc();
        let per_x = printed.with_x(1).cmc().saturating_sub(base);
        if per_x == 0 {
            return Some(planned);
        }
        let max_x = planned.cmc().saturating_sub(base) / per_x;
        let x = values.into_iter().filter(|&x| x <= max_x).max()?;
        Some(planned.with_less_generic((max_x - x) * per_x))
    }
}
