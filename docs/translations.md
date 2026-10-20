# Translations

The interface's words are Project Fluent files, one per table and language:
`crates/baylee-client-core/locales/<lang>/{interface,replay,chat}.ftl` for the
client (`docs/client.md` §"The interface's own words") and
`web/feedback/locales/<lang>/feedback.ftl` for the feedback service's UI
(`docs/feedback.md`). English decides what exists; another language only
translates, and a test refuses a file that lacks a message or drops an
argument. Card text is not translated here: it comes from the catalog, in
the printing's own language (`docs/protocol.md` §"Card text").

## Adding a language

1. Add the code to `Lang` (`crates/baylee-client-core/src/i18n/lang.rs`) and
   to the web UI's language list.
2. Copy `locales/en/` to `locales/<code>/` and translate the values. Keep
   every message id, every `{ $name }` argument and every selector's
   `*[other]` default; add the plural categories the language has (CLDR:
   Polish and Russian have `one`, `few`, `many`; Japanese, Korean and Chinese
   only `other`).
3. A font that draws the script, its licence recorded in `docs/legal.md`.
4. `cargo test -p baylee-client-core i18n::` and the web UI's tests.

## Rules for every language

- **Magic's own terms are the official ones of that language**, as its
  printed cards and Comprehensive Rules spell them: a player who reads cards
  in German reads the same words at our table. Look a term up in German card
  text in the catalog (`cards.lang = 'de'`), not from memory.
- **Words the language's players already say in English stay English**:
  Replay, Chat, Lobby, Deck, Sideboard, Mulligan, Commander, Gateway, Server,
  Client, Update, Screenshot, Feedback, Link, Token (a language model's),
  Account. Translating them reads as a machine's work.
- Address the player as the language does in games (German: *du*).
- Pauses are an en dash with spaces (` – `), quotes are the language's own
  (German „…“ and ‚…‘), the apostrophe is ’.
- A plural is a selector on the count (`{ $count -> [one] … *[other] … }`),
  never a bracket (`Karte(n)`).
- Keep the length near the English one: labels sit on buttons and tabs
  sized for it.

## German

| English | German | Note |
|---|---|---|
| replay | Replay, Replays | not *Wiederholung* |
| record (a game's) | Aufzeichnung | |
| account | Account | not *Konto* |
| password | Passwort | not *Kennwort* |
| report (bug) | Bericht | *einen Bericht senden* |
| reporter (of a report) | Verfasser | not *Melder* |
| seat | Platz | a chair at the table; *seat bridge* is the program, Seat-Bridge |
| seat agent | Seat-Agent, Seat-Agenten | the program that hosts model seats, as Seat-Bridge |
| beta key | Beta-Schlüssel | an account it lets in is *freigeschaltet* |
| room | Raum | a waiting room in the lobby |
| game | Partie, Spiel | |
| duel | Duell | |
| draw (result) | Unentschieden | *Unentschieden anbieten*, not *Remis* |
| token (Magic) | Spielstein | |
| token (language model) | Token, Tokens | |
| counter | Marke | `+1/+1-Marke`, *Giftmarken* |
| stack | Stapel | |
| battlefield | Spielfeld | card text says *ins Spiel* / *im Spiel* |
| library | Bibliothek | |
| graveyard | Friedhof | |
| exile | Exil | |
| command zone | Kommandozone | |
| commander | Commander | the format and the card; German printings since 2024 (before: *Kommandeur*) |
| step | Segment | *Versorgungssegment*, *Ziehsegment*, *Angreifer-deklarieren-Segment*, *Blocker-deklarieren-Segment*, *Kampfschadensegment*, *Endsegment*, *Aufräumsegment* |
| phase | Phase | *Hauptphase*, *Kampfphase* |
| turn | Zug | |
| priority | Priorität | |
| spell | Zauberspruch | |
| instant, sorcery | Spontanzauber, Hexerei | |
| permanent | Permanent, Permanents | *das Permanent*; German printings since 2024 (before: *bleibende Karte*) |
| mana value | Manabetrag | not *Manawert* |
| pile (Fact or Fiction) | Haufen | *Stapel* is the stack |
| crew, miracle | Bemannen, Mirakulum | |
| reveal | offen vorzeigen | *aufdecken* is turning a card face up |
| game log | Protokoll | *Log* only as the chat tab's name |
| badge | Abzeichen | *Marke* is a counter |
| handle (`Name#0012`) | Handle | |
| retry | Erneut versuchen | *Erneut* only on a very narrow button |
| verify (a chat key) | verifizieren | as Signal and WhatsApp say; *bestätigen* is confirm |
| safety number | Sicherheitsnummer | |
| key names | Shift, Enter, Leertaste | as printed on German keyboards' help pages: Shift, not *Umschalt* |
| ability | Fähigkeit | *ausgelöste*, *aktivierte*, *Manafähigkeit* |
| tap, untap | tappen, enttappen | |
| scry, mill | Hellsicht, millen | |
| life | Lebenspunkte | |
| opponent | Gegner | |
