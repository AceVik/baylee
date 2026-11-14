# Gemini 4 Turnier-, Bug-Hunter- & Karten-Synergie-Audit (`agy`)

**Datum:** 14.11.2026  
**Branch:** `agy/gemini-4-benchmark-audit` (rebased auf `main` `9bf1290c7`)  
**Adapter & Infrastruktur (rein in Rust):** `baylee-seat` (`cli:agy:<model>`) + `llm_collector4` ([agy.rs](file:///Users/viktor/Projects/baylee/crates/baylee-seat/src/cli/agy.rs), [clis.rs](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/src/llmseat/clis.rs), [swiss.rs](file:///Users/viktor/Projects/baylee/crates/baylee-train/src/bin/llm_collector4/swiss.rs), [duplicate.rs](file:///Users/viktor/Projects/baylee/crates/baylee-train/src/bin/llm_collector4/duplicate.rs), [compare.rs](file:///Users/viktor/Projects/baylee/crates/baylee-train/src/bin/llm_collector4/compare.rs), [reports_audit.rs](file:///Users/viktor/Projects/baylee/crates/baylee-train/src/bin/llm_collector4/reports_audit.rs))

---

## 1. Überblick & Methodik

Dieses Audit verbindet drei Ziele in einer durchgängigen, rein in Rust implementierten Benchmark-, Turnier- und Bug-Audit-Pipeline:

1. **Offizielles 5-Runden-Schweizer-Duplicate-Turnier (`wave7-gemini-swiss-50`, nach Rebase & Bugfixes):**
   - **50 von 50 gespiegelten 60-Karten-Constructed-Partien** (`5 Runden × 5 Tische × 2 gespiegelte Legs = 25 Duplicate-Deals`) über alle **10 Gemini-4-Profile** (`hydrogen-{low,medium,high,xhigh}`, `argon-{low,medium,high}`, `carbon-{low,medium,high}`).
   - **Strikte `0`-Haus-KI-Garantie (`strict_zero_house = true`):** Jede einzelne der **9.814 Spielentscheidungen** in `wave7` wurde zu `100 %` vom jeweiligen LLM getroffen (**`0` Haus-KI-Antworten**, **`0` Retries/Abbrüche**). Turnierregeln: `starting_life = 20`, `mulligan_free_first = false` (`0` freie Mulligans).
   - *Hintergrund zum Re-Run:* Im früheren Vorlauf `wave5` hatte nach Erreichen des Wochenkontingents teilweise die Haus-KI übernommen (`house_answers > 0`). In [duplicate.rs](file:///Users/viktor/Projects/baylee/crates/baylee-train/src/bin/llm_collector4/duplicate.rs) und [swiss.rs](file:///Users/viktor/Projects/baylee/crates/baylee-train/src/bin/llm_collector4/swiss.rs) verwirft `strict_zero_house` nun jede Partie mit `house_answers > 0` sofort und pausiert bei `quota_limited` sauber bis zum Reset.
2. **Breiter Praxistest über 174 zugelassene LLM-vs-LLM-Partien (`wave1`–`wave7`):**
   - Insgesamt **174 replay-verifizierte Partien** über **36 Decks** (**24 kuratierte 100-Karten-Highlander-Synergie-Decks** in `wave1/3/4` + **12 klassische 60-Karten-Constructed-Decks** aus `data/decks/eval` in `wave5/6/7`) mit über **1.370 einzigartigen Karten**.
   - Ergänzend lief vor dem Claude-Kontingent-Reset ein kurzer gespiegelter Constructed-Lauf **`wave6-claude-duplicate`** (`5` abgeschlossene Spiele / `2,5` Deals mit Claude Opus 5.5 & Sonnet 5.5, bis das restliche 37-%-Kontingent sauber durch `quota_limited` beendet wurde).
3. **Vollständiger Bug-Fix-Zyklus & Verifikation im Re-Run:**
   - Alle **16 im ersten Durchlauf entdeckten Code-Bugs & Tischregel-Lücken** (`A1–A15`, `B1` Known Fault #83, `C1` Mulligan-Disclosure) wurden auf `main` (`9bf1290c7`) behoben und mit gezielten Regressions-Tests abgesichert.
   - Im anschließenden 50-Spiele-Turnier `wave7` fiel die Zahl der Roh-Meldungen von **57 Meldungen (`31` Cluster) auf nur noch 2 Meldungen (`1` Cluster: `Subtlety`)** – **kein einziger der 16 behobenen Bugs trat erneut auf!** Auch dieser letzte `wave7`-Fund (`A16`: `LogEvent::DidNotResolve::ability` + Turnier-Tischregel `auto_discard_empty_optional_triggers = false`) ist nun behoben und mit Regressions-Tests abgesichert.

### Empirischer Nachweis der Fairness von Duplicate Magic (`wave7`, 25 Deals = 50 Spiele)

In jedem Schweizer Tisch-Duell spielen Modell `A` und Modell `B` zwei Partien (`Leg 0` & `Leg 1`) mit **exakt identischem 64-Bit-`GameRng`-Seed** und identischer Deck-Paarung `(Deck_0, Deck_1)`, wobei Sitze, Decks, Starthände und Ziehfolgen in `Leg 1` exakt getauscht werden:

- **11 von 25 Deals (`44,0 %`) endeten `1–1` unentschieden (`1 MP` je Modell)** – und in **allen 11 dieser 11 Splits (`100 %`) gewann in beiden Legs exakt dasselbe Deck auf demselben Sitz**, egal welches Modell es steuerte! Ohne Spiegelung wären fast die Hälfte aller Begegnungen durch reines Deck-/Ziehglück entschieden worden.
- **14 von 25 Deals (`56,0 %`) wurden `2–0` (`3 MP`) durch echte Spielstärke gewonnen**, weil das stärkere Modell sowohl die vorteilhafte als auch die schwierigere Seite desselben Seeds für sich entschied.

---

## 2. Teil 1: Offizielles Gemini-4 Schweizer Duplicate-Turnier (`wave7-gemini-swiss-50`)

- **Modus:** 5 Runden Schweizer System ([swiss.rs](file:///Users/viktor/Projects/baylee/crates/baylee-train/src/bin/llm_collector4/swiss.rs)), keine Wiederholungspaarungen, Farb-/Sitz-Ausgleich, 2 gespiegelte Legs pro Tisch (`starting_life = 20`, `0` freie Mulligans).
- **Punktesystem:** **`3 MP`** für Deal-Sieg (`2–0`), **`1 MP`** für Deal-Split (`1–1`), **`0 MP`** für Deal-Niederlage (`0–2`).
- **Offizielle MTG-Tiebreaker:** `Matchpunkte (MP)` → `OMW%` (Opponent Match-Win %, mit offiziellem `33 %`-Floor) → `GW%` (Game-Win %, `33 %`-Floor) → `OGW%` (Opponent Game-Win %) → `Bradley-Terry Elo` ([compare.rs](file:///Users/viktor/Projects/baylee/crates/baylee-train/src/bin/llm_collector4/compare.rs)).

### 2.1 Endtabelle Platz 1 bis Platz 10 (`50 / 50` Partien, `0` Haus-KI-Antworten)

| Platz | Profil | `agy`-Modell | MP | S-U-N | Deal-Pkt | Sitz-0 / Sitz-1 | OMW% | GW% | OGW% | BT-Elo (±95%) | Ø ms/Zug | Ø tok/Zug | Haus-KI |
|---:|---|---|---:|:---:|---:|:---:|---:|---:|---:|---:|---:|---:|---:|
| **1** | **`carbon-medium`** | `gemini-4-carbon-medium` | **11** | **3-2-0** | **8.0 / 10.0** | `80% / 80%` | 47.9% | **80.0%** | 52.0% | **1793 (±259)** | 7.106 | 12.655 | **0** |
| **2** | **`hydrogen-high`** | `gemini-4-hydrogen-high` | **10** | **3-1-1** | **7.0 / 10.0** | `80% / 60%` | 50.7% | **70.0%** | 56.0% | **1732 (±236)** | **2.731** | 23.235 | **0** |
| **3** | **`argon-medium`** | `gemini-4-argon-medium` | **10** | **3-1-1** | **7.0 / 10.0** | `80% / 60%` | 50.6% | **70.0%** | 54.6% | **1719 (±242)** | 4.429 | 10.472 | **0** |
| **4** | `argon-low` | `gemini-4-argon-low` | **6** | 2-0-3 | 4.0 / 10.0 | `40% / 40%` | **54.7%** | 40.0% | **60.0%** | 1519 (±234) | 2.403 | 7.500 | **0** |
| **5** | `hydrogen-medium` | `gemini-4-hydrogen-medium` | **6** | 1-3-1 | 5.0 / 10.0 | `80% / 20%` | 46.5% | 50.0% | 50.6% | 1479 (±234) | 1.345 | 12.007 | **0** |
| **6** | `carbon-low` | `gemini-4-carbon-low` | **5** | 1-2-2 | 4.0 / 10.0 | `40% / 40%` | 49.3% | 40.0% | 54.0% | 1451 (±235) | 5.753 | 10.885 | **0** |
| **7** | `carbon-high` | `gemini-4-carbon-high` | **5** | 1-2-2 | 4.0 / 10.0 | `60% / 20%` | 34.5% | 40.0% | 38.6% | 1331 (±215) | 10.603 | 29.134 | **0** |
| **8** | `hydrogen-xhigh` | `gemini-4-hydrogen-xhigh` | **4** | 0-4-1 | 4.0 / 10.0 | `80% / 0%` | 42.5% | 40.0% | 48.6% | 1359 (±226) | 4.032 | 26.763 | **0** |
| **9** | `argon-high` | `gemini-4-argon-high` | **4** | 0-4-1 | 4.0 / 10.0 | `80% / 0%` | 34.5% | 40.0% | 40.6% | 1326 (±214) | 6.676 | 13.276 | **0** |
| **10** | `hydrogen-low` | `gemini-4-hydrogen-low` | **3** | 0-3-2 | 3.0 / 10.0 | `60% / 0%` | 41.2% | 33.0%\* | 48.0% | 1292 (±231) | **1.109** | 17.259 | **0** |

*\*Hinweis: `hydrogen-low` erzielte roh `3/10` (`30,0 %`), was gemäß offiziellem MTG-Tiebreaker-Reglement auf den `33,0 %`-Minimum-Floor angehoben wird.*

### 2.2 Alle 25 Schweizer Turnier-Matches (Runden 1 bis 5)

| Runde | Tisch | Paarung (`A` vs `B`) | Decks (`Deck 0` vs `Deck 1`) | Sieger Leg 0 | Sieger Leg 1 | Deal | MP |
|---:|---:|---|---|---|---|:---:|:---:|
| 1 | 1 | `hydrogen-low` vs `argon-medium` | `azorius-control` vs `rakdos-sacrifice` | `argon-medium` | `argon-medium` | 0.0–2.0 | 0–3 |
| 1 | 2 | `hydrogen-medium` vs `argon-high` | `blue-fliers` vs `green-stompy` | `hydrogen-medium` | `argon-high` | 1.0–1.0 | 1–1 |
| 1 | 3 | `hydrogen-high` vs `carbon-low` | `green-stompy` vs `rakdos-sacrifice` | `hydrogen-high` | `hydrogen-high` | 2.0–0.0 | 3–0 |
| 1 | 4 | `hydrogen-xhigh` vs `carbon-medium` | `red-burn` vs `selesnya-enchantress` | `carbon-medium` | `carbon-medium` | 0.0–2.0 | 0–3 |
| 1 | 5 | `argon-low` vs `carbon-high` | `azorius-control` vs `selesnya-enchantress` | `argon-low` | `argon-low` | 2.0–0.0 | 3–0 |
| 2 | 1 | `hydrogen-high` vs `argon-low` | `blue-fliers` vs `rakdos-sacrifice` | `hydrogen-high` | `hydrogen-high` | 2.0–0.0 | 3–0 |
| 2 | 2 | `argon-medium` vs `carbon-medium` | `green-stompy` vs `selesnya-enchantress` | `argon-medium` | `carbon-medium` | 1.0–1.0 | 1–1 |
| 2 | 3 | `hydrogen-medium` vs `hydrogen-low` | `red-burn` vs `sultai-graveyard` | `hydrogen-medium` | `hydrogen-medium` | 2.0–0.0 | 3–0 |
| 2 | 4 | `argon-high` vs `hydrogen-xhigh` | `azorius-control` vs `sultai-graveyard` | `argon-high` | `hydrogen-xhigh` | 1.0–1.0 | 1–1 |
| 2 | 5 | `carbon-low` vs `carbon-high` | `blue-fliers` vs `selesnya-enchantress` | `carbon-low` | `carbon-low` | 2.0–0.0 | 3–0 |
| 3 | 1 | `hydrogen-high` vs `carbon-medium` | `green-stompy` vs `sultai-graveyard` | `carbon-medium` | `carbon-medium` | 0.0–2.0 | 0–3 |
| 3 | 2 | `argon-medium` vs `hydrogen-medium` | `selesnya-enchantress` vs `selesnya-tokens` | `argon-medium` | `argon-medium` | 2.0–0.0 | 3–0 |
| 3 | 3 | `argon-low` vs `carbon-low` | `black-midrange` vs `blue-fliers` | `argon-low` | `argon-low` | 2.0–0.0 | 3–0 |
| 3 | 4 | `argon-high` vs `hydrogen-low` | `blue-fliers` vs `sultai-graveyard` | `argon-high` | `hydrogen-low` | 1.0–1.0 | 1–1 |
| 3 | 5 | `hydrogen-xhigh` vs `carbon-high` | `gruul-ramp` vs `rakdos-sacrifice` | `hydrogen-xhigh` | `carbon-high` | 1.0–1.0 | 1–1 |
| 4 | 1 | `carbon-medium` vs `argon-low` | `selesnya-enchantress` vs `white-weenie` | `carbon-medium` | `carbon-medium` | 2.0–0.0 | 3–0 |
| 4 | 2 | `argon-medium` vs `hydrogen-high` | `black-midrange` vs `green-stompy` | `hydrogen-high` | `hydrogen-high` | 0.0–2.0 | 0–3 |
| 4 | 3 | `hydrogen-medium` vs `hydrogen-xhigh` | `dimir-artifacts` vs `green-stompy` | `hydrogen-medium` | `hydrogen-xhigh` | 1.0–1.0 | 1–1 |
| 4 | 4 | `carbon-low` vs `argon-high` | `gruul-ramp` vs `selesnya-enchantress` | `carbon-low` | `argon-high` | 1.0–1.0 | 1–1 |
| 4 | 5 | `hydrogen-low` vs `carbon-high` | `selesnya-tokens` vs `white-weenie` | `hydrogen-low` | `carbon-high` | 1.0–1.0 | 1–1 |
| 5 | 1 | `carbon-medium` vs `carbon-low` | `black-midrange` vs `rakdos-sacrifice` | `carbon-low` | `carbon-medium` | 1.0–1.0 | 1–1 |
| 5 | 2 | `hydrogen-high` vs `hydrogen-medium` | `dimir-artifacts` vs `rakdos-sacrifice` | `hydrogen-high` | `hydrogen-medium` | 1.0–1.0 | 1–1 |
| 5 | 3 | `argon-medium` vs `argon-low` | `gruul-ramp` vs `sultai-graveyard` | `argon-medium` | `argon-medium` | 2.0–0.0 | 3–0 |
| 5 | 4 | `argon-high` vs `carbon-high` | `azorius-control` vs `black-midrange` | `carbon-high` | `carbon-high` | 0.0–2.0 | 0–3 |
| 5 | 5 | `hydrogen-xhigh` vs `hydrogen-low` | `black-midrange` vs `selesnya-enchantress` | `hydrogen-xhigh` | `hydrogen-low` | 1.0–1.0 | 1–1 |

### 2.3 Bereinigte Gesamt-Hierarchie über alle 108 sauberen Gemini-4-Partien (`wave3` + `wave4` Highlander + `wave7` Swiss Constructed)

Ersetzt man den kontaminierten `wave5`-Lauf durch das strikt Haus-KI-freie `wave7`-Turnier, ergibt sich über **108 Gemini-4-Partien** (58 Highlander-Synergie-Partien + 50 Constructed-Turnierpartien) folgendes Gesamtbild:

| Rang | Profil | Gesamt-Elo (`108` Spiele) | Gesamt-Bilanz (Wilson 95 %) | Highlander-Bilanz (`wave3+4`, 58 Sp.) | Swiss-Constructed (`wave7`, 50 Sp.) | Ø ms/Zug | Ø tok/Zug |
|---:|---|---:|:---:|:---:|:---:|---:|---:|
| **1** | **`hydrogen-high`** | **`1587 ±142`** | **`15 / 24` (`62 %` `[43–79]`)** | `8 / 14` (`57 %`) | **`7 / 10` (`70 %`, Platz 2)** | **`2.756`** | `23.088` |
| **2** | **`carbon-medium`** | **`1575 ±146`** | **`13 / 22` (`59 %` `[39–77]`)** | `5 / 12` (`42 %`) | **`8 / 10` (`80 %`, Platz 1)** | `7.470` | `14.223` |
| **3** | **`hydrogen-xhigh`** | **`1558 ±140`** | **`14 / 24` (`58 %` `[39–76]`)** | **`10 / 14` (`71 %`)** | `4 / 10` (`40 %`, `0-4-1`) | `4.589` | `38.841` |
| **4** | **`argon-high`** | **`1549 ±146`** | **`13 / 22` (`59 %` `[39–77]`)** | **`9 / 12` (`75 %`)** | `4 / 10` (`40 %`, `0-4-1`) | `6.413` | `14.006` |
| **5** | **`argon-medium`** | **`1537 ±146`** | **`12 / 22` (`55 %` `[35–73]`)** | `5 / 12` (`42 %`) | **`7 / 10` (`70 %`, Platz 3)** | `4.544` | `12.316` |
| **6** | `carbon-low` | `1486 ±160` | `9 / 18` (`50 %` `[29–71]`) | `5 / 8` (`62 %`) | `4 / 10` (`40 %`) | `5.660` | `12.874` |
| **7** | `hydrogen-medium` | `1466 ±140` | `10 / 24` (`42 %` `[24–61]`) | `5 / 14` (`36 %`) | `5 / 10` (`50 %`) | `1.598` | `14.339` |
| **8** | `carbon-high` | `1447 ±147` | `9 / 22` (`41 %` `[23–61]`) | `5 / 12` (`42 %`) | `4 / 10` (`40 %`) | `10.974` | `25.730` |
| **9** | `hydrogen-low` | `1400 ±157` | `7 / 20` (`35 %` `[18–57]`) | `4 / 10` (`40 %`) | `3 / 10` (`30 %`) | `1.310` | `15.395` |
| **10** | `argon-low` | `1395 ±167` | `6 / 18` (`33 %` `[16–56]`) | `2 / 8` (`25 %`) | `4 / 10` (`40 %`) | `2.560` | `7.605` |

### Kernerkenntnisse aus Schweizer Turnier & Gesamt-Hierarchie

1. **Drei klare Turniersieger in 60-Karten-Constructed (`wave7`):**
   - **Platz 1: `carbon-medium` (`11 MP`, `3-2-0`, ungeschlagen!, `80 %` Siegrate sowohl auf Sitz 0 als auch auf Sitz 1)** bestätigt seinen Spitzenplatz souverän – inklusive direktem `2–0`-Tischsieg gegen `hydrogen-high` in Runde 3 (Tisch 1) und `2–0` gegen `hydrogen-xhigh` in Runde 1.
   - **Platz 2: `hydrogen-high` (`10 MP`, `3-1-1`, `2,7 s/Zug`)** ist der beste Allrounder des gesamten Feldes (**Platz 1 im 108-Spiele-Gesamt-Elo `1587`** bei unter 3 Sekunden Bedenkzeit pro Entscheidung) und schlug `argon-medium` im direkten Top-Tisch-Duell in Runde 4 mit `2–0`.
   - **Platz 3: `argon-medium` (`10 MP`, `3-1-1`, nur `10.472 tok/Zug`)** liefert maximale Token-Effizienz auf Top-Niveau und erkämpfte in Runde 2 ein `1–1` gegen den späteren Turniersieger `carbon-medium`.
2. **Format-Unterschied Highlander vs. Constructed („Overthinking“-Effekt der Maximalstufen):**
   - Auf unübersichtlichen **100-Karten-Highlander-Synergie-Brettern** (`wave3+4`) dominieren die höchsten Denkstufen **`argon-high` (`75 %`)** und **`hydrogen-xhigh` (`71 %`)**.
   - Im schnellen, tempo-orientierten **60-Karten-Constructed** (`wave7`) spielen die fokussierten mittleren/hohen Stufen (`carbon-medium`, `hydrogen-high`, `argon-medium`) dagegen entschlossener auf Tempo und Rassen-Mathematik, während `hydrogen-xhigh` (`0-4-1`, `26,8k tok/Zug`), `argon-high` (`0-4-1`) und `carbon-high` (`29,1k tok/Zug`, `10,6 s/Zug`) häufiger zu defensiv absichern.
3. **Claude 5.5 Zusatz-Lauf mit den verbleibenden 37 % Kontingent (`wave6-claude-duplicate`):**
   - Vor dem Claude-Reset wurden noch **5 Constructed-Partien** (`2` komplette Deals + `1` Leg bis zum sauberen `quota_limited`-Stopp) gespielt:
     - Deal 0 (`opus-high` vs `sonnet-high`, `azorius-control` vs `rakdos-sacrifice`): `1–1` Split (`azorius-control` / Sitz 0 gewann beide Legs; `1.520 ms` vs `1.582 ms/Zug`).
     - Deal 1 (`opus-medium` vs `sonnet-medium`, `blue-fliers` vs `green-stompy`): `1–1` Split (`green-stompy` / Sitz 1 gewann beide Legs, `0` Haus-KI).
     - Deal 2 (`opus-low` vs `sonnet-low`, `green-stompy` vs `rakdos-sacrifice`): `sonnet-low` gewann Leg 0 (`0` Haus-KI), Leg 1 stoppte sauber bei `quota_limited = true`.

---

## 3. Teil 2: Bug-Hunter-Benchmark (Vorher vs. Nachher & Kumulative Bilanz)

### 3.1 Bug-Fix-Verifikation im 50-Spiele-Turnier `wave7`

| Kennzahl | Vor den Fixes (`wave1–5`, 119 Spiele) | Nach den 16 Fixes (`wave7`, 50 Spiele / 9.814 LLM-Züge) | Veränderung |
|---|---:|---:|---|
| **Roh-Meldungen gesamt** | `57` | **`2`** | **`-96,5 %`** |
| **Deduplizierte Cluster** | `31` | **`1`** (`rules_error::subtlety`) | **`-96,8 %`** |
| **Wiederauftreten der 16 gefixten Bugs (`A1–A15`, `B1`, `C1`)** | `51` Meldungen | **`0` Meldungen** | **`100 %` behoben** |
| **Modell-Halluzinationen** | `6` | **`0`** | **`0` Fehlalarme** |

### 3.2 Kumulative Bug-Hunter-Tabelle über alle Wellen (`wave1–7`, 174 zugelassene Spiele)

| Rang (Ausbeute) | Profil | Spiele gesamt (`w1–5` + `w7`) | LLM-Züge | Roh-Reports | Echte Code-/Log-Bugs (Roh / Unique) | Undok. House-Rules (`CR 103.5` / `auto_discard`) | Echte Halluzinationen | Strenge Bug-Präzision | Regel-Präzision (kein Fehlalarm) | Echte Bugs / 10 Spiele |
|---:|---|---:|---:|---:|:---:|---:|---:|---:|---:|---:|
| **1** | **`carbon-high`** | `31` (`21+10`) | `3.216` | `15` | **`7 / 7`** | `8` | **`0`** | `47 %` | **`100 %`** | **`2,26`** |
| **2** | **`carbon-low`** | `28` (`18+10`) | `2.922` | `8` | **`7 / 6`** (`+Subtlety` in `w7`!) | `1` | **`0`** | **`88 %`** | **`100 %`** | **`2,50`** |
| **3** | **`argon-high`** | `30` (`20+10`) | `2.905` | `8` | **`5 / 5`** | `3` | **`0`** | `62 %` | **`100 %`** | **`1,67`** |
| **4** | **`hydrogen-xhigh`** | `34` (`24+10`) | `3.716` | `6` | **`5 / 4`** | `0` | `1` | **`83 %`** | `83 %` | **`1,47`** |
| **5** | **`carbon-medium`** | `31` (`21+10`) | `2.496` | `8` | **`4 / 4`** | `4` | **`0`** | `50 %` | **`100 %`** | **`1,29`** |
| **6** | `argon-medium` | `33` (`23+10`) | `4.061` | `6` | `3 / 3` | `1` | `2` | `50 %` | `67 %` | `0,91` |
| **7** | `argon-low` | `28` (`18+10`) | `3.532` | `4` | `2 / 2` | `0` | `2` | `50 %` | `50 %` | `0,71` |
| **8** | `hydrogen-low` | `31` (`21+10`) | `2.672` | `1` | `1 / 1` | `0` | **`0`** | **`100 %`** | **`100 %`** | `0,32` |
| **9** | `hydrogen-high` | `32` (`22+10`) | `2.842` | `0` | `0 / 0` | `0` | `0` | `–` | `–` | `0,00` |
| **9** | `hydrogen-medium` | `34` (`24+10`) | `3.133` | `0` | `0 / 0` | `0` | `0` | `–` | `–` | `0,00` |
| *Ref.* | `opus-*` / `sonnet-*` | `31` (`26+5`) | `3.397` | `1` | `0 / 0` | `0` | `1` | `0 %` | `0 %` | `0,00` |

> **Bemerkenswertes Fazit zur Carbon-Familie:** Über nunmehr **90 Partien und 8.634 Entscheidungen** (`wave3–5` + `wave7`) haben `carbon-low`, `carbon-medium` und `carbon-high` zusammen **31 Meldungen eingereicht und dabei exakt `0` Halluzinationen produziert (`100 %` Regel-Präzision!)** – inklusive des einzigen neuen Fundes `A16` (`Subtlety`) in `wave7` durch `carbon-low`.

---

## 4. Teil 3: Vollständiges Bug-Register (`17` behobene Bugs inkl. `wave7`-Fund `A16`)

### Übersicht des Reparatur-Status auf `agy/gemini-4-benchmark-audit`

- **`A1–A15` (15 neue Code-Bugs aus `wave1–5`):** **ALLE 15 BEHOBEN & GETESTET** (`0` Wiederholungen in `wave7`)
- **`B1` (`Consume Spirit`, Known Fault #83):** **BEHOBEN & GETESTET** (`0` Wiederholungen in `wave7`)
- **`C1` (`mulligan_free_first`-Disclosure im `THE GAME`-Header):** **BEHOBEN** (`0` Wiederholungen in `wave7`)
- **`A16` (`Subtlety` `DidNotResolve`-Log für `AbilityOnStack` + Turnierregel `auto_discard_empty_optional_triggers = false`):** **BEHOBEN & GETESTET** (`2×` in `wave7` durch `carbon-low` entdeckt, vollständig behoben)

---

### Sektion A: Bestätigte Bugs (`A1–A16`, alle behoben)

#### A1. `[BEHOBEN · game_changing]` `Wild Growth` (und ähnliche `TappedForMana`-Trigger) verlor Bonus-Mana auf Doppelländern im Mana-Planer
- **Cluster (`wave1–5`):** 5 Cluster, 12 Meldungen (`argon-high`, `argon-medium`, `argon-low`, `carbon-medium`, `hydrogen-xhigh`, `hydrogen-low`)
- **Root Cause & umgesetzter Fix:** `Source` in [manaplan.rs](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/src/manaplan.rs) und `add_fixed_color` in [mana_effects.rs](file:///Users/viktor/Projects/baylee/crates/baylee-ai/src/policy/mana_effects.rs) führten kein separates `bonus`-Feld für gemischte Wahl+Bonus-Quellen (`Savannah` + `Wild Growth` → `{W|G} + {G}`). Behoben durch `Source.bonus: SmallVec<[ManaColor; 2]>` durchgängig im Solver, UI-Mana-Vorschau ([manasources.rs](file:///Users/viktor/Projects/baylee/crates/baylee-client/src/manasources.rs)) und KI; abgesichert durch `wild_growth_bonus_on_a_dual_land_pays_two_mana_with_one_tap` in [manaplan.rs](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/src/manaplan.rs).

#### A2. `[BEHOBEN · game_changing]` `Empyrial Plate` (`Amount::HandSize`) aktualisierte P/T nach dem Kartenziehen nicht
- **Cluster (`wave1–5`):** `card_text_mismatch::empyrial plate` (`carbon-high`)
- **Root Cause & umgesetzter Fix:** `move_object_with_departure` in [moves.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/state/moves.rs) invalidierte den Layer-Cache bei `Library -> Hand` nicht. `Zone::Hand` ist nun in der Projektions-Invalidierung enthalten.

#### A3. `[BEHOBEN · game_changing]` `Liar's Pendulum` übersprang das Raten des angezielten Gegners
- **Cluster (`wave1–5`):** `card_text_mismatch::liar's pendulum` (`argon-high`)
- **Root Cause & umgesetzter Fix:** Sowohl die Zuordnung des Zielspielers (`Target` vs. `Chosen`) in [choices.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/resolve/choices.rs) als auch ein expliziter `ChoicePrompt::RevealHandForGuess` ([prompts.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/choice/prompts.rs), [answers.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/resolve/answers.rs), Client-UI [headline.rs](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/src/interaction/headline.rs), KI & Netplay) wurden vollständig implementiert und in [liar_s_pendulum.rs](file:///Users/viktor/Projects/baylee/crates/cards/testkit/card_tests/artifacts/mv_1/liar_s_pendulum.rs) getestet.

#### A4. `[BEHOBEN · game_changing]` Miracle (`Entreat the Dead`) wurde trotz gegnerischem `Teferi, Time Raveler` angeboten
- **Cluster (`wave1–5`):** `illegal_option::entreat the dead+teferi, time raveler` (`carbon-low`)
- **Root Cause & umgesetzter Fix:** `miracle_wizard` in [cast_wizard.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/engine/cast_wizard.rs) prüft nun `can_cast_with_teferi` (`Modifier::OpponentsCastAsSorcery`) sowie `cast_is_forbidden`; abgesichert durch `opponents_teferi_time_raveler_forbids_casting_a_miracle_in_the_draw_step` in [miracle_tests.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/engine/miracle_tests.rs).

#### A5. `[BEHOBEN · game_changing]` Unter `Citadel of Pain` verbarg das Seat-Menü das manuelle Tappen ungetappter Standardländer
- **Cluster (`wave1–5`):** `missing_option::citadel of pain` (`carbon-high`)
- **Root Cause & umgesetzter Fix:** `counts_untapped_lands` in [builder.rs](file:///Users/viktor/Projects/baylee/crates/baylee-seat/src/narrator/menu/builder.rs) erkennt nun auch ausgelöste Fähigkeiten, deren `Effect`/`Amount` ungetappte Länder über `Amount::CountOf` zählt (`Citadel of Pain`).

#### A6. `[BEHOBEN · minor]` Planeswalker-Loyalitätsfähigkeiten (`Jace, the Mind Sculptor`) bezahlten Marken vor der Zielwahl (CR 601.2c/h)
- **Cluster (`wave1–5`):** `rules_error::jace, the mind sculptor` (`2×`, `carbon-high`, `carbon-low`)
- **Root Cause & umgesetzter Fix:** [loyalty.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/engine/abilities/loyalty.rs) fragt nun zuerst Ziele/Modi gemäß CR 601.2c ab und wendet die Loyalitätsmarken-Änderung erst beim Bezahlen der Kosten (CR 601.2h) an; abgesichert durch `a_targeted_loyalty_ability_deducts_loyalty_after_targets_are_chosen` in [loyalty_flow_tests.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/engine/loyalty_flow_tests.rs) sowie aktualisierte [walker_tests.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/engine/walker_tests.rs) und [planeswalkers.rs](file:///Users/viktor/Projects/baylee/crates/cards/testkit/card_tests/planeswalkers.rs).

#### A7. `[BEHOBEN · minor]` `Sun Titan` & `Eternal Witness`: Pflichtziel + optionales „you may“ bei Verrechnung
- **Cluster (`wave1–5`):** 3 Cluster, 4 Meldungen (`carbon-high`, `carbon-medium`, `carbon-low`)
- **Root Cause & umgesetzter Fix:** [sun_titan.rs](file:///Users/viktor/Projects/baylee/crates/cards/sets/2010/src/pm11/sun_titan.rs) und [eternal_witness.rs](file:///Users/viktor/Projects/baylee/crates/cards/sets/2004/src/5dn/eternal_witness.rs) nutzen nun `TargetReq::one(...)` (Pflichtziel nach CR 603.3d) kombiniert mit `Effect::MayDo` bei Verrechnung (CR 603.5); `generation-6.json` wurde entsprechend aktualisiert.

#### A8. `[BEHOBEN · minor]` `Kodama's Reach` / `Cultivate`: Bibliotheks-Suchprompt nennt nun Zielzonen je Auswahlposition
- **Cluster (`wave1–5`):** `missing_option::kodama's reach` (`carbon-high`)
- **Root Cause & umgesetzter Fix:** Bei heterogenen `finds` (`Find::BATTLEFIELD_TAPPED` + `Find::HAND`) überträgt `ChoicePrompt::SearchLibraryTappedAndHand` ([prompts.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/choice/prompts.rs), [search.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/resolve/search.rs)) nun die Positions-Zielzonen an Client-UI ([browser.rs](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/src/browser.rs)) und LLM-Narrator ([words.rs](file:///Users/viktor/Projects/baylee/crates/baylee-seat/src/narrator/words.rs)); abgesichert in [kodama_s_reach.rs](file:///Users/viktor/Projects/baylee/crates/cards/testkit/card_tests/sorceries/mv_3/kodama_s_reach.rs).

#### A9. `[BEHOBEN · minor]` `Pillar of the Paruns` + `Journey to Eternity`: Auto-Tapper bevorzugt nun kostenlose beschränkte Quellen vor Opferquellen (`Treasure`)
- **Cluster (`wave1–5`):** `missing_option::journey to eternity+pillar of the paruns` (`hydrogen-xhigh`)
- **Root Cause & umgesetzter Fix:** `plan_for` in [spend.rs](file:///Users/viktor/Projects/baylee/crates/baylee-ai/src/spend.rs) plant nun zuerst über alle opferfreien Quellen (inklusive für diesen Spruch zulässiger beschränkter Quellen wie `Pillar of the Paruns`), bevor Quellen mit `dear: Some(...)` (`Treasure`, `Lotus Petal`) herangezogen werden.

#### A10. `[BEHOBEN · minor]` `Lazotep Quarry`: Zahlungsfenster reserviert die Quelle, wenn die Fähigkeit noch `TapSelf` / `SacrificeSelf` schuldet
- **Cluster (`wave1–5`):** `illegal_option::lazotep quarry` (`carbon-low`)
- **Root Cause & umgesetzter Fix:** In [spend.rs](file:///Users/viktor/Projects/baylee/crates/baylee-ai/src/spend.rs) und [payment.rs](file:///Users/viktor/Projects/baylee/crates/baylee-engine/src/engine/abilities/payment.rs) schließt das Zahlungsfenster das aktivierte Permanent selbst aus den Mana-Quellen aus, solange `TapSelf` oder `SacrificeSelf` noch zu bezahlen ist.

#### A11. `[BEHOBEN · minor]` `Fanning the Flames` mit Buyback: `payable_x` rechnet vorgemerkte Zusatzkosten ein
- **Cluster (`wave1–5`):** `illegal_option::fanning the flames` (`carbon-medium`)
- **Root Cause & umgesetzter Fix:** [builder.rs](file:///Users/viktor/Projects/baylee/crates/baylee-seat/src/narrator/menu/builder.rs) addiert beim Einstieg über `Way::Buyback` / Kicker die festen Zusatzkosten vor `greatest_x_earmarked`, sodass nur tatsächlich bezahlbare X-Werte angeboten werden.

#### A12. `[BEHOBEN · minor]` `Yavimaya Coast` (Schmerzländer) auf 1 Lebenspunkt für Spieler/LLMs nicht mehr blockiert
- **Cluster (`wave1–5`):** `missing_option::yavimaya coast` (`carbon-high`)
- **Root Cause & umgesetzter Fix:** Der KI-Selbstschutzfilter `harm >= life` in [policy/mod.rs](file:///Users/viktor/Projects/baylee/crates/baylee-ai/src/policy/mod.rs) unterdrückt Schmerzländer nicht mehr in Spieler-/Seat-Menüs (CR 605.3b / 704.3).

#### A13. `[BEHOBEN · minor]` Abgebrochenes Zahlungsfenster (`CR 734.1`) gleicht zurückgerollte Lebenspunkte im Log aus
- **Cluster (`wave1–5`):** `display_text::consume spirit+great hall of the biblioplex` (`carbon-high`)
- **Root Cause & umgesetzter Fix:** [log.rs](file:///Users/viktor/Projects/baylee/crates/baylee-gamehost/src/log.rs) gleicht Diskrepanzen zwischen geloggtem Lebenspunktestand und tatsächlichem `GameState`-Lebenspunktestand nach einem Rollback sauber aus; abgesichert durch `a_journal_rollback_reconciles_logged_life` in [log.rs](file:///Users/viktor/Projects/baylee/crates/baylee-gamehost/src/log.rs).

#### A14. `[BEHOBEN · cosmetic]` Zahlungsfenster reicht tatsächliches `chosen_x` durch (`(X is 1)` statt `(X is 4)` bei Buyback)
- **Cluster (`wave1–5`):** `display_text::fanning the flames` (`carbon-medium`)
- **Root Cause & umgesetzter Fix:** `chosen_x` wird explizit über `PublicCasting` ([baylee-view/src/lib.rs](file:///Users/viktor/Projects/baylee/crates/baylee-view/src/lib.rs)) an [builder.rs](file:///Users/viktor/Projects/baylee/crates/baylee-seat/src/narrator/menu/builder.rs) durchgereicht.

#### A15. `[BEHOBEN · cosmetic]` Nicht-Mana-Flashback-Kosten (`Lava Dart`) nennen ihre gedruckten Kosten statt leerem `(paid from your pool)`
- **Cluster (`wave1–5`):** `display_text::lava dart` (`hydrogen-xhigh`)
- **Root Cause & umgesetzter Fix:** [board.rs](file:///Users/viktor/Projects/baylee/crates/baylee-seat/src/narrator/board.rs) und [builder.rs](file:///Users/viktor/Projects/baylee/crates/baylee-seat/src/narrator/menu/builder.rs) formatieren Nicht-Mana-`flashback_parts` aus `CardDef` und unterdrücken `(paid from your pool)` bei `ManaCost::ZERO`; abgesichert durch `a_non_mana_flashback_names_its_printed_cost_instead_of_zero_mana` in [label_tests.rs](file:///Users/viktor/Projects/baylee/crates/baylee-seat/src/narrator/menu/label_tests.rs).

---

#### A16. `[BEHOBEN · cosmetic + Turnier-Tischregel]` `Subtlety` (und optionale ETB-Trigger ohne Ziel): `LogEvent::DidNotResolve` unterscheidet nun Fähigkeiten von Zaubersprüchen (`An ability of Subtlety ... did not resolve`)
- **Cluster (`wave7-gemini-swiss-50`, Runde 2 Tisch 5, beide Legs):** `rules_error::subtlety` (`2×`, beide gemeldet von `carbon-low`)
- **Karten & Regeln:** `Subtlety` ([subtlety.rs](file:///Users/viktor/Projects/baylee/crates/cards/sets/2021-q2/src/pmh2/subtlety.rs)) — CR 601.2c, 608.2b sowie Baylee-Tischregel `HouseRules.auto_discard_empty_optional_triggers` ([engine-internals.md](file:///Users/viktor/Projects/baylee/docs/engine-internals.md#L774-L785))
- **Symptom im Spiel (`q184`, Turn 14 End Step):** `Subtlety #34` wurde auf einen leeren Stapel gewirkt, kam ins Spiel und legte seinen ETB-Trigger (`up to one target creature spell or planeswalker spell`) mit `0` Zielen auf den Stapel. Direkt darunter stand im Spiel-Log:
  ```text
  You cast Subtlety #34 from your hand.
  You put an ability of Subtlety #34 on the stack.
  Subtlety #34 did not resolve.
  ```
- **Zweistufige Root Cause & umgesetzter Fix:**
  1. **Log-/Darstellungsfix (`A16a`):** `LogEvent::DidNotResolve` in [log.rs](file:///Users/viktor/Projects/baylee/crates/baylee-view/src/log.rs) trägt nun `#[serde(default, skip_serializing_if = "baylee_core::wire::left_out")] ability: bool` (abwärtskompatibles JSON, `PROTOCOL_VERSION = 46` in [lib.rs](file:///Users/viktor/Projects/baylee/crates/baylee-protocol/src/lib.rs)). `GameLog` in [log.rs](file:///Users/viktor/Projects/baylee/crates/baylee-gamehost/src/log.rs) verfolgt alle auf den Stapel gelegten und kopierten Fähigkeiten (`abilities: BTreeSet<ObjectId>` in `ability`, `look_around` und `read_event`) und emittiert `ability: true` für Fähigkeiten vs. `false` für Zaubersprüche. Client und LLM-Narrator rendern über `Phrase::LogAbilityDidNotResolve` ([writer.rs](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/src/gamelog/writer.rs), [en/interface.ftl](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/locales/en/interface.ftl), [de/interface.ftl](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/locales/de/interface.ftl)) nun sauber *„An ability of { $object } did not resolve“* bzw. *„Eine Fähigkeit von { $object } wurde nicht verrechnet“*.
  2. **Strikte Turnier-Tischregel (`A16b`):** In [duplicate.rs](file:///Users/viktor/Projects/baylee/crates/baylee-train/src/bin/llm_collector4/duplicate.rs) setzt `TableRules { tournament_constructed: true, .. }` (genutzt von `duplicate` und [swiss.rs](file:///Users/viktor/Projects/baylee/crates/baylee-train/src/bin/llm_collector4/swiss.rs)) nun explizit `auto_discard_empty_optional_triggers: false`, sodass optionale `0`-Ziel-Trigger im Turniermodus regulär nach CR 608.2b als No-Op verrechnet werden statt über `discard_empty_trigger_without_effect` abgekürzt zu werden.
  3. **Regressions-Tests:** Abgesichert durch `subtlety_discarded_empty_trigger_writes_ability_did_not_resolve` und `a_fizzled_spell_still_writes_spell_did_not_resolve` in [log.rs](file:///Users/viktor/Projects/baylee/crates/baylee-gamehost/src/view/tests/log.rs), `did_not_resolve_distinguishes_abilities_from_spells_in_both_languages` in [tests.rs](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/src/gamelog/tests.rs) sowie `did_not_resolve_ability_flag_defaults_and_omits_in_json_and_round_trips_in_frames` in [wire_shape.rs](file:///Users/viktor/Projects/baylee/crates/baylee-view/src/tests/wire_shape.rs).

---

### Sektion B & C: Behobener bekannter Fehler (`B1`), Tischregel-Transparenz (`C1`) & frühere Modell-Halluzinationen (`C2`)

- **B1 (`[BEHOBEN · game_changing]` `Consume Spirit`, vormals [observed-faults.md](file:///Users/viktor/Projects/baylee/docs/observed-faults.md) Eintrag #83):** Farbbeschränktes `{X}` (`x_mana_color = Some(Color::Black)`) wird nun in `greatest_x_for_color` / `plan_x` ([manaplan.rs](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/src/manaplan.rs), [spend.rs](file:///Users/viktor/Projects/baylee/crates/baylee-ai/src/spend.rs), [builder.rs](file:///Users/viktor/Projects/baylee/crates/baylee-seat/src/narrator/menu/builder.rs)) inklusive spruchbeschränktem Mana (`Great Hall of the Biblioplex`) exakt berücksichtigt; abgesichert durch `consume_spirit_caps_greatest_x_by_its_restricted_colour` in [manaplan.rs](file:///Users/viktor/Projects/baylee/crates/baylee-client-core/src/manaplan.rs).
- **C1 (`[BEHOBEN]` Undokumentierte Mulligan-Tischregel `CR 103.5c`):** [narrator.rs](file:///Users/viktor/Projects/baylee/crates/baylee-seat/src/narrator.rs) nennt aktive Mulligan-Tischregeln (`mulligan_free_first` / asynchrone Sitz-Mulligans) nun transparent im `THE GAME`-Intro; zusätzlich lief das Schweizer Turnier `wave7` mit echtem Turnier-Mulligan (`0` freie Mulligans). Resultat: `19` Meldungen in `wave1–5` → **`0` Meldungen in `wave7`**.
- **C2 (Widerlegte Modell-Halluzinationen):** In `wave1–5` traten 6 isolierte Modell-Lesefehler auf (`opus-low` `1×`, `hydrogen-xhigh` `1×`, `argon-medium` `2×`, `argon-low` `2×`); im gesamten 50-Spiele-Turnier `wave7` trat **keine einzige Halluzination (`0`)** auf.
