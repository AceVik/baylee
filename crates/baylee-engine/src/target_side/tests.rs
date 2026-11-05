use super::*;
use baylee_cards_dsl::prelude::*;

const CREATURE: TargetSpec = TargetSpec::Object(&Filter::CREATURE);

const fn pump(power: i32, toughness: i32) -> Effect {
    Effect::PumpTarget {
        power: if power < 0 {
            Amount::NegXFixed(power.unsigned_abs())
        } else {
            Amount::Fixed(power.unsigned_abs())
        },
        toughness: if toughness < 0 {
            Amount::NegXFixed(toughness.unsigned_abs())
        } else {
            Amount::Fixed(toughness.unsigned_abs())
        },
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }
}

#[test]
fn removal_is_harmful_whatever_a_rider_gives_back() {
    let swords = [
        Effect::Exile { target: CREATURE },
        Effect::GainLifeFor {
            amount: Amount::Fixed(1),
            who: PlayerRel::Chosen,
        },
    ];
    assert_eq!(effects_side(&swords, 0), Some(TargetSide::Harmful));
}

#[test]
fn a_pump_reads_by_its_sign() {
    assert_eq!(effects_side(&[pump(3, 3)], 0), Some(TargetSide::Helpful));
    assert_eq!(effects_side(&[pump(-3, -2)], 0), Some(TargetSide::Harmful));
    let double_strike = Effect::PumpTarget {
        power: Amount::Fixed(0),
        toughness: Amount::Fixed(0),
        keywords: KeywordSet::DOUBLE_STRIKE,
        duration: Duration::UntilEndOfTurn,
    };
    assert_eq!(effects_side(&[double_strike], 0), Some(TargetSide::Helpful));
}

#[test]
fn damage_and_tapping_hurt_and_untapping_helps() {
    let bolt = Effect::DealDamage {
        amount: Amount::Fixed(3),
        target: CREATURE,
    };
    assert_eq!(effects_side(&[bolt], 0), Some(TargetSide::Harmful));
    assert_eq!(
        effects_side(&[Effect::TapTarget], 0),
        Some(TargetSide::Harmful)
    );
    assert_eq!(
        effects_side(&[Effect::UntapTarget], 0),
        Some(TargetSide::Helpful)
    );
}

#[test]
fn a_branch_keeps_its_word_behind_a_may() {
    const DESTROY: [Effect; 1] = [Effect::Exile { target: CREATURE }];
    let may = Effect::MayDo { effects: &DESTROY };
    assert_eq!(effects_side(&[may], 0), Some(TargetSide::Harmful));
}

#[test]
fn what_has_no_side_says_nothing() {
    let scry = Effect::Scry {
        amount: Amount::Fixed(1),
    };
    assert_eq!(effects_side(&[scry], 0), None);
    assert_eq!(effects_side(&[], 0), None);
    let fight = Effect::Fight {
        fighter: TargetSlot::First,
        foe: TargetSlot::Second,
    };
    assert_eq!(effects_side(&[pump(2, 2), fight], 0), None, "a fight");
    assert_eq!(
        effects_side(&[Effect::ChooseNewTargets], 0),
        None,
        "a redirect"
    );
}

#[test]
fn an_auras_side_is_what_it_does_to_its_creature() {
    let weight = [static_ability!(
        Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
        Modifier::ModifyPT(-3, -2)
    )];
    assert_eq!(aura_side(&weight), Some(TargetSide::Harmful));
    let rancor = [
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPT(2, 0)
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::AddKeyword(KeywordSet::TRAMPLE)
        ),
    ];
    assert_eq!(aura_side(&rancor), Some(TargetSide::Helpful));
    let pacifism = [static_ability!(
        Filter::AttachedToBySource,
        Modifier::CantAttack
    )];
    assert_eq!(aura_side(&pacifism), Some(TargetSide::Harmful));
}

#[test]
fn a_static_with_no_row_leaves_an_aura_unread() {
    let elsewhere = [static_ability!(Filter::CREATURE, Modifier::ModifyPT(1, 1))];
    assert_eq!(aura_side(&elsewhere), None, "not its creature");
}
