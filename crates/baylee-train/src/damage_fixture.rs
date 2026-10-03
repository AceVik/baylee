//! Public-offer fixtures shared by the model boundary tests.
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{
    DamageChoiceId, DamageEffectKind, DamageEffectOption, DamagePartView, Pending,
};
use baylee_engine::event::DamageTarget;

pub(crate) fn questions() -> [Pending; 4] {
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
    [
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
    ]
}
