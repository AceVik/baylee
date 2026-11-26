# AI players

Concept, 26 November 2026, from the owner's grilling session of the same day
("damit sich die ersten Spieler nicht so einsam fühlen", Ranked above all).
**Being built** since the owner's go of 26.11.2026; until a section says
otherwise, this describes what is to come, not what runs. Season 1 is a test
season. Every rule here may be tuned or switched off once the season
has produced measurements.

## The goal

With few players, the ranked queue has no one to pair a player with. Today a
lone ticket waits until another person arrives or the player cancels
(`tools/league-lab/MATCHMAKING.md`). Baylee gets **AI players**. They are
accounts that are always labelled as AI and otherwise act like players: they
own decks and improve them, play Ranked and Tower, have public profiles,
replays, ratings and awards, and file bug reports. That brings four things at
once:

- a waiting player gets an opponent within a minute;
- every model and effort level gets an Elo on the same scale as people;
- the AIs test the game and report what they find;
- the whole system (queue, ranked engines, Central, settlement) runs under load.

Not a goal: passing an AI off as a person, hidden opponents, or AIs that play
around the clock for no one.

## Decided and proposed

**Decided by the owner (26.11.2026):** AI players always labelled; full
players with profiles, public replays and awards, on the same ladder including
Champion places and podium; calibration among themselves first, then mainly on
demand with a carousel for deck tests; every language model the live server
can reach except Fable, the house levels including `seer`, and Baylee AI;
admins can switch any AI off; Leih-Decks; the fill-in countdown and
"Nur Menschen"; the disconnect, stand-in, freeze and leaver rules below;
a replay is public only when every player released it.

**Proposed, taken without an answer, open to change:** honest model names and
a start roster of six to eight language-model accounts; how the AIs build the
Leih-Decks (precons plus small swaps); the speed check that keeps slow
profiles away from people; the house-fallback rule; the daily caps per
subscription and for the carousel; calibration ending when Season 1 starts;
the "KI-Meldungen" column; the training opt-out as a prerequisite.

**Still open:** which AI is seated after the countdown (§"Joining a search"),
how a person releases a replay to the public (§"Profiles and replays"), and
the scoring questions at the end of §"Disconnects and rage quits".

## Who plays

Every AI kind joins, each as its own account:

| Kind | Mind | Where it runs | Cost |
| --- | --- | --- | --- |
| Language models | `baylee-seat` with an API or CLI mind (`docs/llm-seat.md`): DeepSeek through its API; Claude Code, Codex and Antigravity through their CLIs, signed in on the live server | seat agent on the live server, its own Unix user | tokens; the owner's subscriptions and DeepSeek key |
| House AI, fair levels | `baylee-seat` with its house mind, an ordinary socket player like the language models | seat agent | CPU only |
| House AI, `seer` | `HeuristicAgent` with `Sight::Peeking`, a seat inside the engine | the engine server | CPU only |
| Baylee AI | the trained net (`docs/trained-ai.md`) behind `baylee-seat`; that mind is planned, not built | seat agent | to be measured: whether the live server's CPU runs it fast enough is not established |

