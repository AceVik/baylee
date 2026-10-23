use crate::lines::{LineShape, ability_shape, content_fits, map};
use baylee_cards_dsl::{
    Cost, Duration, Modifier, activated, cost, effect::Effect, filter::Filter, spell,
};
use baylee_core::mana::{ManaColor, ManaCost};

const GRANT: Effect = Effect::continuous(
    &Filter::CREATURE,
    Modifier::GrantActivated {
        cost: Cost {
            mana: ManaCost::parse("{2}"),
            ..Cost::FREE
        },
        effects: &[Effect::DrawCards {
            amount: baylee_cards_dsl::effect::Amount::Fixed(1),
        }],
        mana_ability: false,
    },
    Duration::UntilEndOfTurn,
);
const TEXT: &str = "Until end of turn, creatures you control gain \"{2}: Draw a card.\"";

#[test]
fn spell_grants_map_to_the_quoted_cost_and_refuse_a_different_cost() {
    let ability = spell!(&[GRANT]);
    assert_eq!(ability_shape(&ability), LineShape::Grant);
    assert_eq!(map(&[ability], TEXT).lines, [Some(0)]);
    assert!(content_fits(&ability, TEXT));
    let wrong_cost = TEXT.replace("{2}", "{3}");
    assert!(!content_fits(&ability, &wrong_cost));
    assert_eq!(map(&[ability], &wrong_cost).lines, [None]);
}

#[test]
fn spell_grants_are_found_inside_nested_effect_branches() {
    const BODY: &[Effect] = &[
        Effect::mana(ManaColor::Blue, 1),
        Effect::MayDo { effects: &[GRANT] },
    ];
    const EFFECTS: &[Effect] = &[Effect::Sequence(BODY)];
    let ability = spell!(EFFECTS);
    assert_eq!(map(&[ability], TEXT).lines, [Some(0)]);
    assert_eq!(map(&[ability], &TEXT.replace("{2}", "{1}")).lines, [None]);
}

#[test]
fn an_activation_keeps_its_own_cost_when_its_effect_grants_another() {
    let ability = activated!(Cost::TAP, &[GRANT]);
    let text = format!("{{T}}: {TEXT}");
    assert_eq!(ability_shape(&ability), LineShape::Activated);
    assert!(content_fits(&ability, &text));
    assert_eq!(map(&[ability], &text).lines, [Some(0)]);
}

#[test]
fn spell_grants_match_a_nonmana_sacrifice_cost() {
    const MANA: &[Effect] = &[Effect::mana(ManaColor::Black, 1)];
    const EFFECTS: &[Effect] = &[Effect::continuous(
        &Filter::LAND,
        Modifier::GrantActivated {
            cost: cost!(SacrificeSelf),
            effects: MANA,
            mana_ability: true,
        },
        Duration::UntilEndOfTurn,
    )];
    let ability = spell!(EFFECTS);
    let text = "Until end of turn, lands you control gain \"Sacrifice this land: Add {B}.\"";
    assert_eq!(map(&[ability], text).lines, [Some(0)]);
    assert_eq!(
        map(&[ability], &text.replace("Sacrifice this land", "{T}")).lines,
        [None]
    );
}
