//! Finite house ambitions for voluntary token growth. This never limits the game.
use crate::{AIProfile, HeuristicAgent, policy};
use baylee_cards_dsl::{Effect, PlayerRel};
use baylee_core::{mana::ManaCost, types::TypeSet};
use baylee_engine::engine::DecisionContext;
use baylee_view::PlayerView;

const ENOUGH_TOKENS: usize = 32;

impl HeuristicAgent {
    pub(crate) fn pays_optional_price(
        &self,
        view: &PlayerView,
        price: ManaCost,
        context: &DecisionContext<'_>,
    ) -> bool {
        if !self.profile.plays(AIProfile::OPTIONAL_TOKEN_SATURATION)
            || !pure_token_reward(context.effects)
        {
            return policy::pays_tax_as(view, price, context, self.profile);
        }
        !enough_tokens(view)
            && policy::can_pay_token_price(
                view,
                &price,
                context,
                self.profile.plays(AIProfile::RUN_EQUIPMENT_MANA),
            )
    }
}

fn enough_tokens(view: &PlayerView) -> bool {
    view.battlefield_of(baylee_client_core::decision::resource_player(view))
        .filter(|object| {
            crate::filter::public_token_identity(object) == Some(true)
                && object.types.contains(TypeSet::CREATURE)
                && !object.status.is_phased_out()
        })
        .take(ENOUGH_TOKENS)
        .count()
        == ENOUGH_TOKENS
}

pub(crate) fn enough_for(view: &PlayerView, effects: &[Effect]) -> bool {
    token_reward(effects) && enough_tokens(view)
}

fn token_reward(effects: &[Effect]) -> bool {
    !effects.is_empty()
        && effects.iter().all(|effect| match effect {
            Effect::CreateToken { token, .. }
            | Effect::CreateTokenN { token, .. }
            | Effect::CreateTokensFor {
                player: PlayerRel::You,
                token,
                ..
            } => token.types.contains(TypeSet::CREATURE),
            _ => false,
        })
}

// Only a price whose sole reward is your creature tokens and whose refusal is
// empty. Never decline an unrelated draw, ward, upkeep debt or mixed reward
// merely because a board already contains tokens. No item identity is read.
fn pure_token_reward(effects: &[Effect]) -> bool {
    !effects.is_empty()
        && effects.iter().all(|effect| {
            let rewards = match effect {
                Effect::PlayerMayPayThen {
                    player: PlayerRel::You,
                    effects,
                    ..
                }
                | Effect::PlayerMayPayManaThen {
                    player: PlayerRel::You,
                    effects,
                    ..
                } => *effects,
                _ => return false,
            };
            token_reward(rewards)
        })
}
