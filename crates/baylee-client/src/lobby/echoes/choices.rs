//! Translate explicit presses into revision-bound confirmation candidates.
use super::{Action, Command, Ui, api::Phase};
pub(super) use baylee_client_core::echoes::draft::Draft;
use baylee_client_core::echoes::draft::allowed;

pub(super) fn act(ui: &mut Ui, action: Action) {
    let Some(run) = ui.model.run() else {
        return;
    };
    if ui.model.uncertain()
        || !matches!(run.phase, Phase::Shop | Phase::Reward)
        || ui.confirmation.is_some()
    {
        return;
    }
    ui.choices.bind(run);
    let draft = &mut ui.choices;
    let command = match action {
        Action::Reward(card) => {
            draft.reward(run, card);
            None
        }
        Action::ClaimReward => Some(Command::Claim {
            cards: draft.rewards.clone(),
        }),
        Action::Freeze => Some(Command::Freeze {
            frozen: !run.shop.frozen,
        }),
        Action::Offer(id) => {
            draft.offer = Some(id);
            draft.item = None;
            None
        }
        Action::Select(id) => {
            draft.item = Some(id);
            draft.offer = None;
            None
        }
        Action::Place(slot) => {
            draft.slot = slot;
            None
        }
        Action::Ingredient(id) => {
            draft.ingredient(run, id);
            None
        }
        Action::Material(id) => {
            if run
                .economy
                .inventory
                .materials
                .iter()
                .any(|material| material.id == id)
            {
                draft.material = (draft.material != Some(id)).then_some(id);
            }
            None
        }
        Action::Buy => draft.purchase(run),
        Action::Pin(id) => run
            .shop
            .offers
            .iter()
            .find(|offer| offer.id == id)
            .map(|offer| Command::Pin {
                offer: id,
                pinned: !offer.pinned,
            }),
        Action::Reroll => Some(Command::Refresh {}),
        Action::Move => draft.item.map(|item| Command::Move {
            item,
            slot: draft.slot,
        }),
        Action::Sell => draft.item.map(|item| Command::Sell { item }),
        Action::Fuse => Some(Command::Fuse {
            recipe: draft.recipe(),
        }),
        Action::Mythic(slot) => Some(Command::ChooseMythic {
            slot,
            replace_pinned: true,
        }),
        Action::Abandon => Some(Command::Abandon {}),
        Action::SaveDeck => Some(Command::Edit {
            deck: draft.deck.clone(),
        }),
        Action::ResetDeck => {
            draft.deck = run.cards.deck.clone();
            None
        }
        Action::Card(card, add) => {
            if let Some(cohort) = ui.model.publication() {
                draft.card(run, cohort, card, add);
            }
            None
        }
        _ => None,
    };
    if let Some(command) = command
        && (matches!(command, Command::Edit { .. }) || ui.choices.deck_unchanged(Some(run)))
        && ui
            .model
            .publication()
            .is_some_and(|cohort| allowed(run, cohort, &command))
    {
        ui.command(command);
    }
}
