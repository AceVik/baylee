use super::*;

/// The anti-loop rule, both ways round: a free cost with no parts is the
/// one shape that leaves the offer exactly as it found it.
#[test]
fn only_a_free_cost_can_repeat() {
    assert!(!consumes(&Cost::FREE), "a free ability was taken");
    assert!(
        !consumes(&Cost {
            mana: ManaCost::parse("{0}"),
            parts: &[],
        }),
        "a printed {{0}} is one generic symbol of nothing, and pays nothing"
    );
    assert!(consumes(&Cost::TAP), "a tap ability was refused");
    assert!(
        consumes(&Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::SacrificeSelf],
        }),
        "sacrificing the source was not counted as consuming it"
    );
    assert!(
        consumes(&Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::TapOther(&baylee_cards_dsl::Filter::YOUR_CREATURE)],
        }),
        "Earthcraft: the whole cost is somebody else's tap, and a creature \
         that paid cannot pay again"
    );
}

#[test]
fn exerting_alone_does_not_bound_repeated_activations() {
    assert!(!consumes(&Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::ExertSelf],
    }));
    assert!(consumes(&Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf, CostPart::ExertSelf],
    }));
}

/// X may be zero, so paying it is not a limit on anything.
#[test]
fn paying_x_life_is_not_a_limit() {
    assert!(!consumes(&Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::PayLifeX],
    }));
}
