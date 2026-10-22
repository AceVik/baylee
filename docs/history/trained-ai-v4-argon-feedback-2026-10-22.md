# Feedback und Folgeauftrag an Gemini 4 Argon: Playernet v4

**Datum:** 22. Oktober 2026.

**Repository:** `/home/ace/Projects/baylee`.

**Reviewbasis:** laufende, uncommittete v4-Arbeiten auf `30779bc920147a2a2acc713ebcb784badceced4a`.

Diese Datei ist ein vollständiger, direkt ausführbarer Arbeitsauftrag an Argon.
Die Befunde stammen aus einem unabhängigen Read-only-Review von Code,
Trainingsartefakten und Laufprotokollen. Währenddessen wurde weitergearbeitet;
Zeilennummern und Artefakte können inzwischen abweichen. Prüfe den aktuellen
Stand, bevor du einen Befund als noch offen oder bereits behoben einordnest.
Die genannten Messungen gelten für die damals vorhandenen Artefakte und
sind keine Messungen eines später neu trainierten Modells.

## Auftrag und Grundlage

Bitte unterbrich weitere größere Trainingsläufe geordnet und sichere die
vorhandenen Artefakte. Das Review hat grundlegende Probleme gefunden.
Prüfe sie gegen deinen aktuellen Arbeitsstand, repariere die Ursachen und
belege die Korrekturen mit gezielten Tests.

Grundlage bleiben:

- [Ursprüngliche Übergabe](trained-ai-v4-handover-2026-10-22.md).
- [Architektur- und Trainingsproposal](../proposals/trained-ai-v4.md).
- [Lookahead-Design und Anti-Cheat-Grenze](../proposals/lookahead-design.md).
- [AGENTS.md](../../AGENTS.md) und die dort verlinkten Vorgaben.

Behalte funktionierende Teile des Prototyps. Größere Architektur und neues
Training sind ausdrücklich erlaubt. Ziel ist eine starke universelle
Spiel-KI mit kontinuierlicher Beobachtung, gelerntem Planen und Überdenken.
Ein durchlaufendes Skript oder gute Imitationsgenauigkeit erfüllen dieses
Ziel noch nicht. Die wichtigste Anforderung bleibt: mindestens so gut und
präzise spielen wie ein guter erfahrener Spieler, nachweisbar unter fairen
Informationsbedingungen.

**Zusätzlicher Nutzerauftrag:** Importiere möglichst viele unterschiedliche
reale **Decklisten aus dem Web**, die mit dem verfügbaren, vollständig
implementierten Kartenpool spielbar sind, und nutze sie für Training und
Evaluation. „Text“ in der letzten Nutzernachricht wurde im Kontext als
„Decks“ verstanden. Der konkrete Importauftrag steht nach den Reviewbefunden.

## P0: Vor weiterem RL und belastbaren Qualitätsaussagen

### 1. Sitzmerkmale zwischen Rust und Python sind falsch zugeordnet

`crates/baylee-train/src/features4.rs:1202` ff. schreibt:

```text
[0] present, [1] self, [2] teammate, [3] opponent, [4] lost
```

`tools/trainer/model4.py:345` ff. und `:458` ff. verwenden dagegen `[2]`
als „active“ und `[1]` als „allied“.

Dadurch werden gegnerische Sitz-Tokens aus der Attention maskiert.
Im Duell/FFA bleibt für den Value-Softmax nur der eigene Sitz übrig:
Value ist identisch 1,0; der Value-Loss kann diesen Fehler nicht beheben.
Im Teamspiel konkurriert der eigene Sitz mit seinen Teamkameraden.

Am damaligen `/home/ace/baylee-data/v4/models/v4-phase-b/model.onnx`
wurde Value=1,0 direkt reproduziert. Mit einem manuell korrigierten
Active-Maskeneingang änderte sich der Wert.

