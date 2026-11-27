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
2. **The gateway keeps its tables.** Running games (later waiting rooms) are
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
   Done: 2 s bound, 15 minutes of retries, a stop waits up to 5 s and holds
   every game it asked (`FlushRecord.hold`), so the record it stored is the
   whole game. A finished record is compacted into one piece.
3. a. The engine's resume mode (`GameSetup.resume_record`): it rebuilds the
      session on its own build only, continues the record at the next piece
      and opens the table without a curtain. Done.
   b. Running games in PostgreSQL; the gateway loads them at start and orders
      resuming engines. Done for a restart of the gateway (`e2e_restart`: a
      game goes on across a stop and a kill, a reissued token outlives a
      kill). Waiting rooms come later: a room lost in a restart costs a
      re-invite, not a game. An engine whose link breaks while the gateway
      runs still ends its game; resuming that is a step of its own.
4. Protocol per game; retained engine builds; the engine-link compatibility
   test. Done in the gateway and the agent (§"Step 4"); the deploy that
   keeps the old builds installed is step 5.
5. Deploy and reboot without the drain; Caddy's retry; the client's and seat
   bridges' sign-in after a gateway restart. The deploy keeps each release's
   engines in a directory of their own while a game of theirs is kept, names
   them in `BAYLEE_ENGINE_BINS`, and removes them once no game needs them;
   the client does not take an update that changes the protocol while it is
   at a table.
6. An end-to-end test with real processes: two seats mid-game, gateway and
   agent killed and restarted, both seats get the same question, the game
   finishes, and the whole record replays exactly. Resume time measured on
   the largest realistic record.

## Step 3b: the gateway keeps its running games

Only for a restart of the gateway: the agent ends its engines when its link
to the gateway goes (`baylee-agent` `stop_all`), so after a restart every
record has one writer and there is no old engine to fence off. An engine
whose link breaks while the gateway runs still ends its game; resuming that
is a later step.

**What is kept.** One row per running game in `running_game`, keyed by the
game and removed with its record. It holds only what the record cannot give:
the room (host, name, format, list, spectators, password hash, when it was
opened and started) and each chair (who sits there, the seat token's hash,
the session key the chair is bound to, the seat bridge and the hosted order,
the deck, the team, whether its participation ended). The preset and the
engine's execution identity are read from the record's header, never stored
twice. The row is versioned and a golden test holds its shape.

- Ranked duels, Tower runs, challenges and replica jobs get no row.
- The row is written once the record's first piece is stored, since there
  is nothing to resume before it.
- A seat token reissued in a running game (`POST /lobby/games/{id}/seat`,
  the one way a running chair gets a new token; a chair is handed to a seat
  bridge only while the room waits) is written before the route answers,
  and the route answers `503` when the write fails. Restoring an older seat
  token hash would reopen a chair that was taken back. A write that failed
  is tried again at every sweep; until it goes through, a restart restores
  the token before the reissue, which the reissue's caller never received.
