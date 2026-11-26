# Games survive restarts

Design, 26 November 2026, for the owner's ask of 25.11.2026: one server, and
on it updates and crashes that no longer end a game. **Being built**; until a
section says otherwise, this describes what is to come, not what runs.

## The goal

A running game outlives a restart of anything under it: the gateway, the
agent, its engine process, PostgreSQL for a few seconds, the machine. Each
player sees the table reconnect for a few seconds and goes on with the same
question. An update installs a new release without waiting for the last game
to end, and without holding admission for hours.

Not a goal: open WebSockets surviving a restart (the client already redials
after 0.5 s, `crates/baylee-client-core/src/reconnect.rs`), a second machine,
or moving a game onto a newer engine build.

## What a restart ends today

- The gateway keeps every lobby game in memory (`lobby.rs`). Its link to an
  engine is the game's lifeline: a link that closes before `GameEnded` marks
  the game over (`engine/game.rs`).
- An agent that loses its control link stops and reaps its engines
  (`baylee-agent` `stop_all`), and the gateway ends the games they ran
  (`engine/lifecycle.rs`, `owner_lost`).
- An engine sends its record in 32 KiB pieces, or after 30 s
  (`RECORD_FLUSH_MS`); a crash loses up to that much. The gateway's writer
  stops for good at the first database error (`record.rs`).
- So a deploy waits for every game to end, holding admission meanwhile
  (`docs/deploy-hooks.md`), and a reboot waits for the night and an empty
  machine (`baylee-reboot`).

## The design: resume from the record, on the same build

The record already holds the preset, every input with who produced it, and
the engine's hash after each (`docs/protocol.md` §"The game record (#315)").
`Session::resume_recorded` rebuilds a session from it and holds it to every
hash (`crates/baylee-gamehost/src/session/resume.rs`); the client uses it to
continue a game against the house after restarting into an update. A hosted
game is resumed the same way:

1. **The record is kept up to the last answer.** The engine sends a piece
   within a short bound of every answer, not 30 s; the gateway's writer
   retries a failed write instead of giving up; a gateway that is stopped
   asks every engine for its record (`FlushRecord`, #323) and waits until it
   is stored.
2. **The gateway keeps its tables.** Running games and waiting rooms are
   written to PostgreSQL as they change: seats, accounts, seat token hashes,
   the session key a chair is bound to, the execution identity, the room's
   rules. The preset is read back from the record's header, not stored twice.
   At start the gateway loads them; a running game waits for an engine.
3. **The engine resumes.** The gateway orders an engine of the game's exact
   execution identity (`pick_exact`) and, on its hello, sends the setup and
   the stored record. The engine rebuilds the session, checks every hash,
   continues the record at the next piece, and waits for the seats. No
   curtain: the table was open. A record that does not resume ends the game
   as today, with the reason said.
4. **Old engine builds stay installed** until their last game ends; the agent
   offers them beside the current build (`BAYLEE_ENGINE_BINS`). New games
   start on the current build only.
5. **Protocol versions are checked per game.** A seat socket is held to the
   protocol of its game's engine, not the gateway's, and the engine link
   accepts the protocols of the builds it keeps. The engine link only changes
   additively from here on; a test holds a previous build's engine to the
   current gateway.
6. **A deploy no longer drains.** When every running game can be resumed,
   the deploy asks for the records, checks each resumes on its retained
   build, and restarts. A game that cannot be resumed is waited for, as
   today. A reboot does the same.
7. **Clients and seat bridges come back on their own.** Both already redial.
   A ticket refused because the restarted gateway forgot the session makes
   the client sign in again at once instead of counting a refusal; an update
   offer that changes the protocol waits until the game is over. The seat
   bridges (`baylee-seat`, `baylee-seathost`) get the same handling.
8. **Caddy holds requests while the gateway restarts** (`lb_try_duration`),
   so a redial during the restart waits instead of failing.

Ranked duels (replicated through Central), Tower runs (their own durable
recovery) and challenges (a fixed house policy) keep their own rules at
first: a deploy still waits for those games.

## What resuming must reproduce

A resumed table must show every seat exactly what it was shown: its view and
question, its whole log (numbered as before, since clients append by line
number), its roster and print table, and who the table waits for. Less would
show a different game after a restart; more would show what was hidden.
The record does not hold all of that yet:

- **Chair changes** (`Line::Chair`: stood in, handed back, taken over,
  released) are written but passed over by resume, so their log lines and the
  roster's `away` differ afterwards.
- **A lost connection and its return** write log lines
  (`LogEvent::ConnectionLost`, `LogEvent::Returned`) that the record does not
  hold at all.
- **Hand sharing** (`SeatSetting`: shown, shown once, asked, declined) is not
  in the record, so a teammate stops seeing a hand they were shown.

Records are kept forever and old lines never change: what was missing came
as new annotation lines that replay passes over and resume applies
(26.11.2026): `connection` and `setting` lines, and on every input, chair,
connection and setting line `sealed`, how many log lines had been sent when
it was written, because only unsent lines fold into a repeat. `sealed`
depends on which frames a table built, so a table whose frames nobody reads
writes other values there and only there
(`discarding_the_frames_changes_nothing_but_the_frames`); both engines of a
ranked duel build the same frames, so their records still agree. Printings a
seat had earned are earned again from its log and its table when a session
resumes (`Session::earn_what_was_told`); one it was shown only in a view
long gone comes again with the next view that shows it.

`every_seat_of_a_resumed_table_is_shown_what_it_was_shown` (`session/resume.rs`)
and `a_resumed_team_table_shows_the_hands_it_showed`
(`session/tests/shared_hand.rs`) are the tests. Replay projections (replays
in the lobby) still pass over chair and connection lines, as before.

## The order of the work

Each step lands on its own, through the gate:

1. Resume reproduces every seat's table (the record annotations above). Done.
2. The record is durable: short flush bound, retrying writer, flush on stop.
3. Running games and rooms in PostgreSQL; the gateway loads them and orders
   resuming engines; the engine's resume mode.
4. Protocol per game; retained engine builds; the engine-link compatibility
   test.
5. Deploy and reboot without the drain; Caddy's retry; the client's and seat
   bridges' sign-in after a gateway restart.
6. An end-to-end test with real processes: two seats mid-game, gateway and
   agent killed and restarted, both seats get the same question, the game
   finishes, and the whole record replays exactly. Resume time measured on
   the largest realistic record.

## Privacy and the backup

The new tables hold accounts, seat token hashes and session keys of running
games only, and lose a row when its game ends. `docs/privacy.md` names them;
the backup keeps them only in its short-lived set.