Definiere ein gemeinsames, versioniertes Featureschema. Prüfe echte
Rust-Features für Duell, FFA, Teams, ausgeschiedene und unbesetzte Sitze.
Teste zusätzlich Lernfähigkeit und Gradienten des Critics; reine
PyTorch/ONNX-Parität kann denselben semantischen Fehler bestätigen.

### 2. Der RL-Datenvertrag ist unterbrochen

`crates/baylee-train/src/bin/eval4.rs:243` ff. erhält `Answer4` inklusive
Provenienz, speichert aber nur die ausgeführte Aktion im normalen
Session-Record. `crates/baylee-train/src/convert4.rs:305` ff. setzt danach:

```text
temperature=0, thinking_steps=1, behavior_logprob=0
```

`Source::Seat` wird pauschal zu `Net`.

Im untersuchten `/home/ace/baylee-data/v4/datasets/d4-rl1/train` waren:

- 61.440 Entscheidungen;
- 0 nichtnull Behavior-Logprobs;
- 38.196 `House`- und 23.244 `Net`-Einträge.

`dataset4.py` übernimmt die Herkunft nicht in den Trainingsbatch.
`train_rl4.py` verwendet alle Entscheidungen für PPO und ersetzt fehlende
Logprobs durch `lp_t.detach()`. Damit ist die Ratio numerisch immer 1,
obwohl weiter Policygradienten entstehen; PPO-Clipping schützt nicht.

Speichere die tatsächliche Behavior-Policy samt Modellversion,
Temperatur, K, Aktions-/Samplingtrace, Herkunft und gültigem
Gedächtniskontext verlustfrei. PPO nur auf geeigneten Aktionen der
aufgezeichneten Learner-Policy; House-Daten gegebenenfalls separat für BC.
Fehlende Provenienz muss einen klaren Fehler auslösen, keinen Ersatzwert.
Logprob=0 ist außerdem ein gültiger Wert und kein Missing-Sentinel.

### 3. Sampling und berechnete Wahrscheinlichkeiten stimmen nicht überein

`netplay4.rs:532` ff. erzwingt Mindestpicks und beendet die Auswahl bei
`max_k`, zählt dabei aber gewöhnliche Bernoulli-Logprobs.
Bei `min_k=max_k` ist die Auswahl nach Score deterministisch.
Der Decoder ergänzt/entfernt teilweise Picks und Blockerzuordnungen.
Die gespeicherte Wahrscheinlichkeit beschreibt damit nicht zuverlässig
die ausgeführte Aktion.

`train_rl4.py` berücksichtigt weder Rollout-Temperatur noch die tatsächlichen
Auswahlbedingungen. Dropout ist im Update aktiv, im ONNX-Collector aus.
Der Scalar-Head besitzt keinen passenden RL-Aktionswahrscheinlichkeitspfad.

Entwickle einen konsistenten legalen Decoder und eine klar definierte
Verteilung über Aktionen oder vollständig aufgezeichnete Entscheidungspfade.
Bei unveränderten Gewichten müssen Collector und Trainer dieselbe
Logprob liefern. Prüfe dies für alle unterstützten Heads, Temperaturen,
K-Werte und Fallbackfälle. Beweise die Samplerkorrektheit auch mit kleinen
vollständig aufzählbaren Beispielen.

### 4. „Immer aktiv“ und rekurrentes Training sind noch nicht umgesetzt

`convert4.rs:168` verwendet `Session::view_for` für passive Beobachter.
Diese Methode liefert laut `session/answers.rs:59` nur etwas, wenn dem
Sitz gerade eine Frage gestellt wird. Das ist kein Beobachtungsstrom.

`NetPlayer4::observe` aktualisiert lediglich `SeatKnowledge`, nicht H.
`eval4` ruft das Netz nur bei eigenen Entscheidungen auf.
Der untersuchte RL-Datensatz enthielt ausschließlich Entscheidungsframes.

