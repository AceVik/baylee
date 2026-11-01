//! Count independent keyword grants using the same layer projection that
//! determines whether each grant applies. A bit records presence, not
//! multiplicity; flanking's instances trigger separately (CR 702.25b).

use super::{LayerPlan, projection::recompute_observing};
use crate::{object::GameObject, state::GameState};
use baylee_cards_dsl::{KeywordSet, Layer, Modifier};

/// Number of instances of one keyword, including separately applied grants.
pub(crate) fn keyword_instances(
    state: &GameState,
    object: &GameObject,
    keyword: KeywordSet,
) -> u32 {
    if !object.characteristics().keywords.contains(keyword) {
        return 0;
    }
    let mut count = u32::from(object.base.keywords.contains(keyword));
    let plan = LayerPlan::build(&state.effects);
    let (projected, _, _) = recompute_observing(state, object, &plan, |layer, effect, after| {
        if !after.keywords.contains(keyword) {
            count = 0;
            return;
        }
        match effect.modifier {
            Modifier::BecomeCopyOf(_) if layer == Layer::Copy => count = 1,
            Modifier::AddKeyword(bits) if bits.contains(keyword) => {
                count = count.saturating_add(1);
            }
            Modifier::AddKeywordIfCountersAtLeast {
                kind,
                at_least,
                keywords,
            } if keywords.contains(keyword) && object.counters.get(kind) >= u16::from(at_least) => {
                count = count.saturating_add(1);
            }
            _ => {}
        }
    });
    if projected.keywords.contains(keyword) {
        count.max(1)
    } else {
        0
    }
}
