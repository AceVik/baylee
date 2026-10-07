//! `cards/enchantments/mv_2/raging_river.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raging River is `Coverage::Partial` with none of its text written: it is
/// cast and sits on the battlefield doing nothing. Lich and Island Sanctuary
/// are played in `enchantments::lich` and `enchantments::island_sanctuary`.
#[test]
fn partial_enchantments_with_no_text_written_sit_doing_nothing() {
    let raging_river = card_index("a2310312-6e1e-4e34-a351-9aef499a810f");
    still_partial(raging_river);
    assert_eq!(
        cast_saying_nothing(raging_river, mountain(), 2),
        Zone::Battlefield,
        "Raging River"
    );
}