- Everything else (a participation that ended, an account deleted, a team
  or a bridge's readiness) is written at the next sweep, at most five
  seconds later. A deleted account is checked again at the restore, so its
  chair never comes back.
- One task writes the rows, the current state of a game each time, so a
  later state is never overwritten by an earlier one.
- The row goes when the game ends: in the same transaction that completes
  the record, and on every other way a game ends.
- Once the gateway is stopping, nothing is written: a link closing in the
  stop must not delete the game it is keeping.

**At start.** Before the gateway binds its port it reads every row. A row of
an unknown version, a record without a header, a complete record, an
execution identity this gateway does not know, or a chair whose account is
gone ends that one game with the reason logged, and the gateway starts
anyway. Only the header is read here; the whole record is read when the
engine attaches.

**The engine.** A restored game waits for an agent that offers its exact
execution identity (`pick_exact`), ordered when an agent says hello. No
agent yet, or no room, is waiting, not a failure. A game whose engine has
not attached `RESTORE_WAIT_SECS` after the start ends with the reason said.
On its hello the engine gets the setup with the stored record
(`GameSetup.resume_record`, `resume_pieces`) and the record goes on at the
next piece. A record with a gap is not sent.

**What a player sees.** The seats come back by the same tickets as after a
lost connection. A seat whose session the gateway no longer knows signs in
again (step 5); a local account's session is in the database and survives.
The engine starts with every seat away and the table open.

**Tests.** A real gateway, agent and engine in the end-to-end harness:

- A game a few answers in, the gateway stopped (`SIGTERM`) and started
  again on the same database: both seats get the question they had, the
  engine hash is the one before, the first new piece is the stored count,
  the old pieces are unchanged, the game finishes and the whole record
  replays. The same with `SIGKILL`.
- A seat reclaimed, the gateway killed and started: the old token is
  refused and the new one opens.
- The table and lobby views of the game are equal before and after.
- Ranked, Tower and challenge games leave no row; an ended game leaves none.
- Two restarts in a row.

## Step 4: a game goes on on the build that began it

So that an update can replace the gateway while a game an older engine plays
goes on (`docs/protocol.md` §"Which side checks the protocol"):

- **An agent keeps older builds.** It takes an engine of its own protocol or
  an older one into its inventory (`Identity::hostable`) and advertises it;
  a newer one stops its start. The gateway takes such an agent's whole list.
  A new game, a ranked duel, a challenge and a Tower run are put on this
  build only, as before; an older build is ordered only by `pick_exact`, for
  a game restored from a record that build wrote.
- **A table speaks its engine's protocol.** `LobbyGame::protocol` is the
  protocol of the game's execution identity. The engine's hello, a seat
  socket and a spectator are held to it, and the refusal names the table's
  number (`table_refusal`). A client that has not updated goes on with its
  game; one that has is told the table is older, with no
  `update_required`. The social socket stays with the gateway's protocol,
  and its refusal stops only the chat.
- **The engine is sent what it wrote.** A restored game keeps the preset as
  its record's header holds it (`LobbyGame::resume_preset`) and sends those
  bytes back, since this build's `GamePreset` may write fields an older one
  refuses.
- **The link says what it can do.** `EngineHello.link` is the engine link's
  revision (`ENGINE_LINK`, 1 from this step). A restored game is not given
  to an engine below `RESUMING_LINK`, which would start the game anew from
  its preset; the game ends with the reason said.
- **The link only gains.** `tests/engine_link.rs` holds every field, enum
  value and reservation of `transport.proto` to `engine_link.txt`; the
  agent's tests hold the arguments, variables and probe an engine is started
  with.

**Tests.** `e2e_restart` rewrites a kept game's record as if an older build
(protocol one lower, another build) had played it, replaces the gateway and
attaches an agent that keeps that build: the agent is ordered exactly it,
the engine is sent the record and the preset unchanged, a client of this
build's protocol is refused with the table's number, one of the table's
hears the engine's frames, and a new game goes to this build. An engine of
link 0 is refused the game, which ends. A real previous build's binary
against this gateway belongs to step 6.

**Not yet.** The deploy does not keep old builds installed (step 5). A seat
bridge or hosted model chair is replaced with the release, so at a table
older than itself it is refused like a client, and its chair goes as one
whose connection was lost (`docs/protocol.md` §"Leaving, and losing the
connection"); until seat bridges are kept too, a deploy waits for a game
with one. The client may still take an update that changes the
protocol while at a table, and a client that plays on at an older table
shows "update required" over it once it reads the gateway's newer
`/info` (`header.rs`, `update_required`), though nothing stops its game
(step 5: it compares against its table's protocol instead).

## Privacy and the backup

The new tables hold accounts, seat token hashes and session keys of running
games only, and lose a row when its game ends. `docs/privacy.md` names them;
the backup never saves them, as it never saves sessions: after a restore they
are there and empty.