Zusätzlich starten `train4.py` und `train_rl4.py` jedes zufällige
U=16-Segment mit H=0. Live wird H über das Match weitergetragen.
Der Converter entfernt inzwischen erzwungene Prioritäten und fünf von
sechs Pass-Entscheidungen. Das verändert wiederum die Gedächtnishistorie.

Implementiere einen autorisierten Beobachtungsstrom unabhängig vom Pending,
mit Cursor, Duplikat-/Lückenbehandlung und identischem Replay-/Live-Verhalten.
Trenne `observe`, `decide` und temporäres Denken Z. Ein erneutes `decide` darf
dieselbe Beobachtung nicht erneut als Ereignis verbuchen.
Verwende korrekte Segmentzustände bzw. nachvollziehbares Burn-in/Replaying.
Loss-Subsampling darf keine notwendigen Gedächtnisupdates entfernen.

### 5. Trainings- und Live-Features unterscheiden sich und werden abgeschnitten

`dataset4.py` verwendet standardmäßig E=64, Edges=128, Candidates=48,
Pairs=64. Rust erlaubt E=256, Edges=256, Candidates=64, Pairs=128.
Ein außerhalb des Kandidatenlimits liegendes SingleChoice-Label wird
stillschweigend auf den letzten verbleibenden Kandidaten geändert.

Im untersuchten RL-Trainingssplit:

- 17.488/61.440 Frames überschreiten E=64;
- 15 überschreiten 48 Kandidaten;
- mindestens ein SingleChoice-Label wird dadurch umetikettiert.

Außerdem lädt Python die DSL-Daten nur nach CardIndex aus einer
Front-Face-Tabelle. Rust `entity_dsl_summary` berücksichtigt aktive Faces
und Registry-Tokens. Rückseiten und Token erhalten somit unterschiedliche
Eingaben im Training und beim Spielen.

Repariere die gemeinsame Materialisierung. Keine stille Kürzung oder
Labeländerung: dynamische Buckets, vollständige relevante Kandidaten und
explizite Overflow-Behandlung. Teste Rust → Dataset → PyTorch → ONNX anhand
derselben realen Zustände, einschließlich Rückseiten, Tokens, Kopien und
großer Commander-Bretter. Beim Filtern von Paaren müssen auch Labels
identisch remappt werden.

### 6. Der ONNX-Denkschritt-Sweep misst derzeit nicht K=2/4/8

`model4.py:403` ff. begrenzt beim Tracing die Schleife auf
`cfg.default_thinking_steps`. `export_onnx4.py` exportiert standardmäßig K=2.
Ein späterer K=8-Input kann keine sechs fehlenden Schleifen hinzufügen.

Am vorhandenen Phase-B-Modell waren für K=2,4,8 sämtliche Ausgaben
exakt identisch; K=1 unterschied sich.

Exportiere einen tatsächlich wiederholbaren Denkblock, eine geeignete
dynamische Schleife oder überprüfte separate Varianten. Prüfe Ausführung
und Parität für jeden K-Wert. Trainiere variable Tiefe kontrolliert.
Mehr Schleifendurchläufe allein beweisen noch keinen Planungsgewinn.

### 7. Die Anti-Cheat-Seedtrennung ist weiterhin verletzt

`eval4.rs:148` leitet den Policy-RNG per XOR vom echten Spielseed ab.
`league.rs` verwendet ebenfalls `mix(seed ^ ...)`.
XOR/Hash ist keine unabhängige Seedquelle.

Verwende unabhängige, dokumentierte Policy-Seeds und ausschließlich
sitzautorisierte Eingaben. Teste Noninterference: gleiche erlaubte
Beobachtungshistorie und gleicher Policy-Seed müssen bei veränderter
verborgener Welt gleiche Policy-Ausgaben ergeben.
Behalte die bereits verbesserte View-only-Loop-Erkennung bei.

## P1: Abdeckung, Training und Nachweise richtigstellen

### 8. „Lossless“ und „0 unencoded“ sind derzeit irreführend

