//! Public-offer fixtures shared by the model boundary tests.
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{
    DamageChoiceId, DamageEffectKind, DamageEffectOption, DamagePartView, Pending,
};
use baylee_engine::event::DamageTarget;

pub(crate) fn questions() -> [Pending; 9] {
    let player = PlayerId::new(0);
    let choice = DamageChoiceId { batch: 9, step: 4 };
    let damage = vec![DamagePartView {
        id: 77,
        source: ObjectId::new(8, 0),
        recipient: DamageTarget::Player(player),
        amount: u32::MAX,
        is_combat: true,
        preventable: true,
    }];
    let effect = DamageEffectOption {
        id: 91,
        source: None,
        ability: None,
        controller: player,
        kind: DamageEffectKind::PreventNext {
            remaining: u32::MAX,
        },
        parts: vec![77],
    };
    let redirected = Pending::AllocatePrevention {
        player,
        choice,
        damage: damage
            .iter()
            .cloned()
            .map(|mut part| {
                part.preventable = false;
                part
            })
            .collect(),
        effect: DamageEffectOption {
            kind: DamageEffectKind::RedirectNext {
                remaining: u32::MAX,
                to: DamageTarget::Player(PlayerId::new(1)),
            },
            ..effect.clone()
        },
        total: u32::MAX,
    };
    let [mask, command, text, mana] = resolving_questions(player);
    [
        mask,
        command,
        text,
        mana,
        Pending::ChooseDamageSource {
            player,
            choice: baylee_core::ids::SourceChoiceId::new(9),
            options: vec![baylee_core::ids::DamageSourceRef {
                object: ObjectId::new(8, 0),
                version: 2,
            }],
        },
        Pending::ChooseDamageEffect {
            player,
            choice,
            damage: damage.clone(),
            options: vec![effect.clone()],
        },
        Pending::AllocatePrevention {
            player,
            choice,
            damage,
            effect,
            total: u32::MAX,
        },
        redirected,
        Pending::Priority {
            player,
            legal: Box::new(baylee_engine::choice::LegalActions {
                can_pass: true,
                granted_actions: vec![baylee_engine::choice::GrantedActionOffer {
                    id: baylee_core::ids::GrantedActionId::new(99),
                    source: baylee_core::ids::DamageSourceRef {
                        object: ObjectId::new(8, 0),
                        version: 2,
                    },
                    ability: None,
                    timing: baylee_cards_dsl::SpecialActionTiming::ManaAbility,
                    cost: baylee_cards_dsl::SpecialActionCost::Life(1),
                    effect: baylee_engine::choice::GrantedActionKind::AddMana {
                        color: baylee_core::mana::ManaColor::Colorless,
                        amount: 1,
                    },
                }],
                ..baylee_engine::choice::LegalActions::default()
            }),
        },
    ]
}

fn resolving_questions(player: PlayerId) -> [Pending; 4] {
    [
        Pending::ChooseCards {
            player,
            options: vec![ObjectId::new(9, 0)],
            min: 0,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::CastFaceDown {
                x: 3,
                paid: [1, 0, 0, 2, 0, 0],
                fixed_cost: baylee_core::mana::ManaCost::default(),
            },
            total: None,
        },
        Pending::ChooseCards {
            player,
            options: vec![ObjectId::new(9, 0)],
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::CommandCard,
            total: None,
        },
        Pending::ChooseNumber {
            player,
            min: 0,
            max: 19,
            reason: baylee_engine::choice::NumberPrompt::TextReplacement {
                kind: baylee_cards_dsl::TextWordKind::Color,
                target: baylee_core::ids::DamageSourceRef {
                    object: ObjectId::new(9, 0),
                    version: 1,
                },
            },
        },
        Pending::ChooseManaAbility {
            player,
            choice: baylee_engine::choice::ManaChoiceId {
                source: baylee_core::ids::DamageSourceRef {
                    object: ObjectId::new(8, 0),
                    version: 2,
                },
                step: 3,
            },
            options: vec![baylee_engine::choice::ManaAbilityChoice {
                source: baylee_core::ids::DamageSourceRef {
                    object: ObjectId::new(9, 0),
                    version: 1,
                },
                ability_index: None,
            }],
        },
    ]
}
