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
        "https://api.scryfall.com/cards/search?order=released&unique=prints&include_multilingual=true&include_variations=true&q=oracleid%3A{}",
        super::http::escape(oracle)
    );
    page(
        card,
        url,
        Vec::new(),
        fallback,
        epoch,
        mailbox.clone(),
        0,
        0,
    );
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
    retries: u8,
) {
    let mut request = ehttp::Request::get(url.clone());
    request.headers.insert("Accept", "application/json");
    #[cfg(not(target_arch = "wasm32"))]
    request.headers.insert("Cache-Control", "no-cache");
    request
        .headers
        .insert("User-Agent", "baylee-deckbuilder/0.1");
    ehttp::fetch(request, move |response| {
        if response
            .as_ref()
            .is_ok_and(|r| r.status == 429 || r.status >= 500)
            && retries < 2
        {
            // Scryfall's rate limit lasts longer than an ordinary retry delay.
            // Honor its Retry-After header instead of extending the ban.
            let delay = response.as_ref().ok().map_or(1000, |r| {
                r.headers
                    .get("retry-after")
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(if r.status == 429 {
                        60
                    } else {
                        u32::from(retries) + 1
                    })
                    .max(1)
                    .saturating_mul(1000)
            });
            later(delay, move || {
                page(
                    card,
                    url,
                    prints,
                    fallback,
                    epoch,
                    mailbox,
                    pages,
                    retries + 1,
                );
            });
            return;
        }
        let parsed = response
            .and_then(|r| {
                if r.ok {
                    serde_json::from_slice::<Page>(&r.bytes).map_err(|e| e.to_string())
                } else {
                    Err(format!("HTTP {}", r.status))
                }
            })
            .map_err(
                |error| bevy::log::warn!(card, pages, %error, "printing catalog request failed"),
            )
            .ok();
        if let Some(next) = parsed {
            prints.extend(next.data);
            if let Some(url) = next
                .next_page
                .filter(|u| u.starts_with("https://api.scryfall.com/"))
                // Basic lands have thousands of multilingual editions.
                && pages < 127
            {
                // Leave breathing room between pages; basic lands can take
                // dozens of pages and otherwise trigger HTTP 429 mid-catalog.
                later(150, move || {
                    page(card, url, prints, fallback, epoch, mailbox, pages + 1, 0);
                });
                return;
            }
        }
        // A later page failing must not discard the editions already received.
        // Only an entirely unavailable catalog falls back to the known printing.
        let catalog = !prints.is_empty();
        let event = LobbyEvent::Printings {
            card,
            printings: if catalog { prints } else { fallback },
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

/// Schedule on the network edge, never block the UI or browser event loop.
fn later(milliseconds: u32, task: impl FnOnce() + Send + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(u64::from(milliseconds)));
        task();
    });
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let callback = wasm_bindgen::closure::Closure::once_into_js(task);
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.unchecked_ref(),
                i32::try_from(milliseconds).unwrap_or(i32::MAX),
            );
        }
    }
}