`encode_teacher_action4` in `features4.rs:2519–2531` ersetzt nicht gefundene
SingleChoice-Aktionen durch Kandidat 0. Fehlende Subset-/Pair-Elemente
werden teilweise ignoriert. Die Coverage-Zahl erkennt diese Fehler nicht.

Angriffsziele werden verworfen und beim Decoding heuristisch gewählt.
`ChooseNumber` ist weiterhin auf `min+20` begrenzt.
`Arrange` enthält nur wenige vorgegebene Anordnungen.
Blocker werden nachträglich auf eine Zuordnung pro Blocker reduziert.

Fordere für unterstützte Aktionen einen semantisch korrekten
Encode/Decode-Roundtrip. Nicht unterstützte Entscheidungen ausdrücklich
kennzeichnen und zählen; keine falschen Lehrlabels erzeugen.
Strategische Angriffsziele, Verteilungen und Zahlen müssen entsprechend
unserem Ziel vom Netz auswählbar sein.

### 9. Critic, Training und Evaluation benötigen ehrliche Verträge

`train_rl4.py` bezeichnet `outcome−V` als GAE; gamma/lambda werden dabei
nicht verwendet. Implementiere GAE mit korrekten Zeit-/Terminalgrenzen
oder dokumentiere bewusst Monte-Carlo-Advantages.

Die neuen 3x-Gewichte für `single_idx>0` gelten für sämtliche
SingleChoice-Familien, obwohl Index 0 nicht überall Pass bedeutet.
Prüfe familienbezogene Gewichtung als Ablation nach den P0-Reparaturen;
sie ist kein Ersatz für korrekte Features und Labels.

`eval4` erzeugt bei `--no-baseline` trotzdem eine künstliche Baseline und
`duplicate_delta`. Diese Werte müssen als „nicht gemessen“ erscheinen.
Caps dürfen keine regulären Unentschieden vortäuschen.

Trenne RL-Rollout-Seeds/Decks von finaler Evaluation: derzeit verwenden
Aufrufe ohne explizite Änderung wieder dieselben Standardseeds und
deterministisch generierten Deckpools. Verwende Sitz-/Deckrotationen,
eingefrorene Testpools und geeignete Konfidenzintervalle.
Unterscheide „DSL bereits gesehen, Gameplay neu“ von vollständig
unbekannten Kartenstrukturen; Phase A liest die Vokabeltabelle mit.

Die damaligen Rolloutberichte unter `/home/ace/baylee-data/v4/eval/`
zeigten gegen `steady` bei Temperatur 0,75: 3/32 Siege im Duell, 0/16
im 2v2 und 0/16 im Vierer-FFA. Das sind kleine Pilotmessungen,
kein Nachweis erfahrener Spielstärke. Gute Top-1-Accuracy und
`refused=0` beweisen weder korrekte Planung noch vollständige Abdeckung.

### 10. Die Crash-Infrastruktur ist erst ein Anfang

`last_attempt` im RAM und `catch_unwind` helfen bei Rust-Unwinds.
Sie schützen nicht vor Abort, Segfault, OOM oder einem hängenden
Engineaufruf. `Session::new_recorded` liegt außerhalb des Catchers.
Das „self-contained“ Fault-Bundle enthält lediglich die letzten 32
Recordzeilen; Header/Setup fehlen regelmäßig.

Baue wie vereinbart Prozessisolation, Supervisor, Vorabprotokollierung
der versuchten Aktion, vollständige reproduzierbare Referenzen,
Timeout-/Ressourcenbehandlung und Quarantäne.
Validiere gezielt Constructor-Panic, Apply-Panic, Abort und Hang.
Reparaturen brauchen Repro, Regressionstest und die Repo-Prüfungen.

Die damaligen 680 Lehrspiele enthalten acht Answer-Caps und acht
Fault-Bundles, aber keine gemeldeten Panics. Berichte diese Kategorien
getrennt; ein Cap beweist für sich noch keinen Enginebug.

### 11. Architekturumfang und Reproduzierbarkeit offen ausweisen

