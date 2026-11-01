# Playernet v4: mehr Tiefe, danach Lernen aus eigenen Partien

Der Besitzer wünscht zuerst mehr Layer, anschließend eine erneute Übernahme
bewährter Ideen aus v3: Bewertung, Spiele gegen die Haus-KIs und später
Selbstspiel. Zugleich sollen Trainingsspiele die Haus-KI messbar verbessern.

## Erste Änderung: acht auf zwölf Transformer-Schichten

`deepen4.py` vergrößert nur den räumlichen Trunk. Breite 640, zehn Heads,
FFN 2560, Kartenencoder, Köpfe, Gedächtnis H mit neun Slots und Denkblock
bleiben kompatibel. Neue Schichten liegen an den Indizes 2, 5, 8 und 11;
die alten acht Schichten werden in gleicher Reihenfolge übernommen.
Nullinitialisierte Ausgangsprojektionen der neuen Attention-/FFN-Zweige
machen deren Residualblöcke zunächst zur Identität. Die neuen Blöcke sind
trainierbar; weder das bestehende Wissen noch die Funktionsweise der
persistenten Rückkopplung werden absichtlich verworfen.

Die Idee folgt funktionserhaltendem Netz-Wachstum
([Net2Net, Chen/Goodfellow/Shlens](https://arxiv.org/abs/1511.05641)); die
konkrete Initialisierung nutzt die vorhandenen Pre-Norm-Residualblöcke.
Das ist kein Beleg für eine Stärkeverbesserung. Ein eigener Quellstand,
neuer Optimierer und frische Gedächtnis-/Samplerbahnen kennzeichnen den
Architekturversuch. Alte Checkpoints und gefrorene Experimente bleiben erhalten.
CPU-Tests prüfen den unveränderten Gedächtnisverlauf und alle Aktions-/Value-
Ausgaben bei K=0/1/4/8, unabhängige Gewichte, trainierbare neue Zweige und
Abweisung inkompatibler Checkpoints. CUDA-Speicher und ONNX werden separat
am echten Modell geprüft, bevor längeres Lernen freigegeben wird.

Als Start dient das Referenzmodell vor dem zuletzt schwächeren BC-Kandidaten.
Ein kurzer kontrollierter Versuch vergleicht zwölf und acht Schichten auf
demselben vollständig gemischten 240-Spiele-Bestand, mit identischem
Sampler, B/U, Anzahl Updates und Validierung. Die gleichbleibende BC-Aufgabe
dient hier der Isolation des Tiefeneffekts; sie ist noch keine Umsetzung der
anschließenden Liga. Ganze Gedächtnisverläufe bleiben chronologisch erhalten.
Die Auswertung prüft zusätzliche Denkschritte und echte gepaarte Partien;
kein Modell wird allein wegen niedrigerem Loss oder mehr Parametern übernommen.

### Umsetzung und Startnachweis

Experiment: `/home/ace/baylee-data/v4/experiments/depth12-20261024`.
Das erzeugte Modell hat **74.520.456 Parameter**, die Quelle 54.826.376.
Die Übertragung ist in `initial-12/depth-transfer.json` samt Quellhash und
Schichtzuordnung dokumentiert. GPU-Vorprüfung und Export verwenden die
Runtime `3c87d5807`; die Trainings-Runtime `runtime-v2` ist auf `d4e1534c0`
eingefroren. Nur Transferhelfer und zugehöriger Test unterscheiden sich:
Die unabhängige Sol-Prüfung fand, dass `torch.manual_seed` neben der CPU
auch Beschleuniger neu seedet. Der Helfer seedet jetzt ausschließlich den
CPU-Generator; der Aufrufer behält alle Zufallszustände. Modell-, Trainer-,
Encoder- und Evaluatorcode sind gegenüber der Vorprüfung identisch.
Die Engine stammt unverändert aus dem vorherigen Lauf (`8bb208a49`).

Elf fokussierte CPU-Tests bestanden (vier Tiefen-, sechs Gedächtnis- und ein
Thinking-Cache-Test). Die unabhängige Prüfung bestätigt außerdem beim
8→12-Transfer identische Ausgaben und Gradienten der übernommenen Parameter.
Am echten Modell wurden 24 Frames aus großen Spielsituationen mit demselben
Gedächtnis verglichen: maximale FP32-Abweichung **0**.
159 reale U=32-Segmente aus langen Partien lieferten die Stichprobe für drei
CUDA-Stressbatches mit B=8, bis zu 354 Entitäten und 350 Kandidaten.
Training mit K-Curriculum und allen zusätzlichen Layern blieb endlich;
Peak 8,18 GiB allokiert, 8,84 GiB reserviert, null Allocator-Retries.
Das ist keine Obergrenze für alle möglichen Spielsituationen. Die als
`largest_pairs` bezeichnete Stichprobe enthält hier nur den Paddingplatz;
sie belegt keine besondere Belastung durch große Blockerzuordnungen.
Alle vier neuen Blöcke hatten nach den Probeupdates endliche, von null
verschiedene Ausgangsgewichte. Der Probeoptimierer wird nicht übernommen.
ONNX K=0/1/2/4/8: 79 geprüfte Frames, maximaler Fehler 3,18e-5.

Sol bereitete parallel den gemeinsamen Cache aus den sieben geprüften
Paketen vor: 240 Spiele, 728 Sitzverläufe, 3.753.658 Frames, 21.541.330.775
Cachebytes. Die Rohshards sind unveränderte Hardlinks; Metadaten, Katalog,
disjunkte Spielabdeckung, Cursor und Endmarkierungen sind geprüft.
`full-cache-audit.json` hält Herkunft und Prüfsummen fest. Der vollständige
Bestand wird gemeinsam gesampelt, statt nacheinander separat nachtrainierte
Pakete mit wechselndem Formatmix zu verwenden.

Controller-PID beim Start: **8539**, `run.py`. Beide Varianten bekommen
B=8/U=32, LR=1e-5, Seed 31423, BF16, dieselben ersten 1.000 Samplerbatches
und jeweils frischen Adam. Dieser begrenzte Ausschnitt umfasst laut
`prefix-exposure.json` **255.495 Frames, 30.814 Entscheidungsframes aus
33 Spielen / 40 Sitzverläufen**; er ist keine vollständige Epoche über alle
240 Spiele. Der 12-Layer-Trainer hat die Gewichte geladen und die vollständige
Ausgangsvalidierung auf CUDA gestartet. Initial- und Endvalidierung behalten
B=8 und die gleichen 475.248 Frames. Checkpoints entstehen alle 100 Updates.
Anschließend läuft automatisch die Acht-Layer-Kontrolle, dann Export und
128 Duell-/64 Commander-Fälle je Variante, plus K=4 für zwölf Layer.
Die Vergleiche benutzen dieselben bereits untersuchten Development-Seeds
wie der aktuelle v3-Vergleich; sie sind kein unberührter Abschlusstest.
Der Pilot verändert zunächst nur Tiefe und ist noch kein Training aus
eigenen Ligapartien. Ergebnisse und eine Stärkeverbesserung stehen aus.

### Automatische Wiedervorlage (20:02 CEST)

Auf Wunsch des Besitzers überwacht der aktivierte Benutzertimer
`baylee-training-watch@depth12-20261024.timer` den gesamten Controller alle
30 Sekunden. Er erwartet die Phase `depth-pilot-and-comparisons-complete`
und alle zwölf gepaarten JSON-Ergebnisdateien. Fehler und ein bestätigtes
Controllerende ohne Abschluss lösen ebenfalls eine Nachricht an die bestehende
Codex-Sitzung aus. Die Prozessidentität enthält Boot-ID und Startticks;
übernommene PIDs nach einem Neustart zählen nicht als laufender Versuch.

Konfiguration und Zustellstatus liegen im Experiment unter `watchdog/`.
Die installierte eigenständige Datei
`~/.local/lib/baylee-training-watch/watch_run4.py` hat SHA-256
`1531e20d805589f9b35ccdcbb8fac87833c5f3b60bce9508b24cdb20ff22feaf`.
Zwölf isolierte Tests und die systemd-Unit-Prüfung bestanden. Der erste echte
Check endete erfolgreich mit `running`, 48 ms CPU und 10,2 MiB Spitzenverbrauch.
Ein Benutzerdienst konnte den vorhandenen Codex-Daemon erreichen; eine
Testnachricht an genau diese Sitzungs-UUID wurde von `codex queue` angenommen.
Die Testnachricht hat anschließend genau diese Sitzung automatisch im Folgeturn
aktiviert; ihre Zustellung und Verarbeitung sind damit bestätigt. Der Timer war
bei der erneuten Prüfung aktiv, ohne Status-Lesefehler oder verschwundenen
Controller. Dieser Selbsttest hat keinen weiteren Trainingslauf gestartet.

Normale Checks verursachen keinen Modellaufruf; ein erfolgreicher Queue-Beleg
unterdrückt weitere Zustellungen. Bei nicht erreichbarem Codex folgt nach
60 Sekunden ein erneuter Versuch. Ein Absturz direkt nach Annahme durch die
Queue kann dieselbe Ereignis-ID erneut zustellen; der Folgeagent muss sie
deduplizieren. Kein Linger aktiviert: Nach einem Reboot wird der Timer mit
dem Benutzer-Manager erneut aktiv, und der Agent prüft den tatsächlichen
Trainingsstand vor einer Fortsetzung. Laufender Trainer und eingefrorene
Runtimes wurden durch die Einrichtung nicht verändert.

### CUDA-OOM erkannt und Fortsetzung vorbereitet (20:12–20:19 CEST)

Der Wecker meldete Ereignis `6f5dff3f629ff8b46bf37516`: Der Zwölf-Layer-
Trainer scheiterte im Backward nach dem letzten Logeintrag bei Schritt 925
an einer weiteren 72-MiB-Allokation. Der Job bleibt als `quarantined` samt
Fehlerlog erhalten. Zu diesem Zeitpunkt waren laut Fehlermeldung 9,99 GiB
allokiert und weitere 1,23 GiB reserviert, aber unbenutzt. Der atomare
Checkpoint **900** enthält passende Gewichte, Adam, RNG, B=8/U=32,
Sampler-Seed 31423, Cursor 900 und H. Unveränderliche Hardlinks von `last.pt`
und dem bei Schritt 0 ausgewählten `net.pt` liegen unter `recovery-oom/`.
SHA-256 von `last.pt`:
`07cdb83ae31b691c269d0530d0834104514e5e80720e172abe487402b78a8fc8`.

Zwei Sol-Prüfungen untersuchten parallel Speicherhaltung und tatsächliche
Samplerformen. `shape-audit.json` bestätigt: Im ersten 100-Batch-Abschnitt
liegt die mittlere gepaddete Entitätszahl bei 64,65; bei Schritten 926–1.000
bei 417,8, maximal **430**. Haupttreiber ist die lange Partie 239, Sitz 5.
Die frühe Stichprobe mit maximal 354 Entitäten hatte diese Last nicht erfasst.
Der Trainer loggt einen kumulativen Höchststand in dezimalen GB; dessen Anstieg
belegt für sich kein Speicherleck. Die Codeprüfung fand keine kumulative
Graph-Haltung: Statistiken sind detached, H wird zwischen Unrolls getrennt,
Struktur-/Thinking-Caches sind begrenzt und CUDA-Graph-Capture ist ausgeschaltet.

Ein separater GPU-Test spielte mit einer temporären Kopie des Zustands exakt
die nächsten **100 Updates (901–1.000)** einschließlich passender Samplerbahnen
und H durch. Einzige Ausführungsänderung:
`PYTORCH_ALLOC_CONF=expandable_segments:True`. Diese experimentelle
[PyTorch-Speicheroption](https://docs.pytorch.org/docs/2.14/notes/cuda.html#optimizing-memory-usage-with-pytorch-alloc-conf)
soll unbrauchbare Reststücke bei wechselnden Allokationsgrößen reduzieren.
Ergebnis hier: **10,674 GiB** maximal allokiert, **10,844 GiB** maximal
reserviert, **null Allokations-Retries**; nach jedem Update 1,142–1,150 GiB
belegt. Alle 100 Updates liefen durch. Probegewichte wurden verworfen.
Das belegt Eignung für diesen begrenzten Abschnitt, keine Speicherobergrenze
für den vollständigen Bestand oder beliebig große Spielsituationen.

`resume_after_oom.py`, Controller **12863**, setzt seit 20:19 CEST den echten
Lauf von Checkpoint 900 fort. Er überprüft dessen Hash, den bestandenen Test
und die unveränderte `runtime-v2`. B/U, Curriculum, Modell, Optimierer,
Datenreihenfolge und Lernrate bleiben erhalten; derselbe Allocator gilt danach
auch für die Acht-Layer-Kontrolle. `resume-plan.json` dokumentiert die Umgebung
zusätzlich, weil der eingefrorene Job-Runner diese neue Variable noch nicht
in seine Environment-Auswahl aufnimmt. Der neue Job heißt
`train-depth12-resume-oom`; ursprüngliche Fehlversuche bleiben erhalten.
Alle Exporte und gepaarten Vergleiche bleiben Teil desselben Controllers.

Der ursprüngliche Wecker samt Zustellbeleg wurde nach
`recovery-oom/first-watchdog/` archiviert; der Timer überwacht jetzt die
verifizierte neue Prozessidentität. `handled-wake.json` dokumentiert die
Bearbeitung des alten Ereignisses. Vor einem längeren Lauf müssen tatsächliche
späte Formen und größere Blocker-/Kandidatenmengen erneut geprüft werden;
eine kleine frühe Stichprobe genügt nicht.

Nebenfund: `bench_training4.py` verwendete beim Fortsetzen immer Seed 42,
obwohl H aus einem anderen Sampler stammen konnte. Commit `26694ed87`
stellt Seed, Version, Epoche und Cursor aus dem Checkpoint wieder her und
weist inkompatible Fortsetzungen ab. Drei CPU-Tests bestanden. Der echte
Trainer hatte diese Prüfung bereits; der GPU-Test oben verwendete ausdrücklich
den gespeicherten Seed 31423. Eingefrorene Trainer wurden nicht verändert.

## Was sich aus dem alten Modell tatsächlich übertragen lässt

Der geprüfte `model3.py`-/`netplay3.rs`-Pfad besitzt keine neuronale
`h_out -> h_in`-Rückkopplung. Diese ist im aktuellen v4 bereits vorhanden:
Beobachtungen aktualisieren H, spätere Entscheidungen erhalten diesen Zustand.
Die historische Erinnerung des Besitzers kann sich auf einen anderen Entwurf
beziehen; sie ist kein Beleg dafür, dass v4 seinen Speicher erst bekommen muss.
Zusätzliche räumliche Layer vergrößern nicht die TBPTT-Reichweite U.

Das separate frühere `ValueNet` bewertete Stellungen unter Haus-KI-
Fortsetzungen, nicht direkt jede mögliche Alternativaktion. v3 `train_rl.py`
verwendet den eigenen trainierten Value-Head für GAE und PPO auf den
Entscheidungen des Lerners aus Ligapartien gegen Hausprofile, eingefrorene
Netze und sich selbst. Der letzte v4-Lauf lernte dagegen nur aufgezeichnete
Hausaktionen. Diese Trainingsregime dürfen beim Interpretieren der besseren
v3-Spielergebnisse nicht gleichgesetzt werden. Auch GAE ordnet einem Zug
keine beweisbar kausale Güte zu; der Critic kann sich irren.

Nach dem Tiefenpilot: Fehlerzustände aus eigenen v4-Partien sammeln,
Lehrer-/Schülerherkunft trennen und echte terminale Outcomes bewahren.
Den Bewertungskopf auf diesen Zuständen und nach Format/Spielphase prüfen;
ein separater Rater wird erst übernommen, wenn er die Aktionswahl oder das
Training messbar verbessert. Danach kleine on-policy Liga-Iterationen mit
verifizierten Verhaltenswahrscheinlichkeiten, wiederkehrenden Hausgegnern
und historischen Modellen; Selbstspiel ergänzt diese Gegner, es ersetzt
sie nicht vollständig. Ein bloßes weiteres unverändertes BC-Paket erfüllt
diese Stufe nicht.

## Trainingsspiele verbessern auch die Haus-KI

Jeder belastbare Fehlerfund erhält Engine-/KI-Revision, Decks, Seed,
betroffenen Sitz, Profil, Aufzeichnungs-/Replaypfad und Entscheidungscursor.
Zuerst unterscheiden wir Regel-/Enginefehler, unzulässige Antworten,
Kodierungs-/Gedächtnisfehler und schwache legale Spielentscheidungen.
Die gespeicherte Spielaufzeichnung ist für Entwickler bestimmt; Policy und
Hausheuristik dürfen weiterhin nur ihre zulässige `PlayerView` sehen.

Aus dem Fund entsteht ein kleiner deterministischer Regressionstest. Eine
Haus-KI-Änderung wird zusätzlich auf anderen Decks, Seeds und Sitzrotationen
gegen ihren eingefrorenen Vorgänger geprüft; ein einzelner gewonnener
Trainingsfall genügt nicht. Detonate (X-Zielwahl) ist ein bereits behobenes
Beispiel dieses Kreislaufs. Auch verlorene Partien liefern brauchbare Fälle;
die Niederlage allein belegt noch keinen Fehler.

Eine laufende Vergleichssuite behält ihre ursprünglichen Gegner und Engine.
Verbesserte Haus-KIs werden als neue Version in die nächste Liga aufgenommen;
neue Lehrerpartien tragen ihre tatsächliche Herkunft. Geänderte Engine-Regeln
erfordern erneut hashgeprüfte Replays. Damit profitieren Netz und Haus-KI
voneinander, ohne dass ein wechselnder Gegner den Fortschritt verfälscht.

## Abschluss: Tiefenpilot und gepaarte Vergleiche (24.10.2026)

Beide Arme haben **1.000 Updates** erreicht; der nach Validierungs-Policy-Loss
gewählte Checkpoint liegt jeweils bei Schritt 1.000. Alle neuen Trainings-,
Export-, Evaluations- und Vergleichsjobs sind abgeschlossen. Der ursprüngliche
CUDA-OOM-Job bleibt unverändert als `quarantined` erhalten; der dokumentierte
Resume ab Schritt 900 lief mit `expandable_segments:True` bis zum Ende.
`~/baylee-data/v4/experiments/depth12-20261024/result.json` enthält die
Quell-/Checkpoint-/Ergebnis-Hashes, vollständigen gepaarten Unsicherheiten,
Validierungswerte und den Expositionsaudit. **Keine Promotion.**

Die folgenden Ergebnisse sind Policy-Ersetzungen gegen dieselben festen
`sharp`-Hausgegner, kein direkter Zweikampf zwischen den Netzen. Die Suite
verwendet bereits betrachtete Entwicklungsfamilien und Seeds. Siege sind
absichtlich durch alle Fälle geteilt; Caps bleiben unbekannte Ausgänge und
werden nicht als Remis umgedeutet. Remis und Caps stehen getrennt daneben.

| Modell | Duell-Siege / 128 | Remis / Caps | Commander-Siege / 64 | Remis / Caps |
| --- | ---: | ---: | ---: | ---: |
| 8 Layer, K=1 | 33 | 0 / 0 | 5 | 7 / 3 |
| 12 Layer, K=1 | 34 | 0 / 0 | 4 | 4 / 4 |
| 12 Layer, K=4 | 35 | 0 / 0 | 3 | 5 / 4 |
| Initiales v4 | 36 | 0 / 0 | 3 | 5 / 1 |
| Historisches v3 (`rl-23`) | 42 | 0 / 0 | 6 | 5 / 0 |
| Haus-Baseline `steady` | 60 | 0 / 0 | 15 | 6 / 0 |

12 gegen 8 Layer verbessert fünf Duellfälle und verschlechtert vier;
119 bleiben gleich. Der gepaarte Utility-Unterschied beträgt **+0,0078125**,
das nach Tabelle/Seed/Decks geclusterte Bootstrap-95-%-Intervall
**[-0,0390625; +0,0546875]**. Commander enthält vier ungeklärte Cap-Paare;
das entsprechende Intervall ist **[-0,1484375; +0,15234375]**.
Damit ist **kein statistisch belastbarer Tiefenvorteil nachgewiesen**.
K=4 gewinnt gegenüber K=1 genau einen zusätzlichen Duellfall, ohne Regression;
das Bootstrap-Intervall [0; +0,0234375] stützt bei diesem einzelnen Fall,
64 Tabellen und wiederholt betrachteter Suite keinen robusten Vorteil.
Commander: ein besserer, ein schlechterer und vier ungeklärte Fälle,
Intervall [-0,19921875; +0,1484375]. Gegen v3 liegen beide Duell-Arme zurück:
12 Layer 13 besser/21 schlechter, Intervall [-0,15625; +0,03125];
8 Layer 12 besser/21 schlechter, Intervall [-0,15625; +0,015625].

Der Value-Head ist weiterhin unzureichend. Validierungs-MSE:
**0,160539 (12)** und **0,161857 (8)** gegenüber **0,121955** für den
formatabhängigen Gleichverteilungsprior und **0,153021** beim initialen Modell.
Die mittleren Vorhersagen fallen auf **0,068471 / 0,069014**, während das
Outcome-Mittel **0,250093** beträgt. Besserer Policy-Validierungs-Loss
(0,285288 / 0,286003 statt initial 0,297216) bedeutet hier weder bessere
Kalibrierung noch nachgewiesen stärkere Spielweise.

`value-prefix-audit.json` zeigt eine wesentliche Einschränkung dieses kurzen
Piloten: Die identischen ersten 1.000 Sampler-Batches (Seed 31423, B=8, U=32)
sehen nur **33/240 Spiele, 40/728 Sitztrajektorien und 255.495 Frames**.
**84,5 %** der Frames sind Verlustlabels; die fünf längsten gesehenen
Verlusttrajektorien liefern **53,0 %** aller Frames. Updates 201–300 enthalten
kein positives Outcome. Das gewichtete Outcome-Mittel liegt bei **0,151741**
gegenüber **0,229019** im vollständigen Trainingsbestand; die tatsächliche
Normierung nach gültigen Bahnen und Zeitschritt ergibt nahezu identisch
**0,151751**. Padding ist damit keine wesentliche Erklärung.
Acht-Sitz-Partien liefern **69,4 %** des Präfixes, Duelle nur **4,7 %**;
im Gesamtbestand sind es 42,5 % beziehungsweise 7,0 %. Das belegt starken
Verlust-/Formatdruck aus wenigen persistenten Bahnen, erklärt aber nicht
allein kausal die noch stärkere Value-Untervorhersage.

Der nächste getrennte Versuch liegt unter
`~/baylee-data/v4/experiments/critic-own-games-20261024/`: Eigene stochastische
Partien des **nicht promovierten** 12-Layer-Forschungskandidaten werden
gegen `steady` und `sharp` gesammelt (Temperatur 0,6, K=1). Geplant sind
**248 Spiele: 152 Training, 96 Validierung**, getrennte Deck-Splits und sechs
Konfigurationen: Duelle, Commander, Teams sowie FFA mit 3/6/8 Sitzen.
Die Wahl dieses Kandidaten ist keine Behauptung, dass er den Tiefenvergleich
gewonnen habe. Separates Critic-Head-Training bei eingefrorenem Actor und
anschließender Kalibrierungsprüfung folgt erst auf diese Sammlung; es wird
hier noch nicht als laufend oder erfolgreich bezeichnet. Caps und Fehler
bleiben Befunde und erhalten keine erfundenen terminalen Draw-Labels.
