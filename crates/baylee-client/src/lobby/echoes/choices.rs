//! Translate explicit presses into revision-bound authority commands.
use super::{Action, Command, Ui, Work, api::Phase};
pub(super) use baylee_client_core::echoes::draft::Draft;
use baylee_client_core::echoes::draft::allowed;

pub(super) fn act(ui: &mut Ui, action: Action) -> Option<Work> {
    let run = ui.model.run()?;
    if ui.model.uncertain()
        || !matches!(run.phase, Phase::Shop | Phase::Reward)
        || ui.confirmation.is_some()
    {
        return None;
    }
    ui.choices.bind(run);
    let command = selected(&mut ui.choices, run, ui.model.publication(), action)?;
    if (matches!(command, Command::Edit { .. } | Command::Refit { .. })
        || ui.choices.deck_unchanged(Some(run)))
        && ui
            .model
            .publication()
            .is_some_and(|cohort| allowed(run, cohort, &command))
    {
        return ui.command(command);
    }
    None
}

fn selected(
    draft: &mut Draft,
    run: &super::api::Run,
    cohort: Option<&super::api::Cohort>,
    action: Action,
) -> Option<Command> {
    if edit_deck(draft, run, cohort, action) {
        return None;
    }
    match action {
        Action::Purchase(offer) => Some(Command::Buy {
            offer,
            delivery: super::domain::v2::shop::Delivery::Inventory,
        }),
        Action::QuickEquip(id) => {
            baylee_client_core::echoes::workshop::quick_move(&run.economy.inventory, id)
        }
        Action::Transfer(item, slot) => Some(Command::Move { item, slot }),
        Action::Upgrade(item, potion) => {
            baylee_client_core::echoes::workshop::mythic(&run.economy.inventory, item, potion)
        }
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
        Action::SaveDeck => Some(draft.save_deck()),
        Action::ResetDeck => {
            draft.reset_deck(run);
            None
        }
        _ => None,
    }
}

fn edit_deck(
    draft: &mut Draft,
    run: &super::api::Run,
    cohort: Option<&super::api::Cohort>,
    action: Action,
) -> bool {
    let Some(cohort) = cohort else { return false };
    match action {
        Action::Card(card, add) => draft.card(run, cohort, card, add),
        Action::ZoneCard(card, zone, add) => draft.card_in(run, cohort, card, zone, add),
        Action::Commander(card) => draft.commander(run, cohort, card),
        _ => return false,
    }
    true
}