Der aktuelle 160-Float-DSL-Summary plus acht Ability-Slots ersetzt noch
nicht den vereinbarten strukturellen Encoder. Prüfe verlorene Reihenfolge,
Filter, Kostenunterschiede, Aktivierungsbedingungen, gewährte Fähigkeiten
und Kartenidentitäten im eigenen Deck. Ein 24-Werte-Decksummary reicht
nicht, um konkrete noch verfügbare Komboteile zu kennen.

Persistentes Beobachtungsgedächtnis, temporäres Denken und optionaler
Planstatus müssen getrennte Verträge erhalten. Pondering, überprüfbare
Planrevision und die sichere Hybrid-Suche sind noch offene Arbeiten.
Nenne den K-Sweep nicht die erledigte Suchphase T6 aus der Übergabe.

`run_v4_pipeline.sh` löscht wiederholt alte Datensätze und Checkpoints.
Nutze eindeutige Run-Verzeichnisse und Manifeste. Sichere Code-/Schema-/
Daten-/Modellhashes, Seeds und vollständigen Trainingszustand inklusive
Optimizer/RNG für echte Wiederaufnahme. Bestehende Modelle nur als
Pilotartefakte kennzeichnen, nicht automatisch promoten.

## Zusatzauftrag: Viele vielfältige reale Web-Decks für das Training

### 12. Decksammlung tatsächlich importieren und in die Spiele integrieren

Baue neben den vorhandenen Precons, Hausdecks und prozedural generierten
Decks eine reproduzierbare Sammlung realer Web-Decklisten auf. Recherchiere
geeignete Quellen und führe den Import tatsächlich durch; liefere nicht nur
eine Liste möglicher Websites. Möglichst viele **unterschiedliche, valide,
vollständig spielbare Decks** sind gewünscht. Die Zahl heruntergeladener
Dokumente allein ist kein Erfolgskriterium.

Ein breiter Pool soll unterschiedliche Farben/Farbkombinationen, Commander,
Manakurven, Spielgeschwindigkeiten, Schwierigkeitsgrade und Strategien
abdecken: etwa Aggro, Tempo, Midrange, Control, Ramp, Combo, Reanimator,
Graveyard, Sacrifice/Aristocrats, Tokens, Blink/ETB, Spellslinger, Voltron,
Tribal/Kindred, Artefakte, Verzauberungen, Länder und Counter-Synergien.
Das sind Such- und Auswertungsmerkmale, keine fest vorgegebenen Spielpläne
für das Netz. Gemischte Strategien dürfen mehrere Merkmale tragen.

Suche auch nach Budget-, älteren und weniger populären Listen. Eine Sammlung
fast identischer Top-Meta-Decks oder Commander mit jeweils einem Kartentausch
erfüllt den Diversitätsauftrag nicht. Realistische stärkere und schwächere
Decks sind beide nützlich; ihre Stärke und die Stärke des Piloten sind zu
trennen. Sammle Decks für die tatsächlich unterstützten Spielregeln und
teste passende Listen in Duellen, Teams und Mehrspielertischen bis acht
Spieler. Eine Deckliste macht eine spezielle Tischvariante noch nicht zu
einem implementierten Format.

### 13. Bestehende Importpfade und zulässige Quellen verwenden

Lies vor der Implementierung:

- [Deckformat und Parser](../deck-format.md).
- [Precons, bestehender MTGJSON-Import und Community-Recherche](../precons.md).
- [Formate und bisher tatsächlich implementierte Validierung](../banlists-and-formats.md).
- [Rechtliche Vorgaben und Herkunftsnachweise](../legal.md), insbesondere
  Guardrails und die Decklisten-Regelung.
- `crates/baylee-deckio/`, `crates/baylee-core/src/deckrow.rs`,
  `xtask/src/precons.rs`, `crates/baylee-train/src/working.rs`,
  `housedeck.rs` und `deckgen.rs`.

