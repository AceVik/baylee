//! A duel against the house AI, offline: the opening hand, the keyboard,
//! the pointer, and the table at the desktop sizes.
use baylee_client_e2e::prelude::*;

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn the_house_game_opens_at_the_opening_hand() {
    e2e::run(|client| {
        let table = client.play_the_house()?;
        ensure!(table.cards_in("hand").count() == 7, "a hand of seven");
        table.expect_shows("Keep this hand?")?;
        table.locate(&press("Keep"))?;
        table.locate(&press("Mulligan"))?;
        table.expect_layout_ok()
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn the_key_on_the_keep_button_keeps_the_hand() {
    e2e::run(|client| {
        client.play_the_house()?;
        client.press("K")?;
        let kept = client.wait_until("the hand kept", |s| s.control("Keep").is_none())?;
        kept.expect_not_shown("Keep this hand?")?;
        ensure!(kept.cards_in("hand").count() == 7, "the hand stays seven");
        Ok(())
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn clicking_mulligan_deals_a_new_hand() {
    e2e::run(|client| {
        let first = client.play_the_house()?;
        let before: Vec<u64> = first.cards_in("hand").map(|c| c.object).collect();
        client.click(press("Mulligan"))?;
        let again = client.wait_until("a new hand", |s| {
            let now: Vec<u64> = s.cards_in("hand").map(|c| c.object).collect();
            now.len() == 7 && now != before
        })?;
        again.expect_shows("Keep this hand?")
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn the_card_cursor_walks_the_hand() {
    e2e::run(|client| {
        let table = client.play_the_house()?;
        let hand: Vec<u64> = table.cards_in("hand").map(|c| c.object).collect();
        ensure!(table.hovered().is_none(), "nothing under the cursor yet");
        client.press("D")?;
        let first = client.wait_until("a card under the cursor", |s| s.hovered().is_some())?;
        let first = first.hovered().unwrap_or_default();
        ensure!(hand.contains(&first), "the cursor starts in the hand");
        client.press("D")?;
        let next = client.wait_until("the cursor moved", |s| {
            s.hovered().is_some_and(|card| card != first)
        })?;
        ensure!(
            next.hovered().is_some_and(|card| hand.contains(&card)),
            "the cursor stays in the hand"
        );
        Ok(())
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn hovering_a_card_puts_it_under_the_cursor() {
    e2e::run(|client| {
        let table = client.play_the_house()?;
        let card = table.cards_in("hand").nth(2).cloned();
        let Some(card) = card else {
            bail!("no third card")
        };
        client.hover(in_hand(card.name.as_str()))?;
        client.wait_until("the hovered card", |s| s.hovered() == Some(card.object))?;
        Ok(())
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn the_table_fits_every_desktop_viewport() {
    e2e::run(|client| {
        client.play_the_house()?;
        client.across(Viewport::DESKTOP, |client, viewport| {
            let table = client.settled()?;
            table.expect_layout_ok()?;
            for card in table.cards_in("hand") {
                ensure!(
                    table.window().contains(&card.rect, 1.0),
                    "{} lies outside the window at {viewport}: {}",
                    card.name,
                    card.rect
                );
            }
            table.locate(&press("Keep")).map(drop)
        })
    });
}

/// Several of the player's own triggers at once ask for their order, in the
/// real client: the sheet stands, says what each does, and Enter sends it.
#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn the_order_of_triggers_is_asked_and_enter_sends_the_suggestion() {
    let launch = Launch::new().env(
        "BAYLEE_DEV_SEAT_BOARD",
        "Phyrexian Arena; Copper Tablet; Verdant Force; Thallid",
    );
    e2e::run_with(launch, |client| {
        client.play_offline()?;
        client.click(press("Play(PlayHouse)"))?;
        client.wait_for(
            "the opening hand",
            std::time::Duration::from_secs(60),
            |s| s.in_duel() && s.at_rest() && s.cards_in("hand").next().is_some(),
        )?;
        client.press("K")?;
        let sheet = client.wait_for(
            "the order of triggers",
            std::time::Duration::from_secs(30),
            |s| s.shows("Top resolves first"),
        )?;
        sheet.expect_shows("Copper Tablet")?;
        sheet.expect_shows("Verdant Force")?;
        client.press("Enter")?;
        client
            .wait_until("the sheet gone", |s| !s.shows("Top resolves first"))?
            .expect_not_shown("Top resolves first")
    });
}
