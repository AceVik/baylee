//! Retained modal subsets keep their actual printed sentences.

use super::super::map_modes;
use baylee_cards_dsl::effect::{Amount, Effect};
use baylee_cards_dsl::{Filter, PlayerRel, SpellMode, TargetReq, TargetSpec, mode};

fn retained_modes() -> [SpellMode; 2] {
    [
        mode!(
            &[Effect::Mill {
                amount: Amount::Fixed(4),
                target: PlayerRel::Chosen
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
        ),
        mode!(
            &[Effect::PhaseOut {
                target: Some(TargetSpec::Object(&Filter::ARTIFACT))
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT))),
        ),
    ]
}

#[test]
fn retained_modes_skip_an_unsupported_middle_bullet() {
    let text = "Choose one —\n\
                • Target player mills four cards.\n\
                • Choose a land type and a basic land type. Each land of the first chosen type becomes the second chosen type until end of turn.\n\
                • Target artifact phases out.";
    assert_eq!(map_modes(&retained_modes(), text), vec![Some(1), Some(3)]);
    let [mill, phase] = retained_modes();
    assert_eq!(
        map_modes(&[phase, mill], text),
        vec![Some(3), Some(1)],
        "content, rather than retained mode position, owns the bullet"
    );
}

#[test]
fn an_ambiguous_effect_refuses_the_whole_subset() {
    let text = "Choose one —\n\
                • Target player mills four cards.\n\
                • Each opponent mills two cards.\n\
                • Target artifact phases out.";
    assert_eq!(
        map_modes(&retained_modes(), text),
        vec![None, None],
        "a unique phase-out line cannot make the ambiguous mill line known"
    );
}

#[test]
fn two_modes_cannot_claim_the_same_retained_bullet() {
    let text = "Choose one —\n\
                • Target player mills four cards.\n\
                • Target artifact phases out.\n\
                • Draw a card.";
    let [mill, _] = retained_modes();
    assert_eq!(map_modes(&[mill, mill], text), vec![None, None]);
}

#[test]
fn reminder_words_cannot_identify_a_retained_mode() {
    let text = "Choose one —\n\
                • Target player mills four cards.\n\
                • Draw a card. (An artifact phases out only when an effect says so.)\n\
                • Untap target creature.";
    assert_eq!(map_modes(&retained_modes(), text), vec![None, None]);
}

#[test]
fn an_unknown_effect_is_not_assigned_by_elimination() {
    let text = "Choose one —\n\
                • Draw a card.\n\
                • Target player mills four cards.\n\
                • Target artifact phases out.";
    let modes = [mode!(&[Effect::DrawCards {
        amount: Amount::Fixed(1)
    }])];
    assert_eq!(map_modes(&modes, text), vec![None]);
}