Nutze die vorhandenen Parser und die Kartenidentitätsauflösung; baue keinen
zweiten, abweichenden Deckparser. Der Web-Collector gehört in ein Offline-
Importwerkzeug, nicht in den Gateway oder die reine `baylee-deckio`-Bibliothek.

Prüfe die aktuellen Bedingungen jeder Quelle vor dem Download. Die
Community-Recherche in `docs/precons.md` stammt vom 30. September 2026
und ist keine aktuelle Freigabe sämtlicher Anbieter. Bevorzuge dokumentierte
Bulk-Downloads, freigegebene Datensätze und erlaubte Export-/API-Wege;
protokolliere Quelle, Abrufdatum, Nutzungsgrundlage und erforderliche Attribution.
Nutze Caches, begrenzte Anfrageraten, Backoff und einen wiederaufnehmbaren Import.

Prüfe zuerst den vorhandenen MTGJSON-Archivpfad und passende freigegebene
Deckdatensätze. 17Lands ist laut bestehender Repo-Recherche eine mögliche
Bulk-Quelle für Limited; prüfe konkret, ob der gewählte Dump vollständige
Decks enthält. Draftpicks und Kartennutzungsstatistiken sind nicht automatisch
die tatsächlich gespielten 40-Karten-Listen. Limited nur einbeziehen, wenn
die passende Deckform validiert und spielbar ist; keine 40-Karten-Liste als
fehlerhaftes 60-Karten-Constructed behandeln.

Moxfield ist im aktuellen Repo als Textimport, nicht als automatischer Fetch
vorgesehen. Bestehende Beschränkungen und Bot-Schutz nicht umgehen. Bei einer
ungeeigneten oder nicht freigegebenen Quelle mit anderen zulässigen Quellen
weiterarbeiten. Keine automatische Kontaktaufnahme zu Betreibern. Dokumentiere
offene Quellenfragen, ohne den gesamten Import auf eine einzige Quelle warten
zu lassen. Öffentliche Erreichbarkeit allein ist keine bestätigte Nutzungsfreigabe.

Importiere die benötigten Deckdaten und Herkunftsnachweise. Keine Deckprimer,
Artikel, Kartenbilder, fremden Implementierungen oder personenbezogenen
Spielerprofile als beiläufigen Trainingskorpus sammeln. Decklisten werden
als Spielsetups genutzt; sie sind keine aufgezeichneten Expertenentscheidungen.

### 14. Nur vollständig unterstützte, korrekt rekonstruierte Decks zulassen

Bestimme den verfügbaren Kartenpool aus dem aktuellen Checkout, einschließlich
`Coverage::Implemented` und der vorhandenen `Working`-Prüfung. Alte Zahlen aus
der Übergabe oder `STATUS.tsv` nicht ungeprüft übernehmen. Die Referenz in einem
Test ist ein Aufnahmefilter, kein Beweis, dass jede Interaktion korrekt läuft;
die Crash-/Quarantäne-Pipeline bleibt deshalb erforderlich.

Löse Karten auf kanonische Identitäten auf, vorzugsweise Oracle-ID bzw. die
vorhandene Ledger-Zuordnung. Behandle alternative Schreibweisen, Split-/MDFC-
Namen, Commander/Partner/Background, Companion, Sideboard und Maybeboard
korrekt. Ein Sideboard erhöht nicht die Größe des Main-Decks. Token sind
keine normalen Deckslots; prüfe zusätzlich, ob benötigte Token- und andere
von Karten erzeugte Regeln von der Engine unterstützt werden.

Ein Deck kommt nur dann in den aktiven Trainingspool, wenn alle im jeweiligen
Spielmodus verwendeten Karten vollständig unterstützt und seine Deckform
gültig sind. Berichte separat, ob auch das Sideboard unterstützt wird und
ob Sideboarding überhaupt Teil des trainierten Modus ist.

