//! Finite house ambitions for voluntary token growth. This never limits the game.
use crate::{AIProfile, HeuristicAgent, policy};
use baylee_cards_dsl::{Effect, PlayerRel};
use baylee_core::{mana::ManaCost, types::TypeSet};
use baylee_engine::engine::DecisionContext;
use baylee_view::PlayerView;

impl HeuristicAgent {
    pub(crate) fn pays_optional_price(
        &self,
        view: &PlayerView,
        price: ManaCost,
        context: &DecisionContext<'_>,
    ) -> bool {
        if self.profile.generation >= AIProfile::OPTIONAL_TOKEN_SATURATION
            && pure_token_reward(context.effects)
            && view
                .battlefield_of(baylee_client_core::decision::resource_player(view))
                .filter(|object| {
                    crate::filter::public_token_identity(object) == Some(true)
                        && object.types.contains(TypeSet::CREATURE)
                        && !object.status.is_phased_out()
                })
                .take(32)
                .count()
                == 32
        {
            return false;
        }
        policy::pays_tax(view, price, context)
    }
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
            !rewards.is_empty()
                && rewards.iter().all(|reward| match reward {
                    Effect::CreateToken { token, .. }
                    | Effect::CreateTokenN { token, .. }
                    | Effect::CreateTokensFor {
                        player: PlayerRel::You,
                        token,
                        ..
                    } => token.types.contains(TypeSet::CREATURE),
                    _ => false,
                })
        })
}
