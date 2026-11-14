//! The player-facing loop setting, through real virtual input and rendered text.
use baylee_client_e2e::prelude::*;

fn picker(language: &str, title: &str, off: &str, draw: &str, interrupted: &str) {
    e2e::run_with(
        Launch::new()
            .lang(language)
            .setting("reduce_motion", true.into()),
        |client| {
            client.play_offline()?;
            client.click(press("Play(CreateTable)"))?;
            client.click(press("Play(Adjust)"))?;
            client.scroll(press("Play(Adjust)"), 12.0)?;
            client.settled()?.expect_shows(title)?;
            client.settled()?.expect_shows(off)?;
            client.click(press("Play(LoopPolicy(CompRulesDraw))"))?;
            client.wait_until("draw rule explanation", |screen| screen.shows(draw))?;
            client.click(press("Play(LoopPolicy(RunOnceThenBreak))"))?;
            client.wait_until("house rule explanation", |screen| screen.shows(interrupted))?;
            client.click(press("Play(LoopPolicy(Off))"))?;
            client.wait_until("detection disabled", |screen| screen.shows(off))?;
            client.across(
                &[Viewport::DESKTOP_FLOOR, Viewport::LAPTOP],
                |client, viewport| {
                    // A shorter viewport has more scrollable content than the
                    // original window. Bring the full explanation into view.
                    client.scroll(press("Play(Adjust)"), 2.0)?;
                    let screen = client.settled()?;
                    screen.expect_layout_ok()?;
                    for words in [title, off] {
                        let (index, node) = screen
                            .nodes()
                            .iter()
                            .enumerate()
                            .find(|(_, node)| node.text.as_deref() == Some(words))
                            .expect("the selected rule and its explanation are drawn");
                        assert!(!node.hidden && screen.clipped_by(index).is_none());
                    }
                    client.keep(&format!("{language}-{}", viewport.name), &screen);
                    client.screenshot(&format!("{language}-{}", viewport.name))?;
                    Ok(())
                },
            )
        },
    );
}

#[test]
#[ignore = "opens a client window: cargo dev e2e loop_rule_picker"]
fn english_loop_rule_picker_defaults_off_and_explains_every_option() {
    picker(
        "en",
        "Loop detection",
        "No automatic loop detection. You can still agree to a draw.",
        "A detected mandatory endless loop ends the game in a draw.",
        "House rule: a detected endless loop runs once, then is interrupted.",
    );
}

#[test]
#[ignore = "opens a client window: cargo dev e2e loop_rule_picker"]
fn german_loop_rule_picker_defaults_off_and_explains_every_option() {
    picker(
        "de",
        "Schleifenerkennung",
        "Keine automatische Schleifenerkennung. Ihr könnt weiterhin ein Unentschieden vereinbaren.",
        "Eine erkannte zwingende Endlosschleife beendet die Partie unentschieden.",
        "Hausregel: Eine erkannte Endlosschleife wird einmal ausgeführt und dann unterbrochen.",
    );
}