Prüfe Größe, Kopienlimits, Commander-Eignung, Farbidentität und tatsächlich
verfügbare Formatregeln. Die aktuelle Codebase hat nicht automatisch sämtliche
offiziellen Banlists implementiert. Unterscheide Engine-Spielbarkeit,
Deckform-Gültigkeit und datierte offizielle Formatlegalität; behaupte keine
Legalität, die der Validator nicht prüft. Historische und Freeform-Listen
gegebenenfalls ausdrücklich als solche führen.

Fehlende oder defekte Karten **nicht stillschweigend durch Basics ersetzen**.
Bewahre solche Listen außerhalb des aktiven Trainingspools mit konkretem
Ablehnungsgrund auf. Falls zusätzlich angepasste Varianten sinnvoll sind,
erzeuge sie als klar bezeichnete Ableitungen mit Originalreferenz und
Änderungsliste; prüfe erneut Spielplan, Manabasis, Legalität und Kartenabdeckung.
Sie zählen nicht als unveränderte importierte Originaldecks. Neue Karten zu
implementieren ist keine Voraussetzung für den ersten nutzbaren Importpool.

### 15. Deduplizierung, faire Splits und messbare Vielfalt

Führe ein maschinenlesbares Manifest mit mindestens folgenden Angaben:

- Quellenreferenz, Abruf-/Quelldatum und Importerversion;
- kanonische Kartenzählungen pro Zone, Commander und deterministischer Deckhash;
- Original-/abgeleitet-Status, gegebenenfalls konkrete Anpassungen;
- Deckform, Strategie-/Farbmerkmale und Unsicherheit geschätzter Merkmale;
- Pool-/Engine-/Validierungsversion und Aufnahme-/Ablehnungsgründe;
- Duplikat-/Ähnlichkeitsgruppe und fest zugewiesener Dataset-Split.

Dedupliziere nach Kartenmultiset und relevanten Zonen, unabhängig von Name,
Quelle, Reihenfolge, Sprache, Druckversion oder Foil-Markierung. Gruppiere
auch nahe Varianten, damit ein Deck und sein fast identischer Abkömmling
nicht auf Training und abschließenden Test verteilt werden. Erhalte nötige
Provenienz, ohne unnötige Autor-/Spielerprofile zu speichern.

Lege Train/Validation/Test **vor** gameplaybasiertem Training fest.
Alle Perspektiven und Spiele einer vorgesehenen Testgruppe bleiben aus dem
Training. Die bekannten 15 Eval-Decks sind kein neuer unangetasteter Testpool;
schütze zusätzliche, eingefrorene Testdecks. Prüfe auch die Überschneidung
mit bisherigen Precons, Hausdecks und Sammlungen.

Halte drei unterschiedliche Tests auseinander: neue Deckkombinationen mit
bekannten Karten; gameplay-unbekannte Karten bei bereits bekannten DSL-
Strukturen; vollständig zurückgehaltene Kartenstrukturen. Ein Webimport
darf die jeweilige Holdout-Definition nicht versehentlich aufheben. Bei
Änderungen am Split muss die Trainings-/Modellabstammung nachvollziehbar sein.

### 16. Breite Sammlung in tatsächliche Trainingsabdeckung übersetzen

Beginne mit einem kleinen, überprüften Importpilot über mehrere Strategien
und Deckformen. Erweitere danach systematisch um weitere zulässige Quellen
und neue Deckgruppen; keine künstliche Beschränkung auf nur 15 oder 50 Listen.
Als Planungsgröße sind Hunderte bis Tausende unterschiedlicher spielbarer
Decks interessant, **sofern** Quellen und implementierter Pool das hergeben.
Erfinde keine Deckzahlen und ersetze fehlende Vielfalt nicht durch triviale
Duplikate. Berichte die tatsächlich erreichbare Größe und verbleibende Lücken.

