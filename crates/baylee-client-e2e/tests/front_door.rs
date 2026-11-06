//! The front door: what a player sees before signing in or playing, at
//! every size and in both languages.
use baylee_client_e2e::prelude::*;

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn the_front_door_fits_every_viewport() {
    e2e::run(|client| {
        client.across(Viewport::ALL, |client, _| {
            let screen = client.settled()?;
            screen.expect_shows("Play offline")?;
            screen.expect_layout_ok()
        })
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn the_largest_text_step_still_fits_the_small_windows() {
    e2e::run_with(Launch::new().text_step(TextStep::Xl), |client| {
        client.across(Viewport::SMALL, |client, _| {
            client.settled()?.expect_layout_ok()
        })
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn a_german_client_opens_in_german() {
    e2e::run_with(Launch::new().lang("de"), |client| {
        let screen = client.settled()?;
        screen.expect_shows("Offline spielen")?;
        screen.expect_shows("Ohne Konto weiter")?;
        screen.expect_not_shown("Play offline")
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn picking_a_language_rewrites_the_interface_in_place() {
    e2e::run(|client| {
        client.click(press("Front(FrontMenu)"))?;
        client.click(press("Shared(PickLang(De))"))?;
        client.wait_until("German", |s| s.shows("Offline spielen"))?;
        client.click(press("Front(FrontMenu)"))?;
        client.click(press("Shared(PickLang(En))"))?;
        client
            .wait_until("English again", |s| s.shows("Play offline"))?
            .expect_not_shown("Offline spielen")
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn going_on_without_an_account_reaches_the_gateways_and_comes_back() {
    e2e::run(|client| {
        client.across(Viewport::ALL, |client, _| {
            client
                .settled()?
                .expect_shows("Continue without an account")?;
            client.click(press("Front(WithoutAccount)"))?;
            client.wait_until("the device's gateways", |s| {
                s.control("Front(BackToCentral)").is_some()
            })?;
            let list = client.settled()?;
            list.expect_shows("Choose your gateway")?;
            list.expect_shows("Sign in with an account")?;
            list.expect_layout_ok()?;
            client.click(press("Front(BackToCentral)"))?;
            client.wait_until("the sign-in", |s| {
                s.control("Front(WithoutAccount)").is_some()
            })?;
            client.settled()?.expect_layout_ok()
        })
    });
}

#[test]
#[ignore = "opens a client window: cargo dev e2e"]
fn playing_offline_offers_the_house_and_a_deck() {
    e2e::run(|client| {
        let lobby = client.play_offline()?;
        lobby.locate(&label("Play the house"))?;
        lobby.expect_shows("Offline · local decks")?;
        lobby.expect_layout_ok()
    });
}
