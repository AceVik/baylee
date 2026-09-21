//! Public printing metadata for offline clients and gateways without a catalog.
use super::{LobbyEvent, Mailbox, Reply};
use baylee_client_core::deckbuilder::Printing;
use std::sync::Arc;

#[derive(serde::Deserialize)]
struct Page {
    data: Vec<Printing>,
    #[serde(default)]
    next_page: Option<String>,
}

pub(super) fn fetch(
    card: u32,
    oracle: &str,
    fallback: Vec<Printing>,
    epoch: u64,
    mailbox: &Mailbox,
) {
    let url = format!(
        "https://api.scryfall.com/cards/search?order=released&unique=prints&q=oracleid%3A{}",
        super::http::escape(oracle)
    );
    page(card, url, Vec::new(), fallback, epoch, mailbox.clone(), 0);
}

#[allow(clippy::too_many_arguments)] // serial pagination keeps one HTTP request in flight
fn page(
    card: u32,
    url: String,
    mut prints: Vec<Printing>,
    fallback: Vec<Printing>,
    epoch: u64,
    mailbox: Mailbox,
    pages: u8,
) {
    let mut request = ehttp::Request::get(url);
    request.headers.insert("Accept", "application/json");
    request
        .headers
        .insert("User-Agent", "baylee-deckbuilder/0.1");
    ehttp::fetch(request, move |response| {
        let parsed = response
            .ok()
            .filter(|r| r.ok)
            .and_then(|r| serde_json::from_slice::<Page>(&r.bytes).ok());
        let catalog = parsed.is_some();
        if let Some(next) = parsed {
            prints.extend(next.data);
            if let Some(url) = next
                .next_page
                .filter(|u| u.starts_with("https://api.scryfall.com/"))
                && pages < 31
            {
                page(card, url, prints, fallback, epoch, mailbox, pages + 1);
                return;
            }
        }
        // Always finish: an unavailable catalog still permits the known printing.
        let event = LobbyEvent::Printings {
            card,
            printings: if prints.is_empty() { fallback } else { prints },
            from_catalog: catalog,
        };
        let queue = Arc::clone(&mailbox.0);
        if let Ok(mut queue) = queue.lock() {
            queue.push(Reply::Remote(
                epoch,
                Box::new(Reply::PrintingCatalog(event)),
            ));
        }
    });
}