Integriere den Pool in Teacher-Selfplay und später RL/League. Verwende die
faire Haus-KI und die korrigierte Beobachtungs-/Provenienzpipeline.
Kombiniere reale Decks mit den bestehenden Precons, Hausdecks, gezielten
Taktikfällen und guten prozeduralen Varianten. Leite die Mischung aus
Abdeckung und Messungen ab; ein großer Importordner allein trainiert nichts.

Stratifiziere nach Strategie, Farbe, Commander, Tischkonstellation und
relevanten Mechaniken. Begrenze die Dominanz populärer Archetypen und
rotiere Sitzpositionen, Startspieler, Gegner und Teams. Lange Spiele dürfen
nicht allein durch ihre vielen Prioritätsfragen das Training dominieren.
Respektiere dabei den vollständigen Gedächtnisstrom; Loss-Gewichtung und
Beobachtungssequenz sind getrennte Entscheidungen.

Decklisten liefern Spielsetups und später Karten-/Synergiekontext für
Experte 2. Sie liefern keine Wahrheit über den besten Zug. Gewinnerlabels
entstehen aus gesunden Spielen; gute Alternativen aus validierten Such-/
Lehrerprozessen. Webseiten-Popularität, Nutzerbewertungen und angegebene
Commander-Power-Level sind keine verlässlichen Ground-Truth-Labels.
Formatlegalität bleibt deterministischer Code; Bracket-Angaben benötigen
Version, Herkunft und eine ehrliche Unsicherheitsbehandlung.

Liefere zum Import ein reproduzierbares Kommando und einen Bericht:
Quellen, gelesene/einzigartige/abgelehnte/aufgenommene Decks, Duplikate,
Ablehnungsgründe, Farben, Strategien, Deckformen, Karten-/Mechanikabdeckung,
Splitüberschneidungen sowie tatsächlich gespielte gesunde Spiele pro Gruppe.
Fehlerhafte Spiele werden mit vollständiger Herkunft quarantänisiert.
Speichere große Rohdaten in einem vorgesehenen Cache/Datenverzeichnis;
versioniere Code, Manifeste und geeignete kleine Fixtures gemäß Repo-Konventionen.

## Gewünschtes Vorgehen und Ergebnis

1. Bestätige oder widerlege jeden Reviewbefund anhand des aktuellen Codes.
2. Zuerst kleine Regressionstests für die P0-Verträge, dann Reparaturen.
   Quellenrecherche und Deckimport-Vorbereitung können unabhängig davon
   fortschreiten; größere Trainingsläufe warten auf korrekte Verträge.
3. Danach ein kleiner vollständiger Durchlauf: autorisierte Beobachtung →
   Collector mit Behavior-Trace → Dataset → rekurrentes Update → Export →
   Rust-Spielbetrieb. Nutze dabei auch erste validierte importierte Decks.
4. Belege funktionierenden Value, Samplerkorrektheit, Ratio-Identität,
   Gedächtnisparität, reale K-Ausführung und Feature-/Aktionsroundtrips.
5. Erst danach neue Pilottrainings, eine breitere Decksammlung und größere
   Läufe. Bewahre gesunde Rohrecords; regeneriere fehlerhafte abgeleitete Daten.
6. Berichte implementiert/getestet/gemessen/offen getrennt und korrigiere
   überzogene Kommentare oder Statusaussagen. Ordne die Arbeit in die
   tatsächlichen T0–T7-Phasen der Übergabe ein.
7. Führe die passenden Tests und vor einem Push die vorgeschriebenen lokalen
   Gates aus. Testclients, die du startest, anschließend schließen und ihren
   Exit prüfen. Keine fremden laufenden Arbeiten oder Artefakte überschreiben.

Bitte löse die Probleme konkret und führe den autorisierten Deckimport aus.
Weitere Loss-Gewichtungen, mehr Decks oder eine längere Trainingsdauer können
die beschriebenen Vertragsfehler nicht reparieren. Nach deren Korrektur soll
die zusätzliche Vielfalt dem Netz helfen, über einzelne Decks und einfache
Haus-KI-Muster hinaus zu generalisieren.
