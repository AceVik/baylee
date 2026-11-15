use super::{Action, Page, Read, Slot, Ui};
use crate::shellkit::focus::{Stop, TabOrder};

pub(crate) const ORDER: TabOrder = TabOrder {
    name: "echoes",
    stops: &[
        "back",
        "overview",
        "refresh",
        "retry",
        "play",
        "hosts",
        "tabs",
        "shop-return",
        "reroll",
        "offers",
        "pins",
        "slots",
        "ingredients",
        "materials",
        "buy",
        "tiers",
        "items",
        "move",
        "sell",
        "fuse",
        "card-preview",
        "remove",
        "add",
        "save",
        "reset",
        "mythic",
        "abandon",
        "runs",
        "run-page",
        "starters",
        "cohort-page",
        "withdraw",
    ],
    modal: false,
};

pub(crate) const CONFIRM: TabOrder = TabOrder {
    name: "echoes-confirm",
    stops: &["cancel", "confirm"],
    modal: true,
};

pub(super) fn stop(action: Action, ui: &Ui) -> Stop {
    let index = |id, offer| {
        ui.model
            .run()
            .and_then(|run| {
                if offer {
                    run.shop.offers.iter().position(|entry| entry.id == id)
                } else {
                    run.economy
                        .inventory
                        .relics
                        .iter()
                        .map(|item| item.id)
                        .chain(run.economy.inventory.materials.iter().map(|item| item.id))
                        .position(|item| item == id)
                }
            })
            .unwrap_or(0)
    };
    let (key, at) = match action {
        Action::Accept => return Stop::new(CONFIRM.name, "confirm"),
        Action::Cancel => return Stop::new(CONFIRM.name, "cancel"),
        Action::Open | Action::Back => ("back", 0),
        Action::Overview => ("overview", 0),
        Action::Refresh => ("refresh", 0),
        Action::Retry => ("retry", 0),
        Action::GoShop => ("shop-return", 0),
        Action::Withdraw => ("withdraw", 0),
        Action::Resume => ("play", 0),
        Action::Play(id) => (
            "play",
            ui.model
                .hosts()
                .iter()
                .position(|host| host.id == id)
                .unwrap_or(0),
        ),
        Action::Page(page) => ("tabs", page_index(page)),
        Action::Offer(id) => ("offers", index(id, true)),
        Action::Pin(id) => ("pins", index(id, true)),
        Action::Select(id) => ("items", index(id, false)),
        Action::Ingredient(id) => ("ingredients", index(id, false)),
        Action::Material(id) => ("materials", index(id, false)),
        Action::Place(slot) => (
            "slots",
            match slot {
                Slot::Active(i) => usize::from(i),
                Slot::Reserve(i) => usize::from(i) + 3,
            },
        ),
        Action::Card(card, _) | Action::InspectCard(card) => (
            match action {
                Action::Card(_, true) => "add",
                Action::Card(_, false) => "remove",
                _ => "card-preview",
            },
            ui.model
                .publication()
                .and_then(|c| c.catalog.cards.iter().position(|entry| entry.card == card))
                .unwrap_or(0),
        ),
        Action::Buy => ("buy", 0),
        Action::Tiers(_) => ("tiers", 0),
        Action::Move => ("move", 0),
        Action::Sell => ("sell", 0),
        Action::Fuse => ("fuse", 0),
        Action::Reroll => ("reroll", 0),
        Action::Abandon => ("abandon", 0),
        Action::Mythic(slot) => ("mythic", usize::from(slot)),
        Action::SaveDeck => ("save", 0),
        Action::ResetDeck => ("reset", 0),
        Action::Read(Read::Run(id)) => (
            "runs",
            ui.model
                .runs()
                .iter()
                .position(|run| run.id == id)
                .unwrap_or(0),
        ),
        Action::Read(Read::Runs(before)) => ("run-page", usize::from(before.is_some())),
        Action::Read(Read::Hosts(_)) => ("hosts", 0),
        Action::Read(_) => ("cohort-page", 0),
        Action::Starter(id, starter) => (
            "starters",
            ui.model
                .cohorts()
                .iter()
                .position(|cohort| cohort.id == id)
                .unwrap_or(0)
                * 3
                + usize::from(starter),
        ),
    };
    Stop::item(ORDER.name, key, u8::try_from(at).unwrap_or(u8::MAX))
}

const fn page_index(page: Page) -> usize {
    match page {
        Page::Overview => 3,
        Page::Shop => 0,
        Page::Equipment => 1,
        Page::Deck => 2,
    }
}
