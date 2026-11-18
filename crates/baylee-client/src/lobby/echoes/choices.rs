//! Translate explicit presses into revision-bound authority commands.
#[cfg(any(not(target_arch = "wasm32"), test))]
use super::{Action, Command, Ui, Work};
pub(super) use baylee_client_core::echoes::draft::Draft;
#[cfg(any(not(target_arch = "wasm32"), test))]
use baylee_client_core::echoes::draft::allowed;

#[cfg(any(not(target_arch = "wasm32"), test))]
pub(super) fn act(ui: &mut Ui, action: Action) -> Option<Work> {
    let run = ui.model.run()?;
    if ui.model.uncertain()
        || !super::phase_takes(run.phase, action == Action::Abandon)
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

#[cfg(any(not(target_arch = "wasm32"), test))]
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
        Action::PurchaseEquipped(offer) => purchase_equipped(run, offer),
        Action::QuickEquip(id) => {
            baylee_client_core::echoes::workshop::quick_move(&run.economy.inventory, id)
        }
        Action::Transfer(item, slot) => Some(Command::Move { item, slot }),
        Action::Upgrade(item, potion) => {
            baylee_client_core::echoes::workshop::mythic(&run.economy.inventory, item, potion)
        }
        Action::Reward(card) => {
            if let Some(reward) = &run.reward
                && reward.take == 1
                && draft.rewards.len() == 1
                && !draft.rewards.contains(&card)
                && reward.cards.contains(&card)
                && let Some(&previous) = draft.rewards.first()
            {
                draft.reward(run, previous);
            }
            draft.reward(run, card);
            None
        }
        Action::ClaimReward => Some(Command::Claim {
            cards: draft.rewards.clone(),
        }),
        Action::SellCard(card) => Some(Command::SellCard { card }),
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

#[cfg(any(not(target_arch = "wasm32"), test))]
fn purchase_equipped(run: &super::api::Run, offer: u32) -> Option<Command> {
    use super::domain::v2::{
        inventory::Slot,
        shop::{Delivery, Goods},
    };
    let goods = run
        .shop
        .offers
        .iter()
        .find(|entry| entry.id == offer && !entry.sold)?
        .goods;
    let delivery = if let Goods::Relic(definition) = goods {
        // Preview the shared pure transaction, including cascading fusion. A full
        // loadout can still accept an upgrade that reuses an equipped ingredient.
        let preview = run.economy.bought_automatic(definition).ok()?;
        let item = preview.inventory.relics.last()?;
        let slot = if let Slot::Active(slot) = item.slot {
            slot
        } else {
            (0..preview.inventory.layout.active_slots()).find(|slot| {
                !preview
                    .inventory
                    .relics
                    .iter()
                    .any(|item| item.slot == Slot::Active(*slot))
            })?
        };
        Delivery::Slot(Slot::Active(slot))
    } else {
        Delivery::Inventory
    };
    Some(Command::Buy { offer, delivery })
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn edit_deck(
    draft: &mut Draft,
    run: &super::api::Run,
    cohort: Option<&super::api::Cohort>,
    action: Action,
) -> bool {
    let Some(cohort) = cohort else { return false };
    match action {
        Action::Card(card, add) => draft.card(run, cohort, card, add),
        #[cfg(test)]
        Action::ZoneCard(card, zone, add) => draft.card_in(run, cohort, card, zone, add),
        _ => return false,
    }
    true
}