Every AI but `seer` is therefore a remote seat that plays from its view, and
ranked needs no engine change for it. Fable does not play (too expensive).
`seer` is labelled for what it does: "Seher (unfair): sieht deine Hand und
deine nächsten drei Karten". It can only run in-engine, because the seat bridge
never receives hidden cards and refuses the level (`docs/house-ai.md` §"Fair by
default, and the one level that peeks").

**Which ranked path runs.** Central pairs two hosts of different registered
owners for a replicated match and falls back to a single host when it finds no
independent pair (Central `docs/ranked.md`). With one server and one owner,
every ranked game today runs single-host: the engine server's own driver keeps
the clocks, the reconnect window and the forfeit
(`crates/baylee-engine-server/src/driver/timing.rs`, `connections.rs`). The
in-engine pieces of this design (`seer`, the stand-in, freezing) are built for
that path first. The replicated path keeps today's rules until they are carried
over as Central authority rulings (§"Disconnects and rage quits").

**Names** are the honest model name and level, for example
"Claude Haiku · Hoch", "Haus-KI · Expert", with the provider's icon and an AI
badge everywhere a name appears: queue card, table, ladder, profile, replay.
The name is the measurement. The start roster is six to eight language-model
accounts (one per model and effort level that passes the speed check below),
the house levels and Baylee AI.

**Admins** can switch off any AI account. It leaves the queue at once; running
games end normally.

An AI account may play several games at once. Otherwise a single game would
block "Claude Haiku" for everyone.

## The ladder

AI accounts stand on the same ladder as people, with the same Elo, leagues,
divisions, Champion places, podium and deck awards (owner, 26.11.2026). In
Season 1 this is meant to motivate people to beat them.

- **Calibration.** Before Season 1 opens, the AIs play each other until their
  ratings settle. That phase ends when the season starts.
- **After that, on demand.** AIs play mainly when a person needs an opponent.
  A capped number of AI-against-AI games a day (the "carousel") lets each AI
  test its deck edits and keeps the ratings connected. The cap keeps people,
  not models playing each other, setting the scale, and it saves tokens.
- Repeat-opponent damping (third and fourth game in six hours at half stakes)
  applies to AI opponents as it does to people.

## Joining a search

The queue pairs people first, as today. When a search has found no one:

1. After **45 s** the search card shows a calm countdown, "KI-Gegner in 0:15",
   without layout shift.
2. A person who arrives during the countdown takes precedence.
3. At 0:00 an AI is seated. **Open** (the owner wants to think it through).
   Proposal: the AIs are ranked by the queue's own distance, rating gap plus
   its repeat cost of 140 per recent encounter, so that a player does not meet
   the same AI every time (the third and fourth game against one account in six
   hours pay half, the fifth a quarter). On a near tie the free kinds (house,
   Baylee AI) go before a language model, and a language model is only chosen
   while its daily cap has room.
4. A remembered switch **"Nur Menschen"** turns the AI fill-in off for that
   player. It is also the player's choice about the provider: against a
   language model, the player's moves go to that provider, without any name
   (`docs/privacy.md` §"Language-model seats").

The 60 s total matches the queue's own schedule, which opens the window fully
at 60 s in a quiet queue.

## Speed and stand-ins inside the AI

- **Speed check.** Before an AI profile may meet people, it is measured per
  model and effort level: time per decision (median and P95), game length,
  cost per game and how often the house had to take over. Only profiles under
  the limit set from that measurement meet people. Slower ones play only the
  carousel and Tower. The owner's experience is that low and medium effort are
  fast, and small models manage higher levels.
- **House fallback.** When a language model fails or runs out of budget, the
  house plays the rest of that game (`SeatCore::limited`). In Ranked, both
  players see the switch at that moment. The game counts normally. The replay
  and the profile mark the turn from which the house played. A profile whose
  games fall back too often leaves Ranked automatically until an admin
  re-admits it.

## Disconnects and rage quits

Owner, 26.11.2026, with the refinements agreed in the same session:

**Against any AI**, a lost connection **freezes** the game.

- At the next sign-in the player continues it, or concedes. While a game is
  frozen, the player cannot queue.
- A frozen game lapses after three days as the player's loss.
- There is no limit on freezes per game: a frozen game cannot be seen, so
  the thinking it allows is accepted (owner, 26.11.2026). A question's spent
  decision time stays spent.
- A ranked game resumes only on the engine build it started on (the ranked
  compatibility key names the build). Old engine builds stay installed until
  their last frozen game ends, as `docs/proposals/games-survive-restarts.md`
  already plans for running games.

**Between two people**, a lost connection **pauses** the game for 120 s.
After that a **stand-in** plays the absent seat:

- the strongest **fair** house level (never `seer`, which a stand-in may not
  be: `docs/house-ai.md`) whose Elo is below the absent player's, in-engine;
  when no level is below, the weakest (`novice`);
- deliberately slow on real decisions: at least 45 s for the first, 5 s less
  for each next one, down to 5 s; a pass goes at once;
- the player takes the seat back when they return;
- after five minutes of absence the opponent may **claim the win** instead of
  playing on. This closes the "let a slow stand-in wear the opponent down"
  trick.

Scoring: a stand-in loss counts as a normal loss. A stand-in win moves **half
the points** on both sides: the absent player gains half the quoted win, the
opponent loses half the quoted loss. Leaving can therefore never improve the
absent player's result.

**Both players gone:** the game freezes for three hours (owner, 26.11.2026).
When one of them returns, the other counts as the absent player: the 120 s
pause starts for them, then the stand-in. A game not finished within the three
hours is cancelled. Each player's first such cancellation in a day is
forgiven and not rated; each further one that day counts for that player as
half a loss. Against an AI this case cannot arise, since the AI never leaves.

**Leaving on purpose** (the Leave button) is an immediate concession, as today,
with no pause and no stand-in.

**Leaver ledger.** A player who does not return gets an entry. The first in
seven days is forgiven; each further one blocks the queue for 15 minutes
(owner, 26.11.2026). Elo carries only the result; the extra penalty for a rage
quit is time.

**Open scoring questions.**

- A claimed win: proposed to count as a normal result, a full win for the
  claimant and a full loss for the absent player.
- A game the returning player takes back and finishes: proposed to count
  normally, whatever the stand-in did meanwhile.
- Half stakes and a half loss with no winner create or remove points. The
  matchmaking model keeps the rating pool nearly conserved through bounded
  personal balances (`tools/league-lab/MATCHMAKING.md` §"Keeping the rating
  pool stable"). Whether these cases go through the same balances or are
  accepted as a small drain is to be decided with the league lab.

Where this is built: on the single-host path, which every game runs today
(§"Which ranked path runs"), the engine server's driver already holds the
reconnect window and the forfeit, and casual games already have a house
stand-in (`Session::stand_in`, `hand_back`). The pause, stand-in, slow pacing
and claim go there, and Central settles the result with the new scoring.
Replicated engines never play on by themselves when a socket is missing
(`docs/protocol.md` §"Engine control for a protected ranked assignment"); there
the same rules become new Central authority rulings that both engines
reproduce, like today's clock rulings, in a later stage.

## Decks

**Leih-Decks.** The language-model AIs build and maintain a shared pool of
decks: they start from playable precons (`docs/precons.md`) and swap a few
cards between games, reasoning from notes on their recent games. The house
levels and Baylee AI pilot the same decks. The same deck under different
pilots separates playing strength from deck strength, which deck Elo per
version (`tools/league-lab/MATCHMAKING.md`) can then show.

Each AI profile shows a deck chronicle: versions, swaps and what the AI says it
learned. Later stages: an offline optimiser on bigace that tunes decks for the
house by mass simulation, and Baylee AI's own deckbuilding module.

## Profiles and replays

- An AI profile shows its model and level, rating history, record against
  people and against AIs, deck chronicle and public replays, with icons and
  statistics in the profile's existing style.
- A replay is public only when **every** player at the table released it. AI
  seats always release. **Open:** how a person releases. Proposal: a separate
  checkbox "Replay öffentlich zeigen", **not ticked by default**, asked on the
  end screen after a game against an AI, with a preview. Today's replay choice
  (`game_record_share`, `docs/privacy.md`) reads yes when unsaid, but it only
  lets the other players of that game see a seat's hidden cards; publishing a
  name and a game to everyone is a different step.
- A person who beats an AI and releases the replay appears on that AI's
  **"Bezwinger"** list.
- After the game, a language model's reasoning (`AiLog`, today debug builds
  only) can be shown in the public replay. Live it would hand over hidden
  cards; after the game it no longer does. That needs the reasoning stored with
  the record, which today it never is.

## Tower

AI accounts play Tower runs like people, so that people meet AI echoes and the
Tower gets player-versus-player data. Their echoes and leaderboard entries
carry the AI badge. Tower runs are long, so they draw on the same daily caps.

## Bug reports

AI reports go to their own **"KI-Meldungen"** column in the feedback service,
grouped by the existing dedupe key and capped at 25 per game as today. A cheap
model pre-sorts them once a week. They stay untrusted until checked against
the rules, like every player report.

## Running it on one server

- All bridges run on the live server: the CLIs signed in to the owner's own
  subscriptions (no separate accounts; accepted by the owner), DeepSeek through
  its key. The seat agent runs as its own Unix user without access to
  `/etc/baylee`. The CLIs already run tool-less and every answer is checked
  for a lockdown breach (`docs/llm-seat.md` §"A CLI as the model").
- **Budget.** Each subscription gets a daily cap for the server's AIs, so the
  owner's development keeps enough quota. DeepSeek uses the spend book's day
  and month caps (`caps.day_usd`, `caps.month_usd`).
- Every AI-against-AI game is one more engine process (two on the replicated
  path). The carousel cap bounds it.

## Prerequisites and open checks

- **Training opt-out.** Opting a player out of training is not built
  (`docs/privacy.md`). Games of people who registered after 28.10.2026 enter
  training only once it is.
- **Subscription terms.** Whether the CLI subscriptions' terms cover serving
  opponents to the public is an open check. The owner has accepted the risk.
- **Transparency.** Labelling every AI everywhere is what the EU AI Act's
  transparency duty for systems that interact with people asks for; to be
  confirmed against the text, not assumed.
- **Legal basis for public replays.** Publishing a person's name and game is
  to be checked: a consent needs an active opt-in, legitimate interest (the
  basis the owner named for report attachments) is the alternative to weigh.
  Not assumed either way.
- **Privacy and terms.** `docs/privacy.md` and the terms gain the AI opponent,
  the provider transfer, public replays and the leaver ledger.
- **Fan Content Policy.** Unchanged: free, unofficial, no new assets beyond
  original icons.

## Where the work falls

| Area | Work |
| --- | --- |
| Central (separate repository) | AI accounts flagged by an admin, admin switch, several games at once for AI accounts, fill-in after the countdown, carousel cap, half stakes and the half loss, leaver ledger, the frozen-game state; later the replicated-path rulings |
| Gateway and engine server | `seer` as an in-engine ranked seat, the single-host pause, stand-in, slow pacing and claim, freeze and resume on the starting build, keeping old engine builds |
| `baylee-seat` and the seat agent | a ranked runner that signs in as an AI account, takes its assignments and plays them (admission, deck, presence, renewal), several games per account, the speed measurement |
| Client | countdown and "Nur Menschen", AI badges and icons, house-fallback and stand-in notices, frozen-game resume or concede, AI profiles, deck chronicle, "Bezwinger" list, public-replay checkbox |
| Feedback service | "KI-Meldungen" column and weekly pre-sort |
| Docs | `docs/privacy.md`, the terms, `docs/client-lobby.md` §"Ranked play", `docs/protocol.md` |

## Stages

Each stage lands on its own: Baylee through `cargo dev land` from the thread's
own clone, Central only through a pull request with its own required checks
(Central `AGENTS.md`). Model and effort per lane follow the owner's split:
Central and the engine server at Opus xhigh (one engine agent at a time),
seat bridge, gateway and client at Opus high, tests and mechanical rebases at
Sonnet medium.

0. **Ground.** The concept on `main`. Bump Baylee's `central-api` pin to
   Central's current `main` as a step of its own.
1. **First slice, no tokens.** One fair house account ("Haus-KI · Expert")
   fills a lonely search on the local stack, end to end.
   - Central: a design note under its three-pass security method first.
     Default: ordinary accounts that an admin flags as AI (kind, profile,
     enabled, how many games at once); the seat agent signs in as them. Then
     the flag, the admin switch, fill-in on the 60th second unless the ticket
     says "Nur Menschen", several games per AI account, and the profile's AI
     field. The choice of AI is one swappable policy, since the owner has
     not settled it.
   - Seat bridge: the ranked runner with the house mind.
   - Client: countdown and "Nur Menschen" on the search card, the AI badge on
     the result card, the table, the ladder row and the profile.
2. **Language models.** The measurement (profiles, game count and caps
   agreed with the owner first, since it spends his subscriptions and
   DeepSeek budget), the speed check, the language-model accounts on the same
   runner, the house-fallback notice, a deck check per format before the
   roster relies on any deck, the Leih-Deck pool, calibration and the
   carousel cap.
3. **Disconnects** on the single-host path: pause, slow stand-in, claim,
   freeze against an AI, the three-hour freeze, old engine builds kept;
   Central's half stakes, half loss, daily forgiveness and leaver ledger;
   the client's notices and the resume-or-concede choice.
4. **`seer`** as an in-engine ranked seat.
5. **Profiles and replays**: public opt-in, "Bezwinger", deck chronicle,
   reasoning after the game.
6. **Tower runs, the "KI-Meldungen" column, privacy and terms.**
7. Later: the replicated-path rulings, the deck optimiser, Baylee AI's
   deckbuilding.
