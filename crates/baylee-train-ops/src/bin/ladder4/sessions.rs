//! Ordinary, reusable accounts for an evaluation table on a private gateway.
use super::{Args, Game, Gateway, guest, main_rows, reap};
use anyhow::{Context as _, Result, ensure};
use baylee_train_ops::session_input;
use serde::Deserialize;
use serde_json::json;
use std::io::{BufRead, Write as _};
use std::path::Path;
use std::process::Child;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Sessions {
    net_session: String,
    llm_session: String,
}

impl Sessions {
    /// The pair as the coordinator's one line on stdin.
    pub(super) fn read(input: impl BufRead) -> Result<Self> {
        let pair: Self = session_input::read_line(input)?;
        session_input::validate_token(&pair.net_session)?;
        session_input::validate_token(&pair.llm_session)?;
        ensure!(
            pair.net_session != pair.llm_session,
            "each seat must own its own account"
        );
        Ok(pair)
    }

    /// Writes the net account's session to `seat4 --session-stdin`, whose
    /// stdin closes with it.
    pub(super) fn hand_net_session(&self, net: &mut Child) -> Result<()> {
        let mut input = net.stdin.take().context("seat4 input")?;
        serde_json::to_writer(&mut input, &self.net_session)?;
        input.write_all(b"\n")?;
        Ok(())
    }

    /// Sits the second account in chair 1 of `room` and hands that chair to
    /// a bridge. The account's name must say what plays it.
    pub(super) fn seat_model(
        &self,
        args: &Args,
        game: &Game,
        room: &str,
        work: &Path,
    ) -> Result<Child> {
        let gateway = Gateway::new(&args.gateway);
        let token = self.llm_session.as_str();
        let expected = if game.profile.starts_with("house:") {
            "HOUSE-"
        } else {
            "LLM-"
        };
        let profile = gateway.call("/me", Some(token), None)?;
        ensure!(
            profile["display_name"]
                .as_str()
                .is_some_and(|name| name.starts_with(expected)),
            "the second account must disclose its seat type"
        );
        let deck = std::fs::read_to_string(&game.llm_deck)?;
        let deck_id = gateway.upload(token, "llm-deck", &main_rows(&deck))?;
        let reply = gateway.call(
            &format!("/lobby/games/{room}/join"),
            Some(token),
            Some(&json!({"deck_id":deck_id, "seat":1, "password":null})),
        )?;
        ensure!(reply["seat"] == 1, "the gateway assigned the wrong chair");
        let seat_token = reply["seat_token"]
            .as_str()
            .context("missing occupied chair token")?;
        session_input::validate_token(seat_token)?;
        let handoff =
            json!({"game_id":room, "seat":1, "seat_token":seat_token, "session_token":token});
        let mut model = guest(args, game, room, Some(1), work)?;
        let result = (|| -> Result<()> {
            let input = model.stdin.as_mut().context("model seat input")?;
            serde_json::to_writer(&mut *input, &handoff)?;
            input.write_all(b"\n")?;
            Ok(())
        })();
        if let Err(error) = result {
            reap(&mut model);
            return Err(error);
        }
        Ok(model)
    }

    /// Gives both chairs of a room that never began back, best effort: the
    /// accounts are used again, and an account may host only a few waiting
    /// rooms.
    pub(super) fn leave(&self, gateway: &str, room: &str) {
        let gateway = Gateway::new(gateway);
        let path = format!("/lobby/games/{room}/leave");
        for token in [&self.llm_session, &self.net_session] {
            let _ = gateway.call(&path, Some(token), Some(&json!({})));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(line: &str) -> Result<Sessions> {
        Sessions::read(line.as_bytes())
    }

    #[test]
    fn the_two_seats_bring_distinct_accounts_on_one_whole_line() {
        let pair = read("{\"net_session\":\"one\",\"llm_session\":\"two\"}\n").unwrap();
        assert_eq!(
            (pair.net_session.as_str(), pair.llm_session.as_str()),
            ("one", "two")
        );
        for line in [
            // The same account twice, a missing one, one that is no bearer
            // credential, a field nobody reads, and a line cut short.
            "{\"net_session\":\"one\",\"llm_session\":\"one\"}\n",
            "{\"net_session\":\"one\"}\n",
            "{\"net_session\":\"one\",\"llm_session\":\"two\\r\\nX: y\"}\n",
            "{\"net_session\":\"one\",\"llm_session\":\"two\",\"more\":1}\n",
            "{\"net_session\":\"one\",\"llm_session\":\"two\"}",
        ] {
            let error = read(line).err().expect("refused").to_string();
            assert!(!error.contains("one") && !error.contains("two"), "{error}");
        }
    }
}
