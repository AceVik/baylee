# Ansichten als Änderungen statt als ganzes Brett

Stand 7. November 2026, Entwurf. Noch nichts davon ist gebaut. Der Eigentümer hat entschieden, ans
Protokoll zu gehen; Kompatibilität wird nicht gebraucht, solange niemand baylee.app nutzt.

## Heute

Nach jeder Aktion bekommt jeder Sitz seine ganze Ansicht, als JSON in einem Protobuf-Rahmen. Eine
Ansicht mit 25 Objekten sind rund 10,8 KB, also gut 400 Byte je Objekt, und fast alle Objekte sind
unverändert. Das Schreiben kostet auf dem Server, das Lesen kostet auf dem Client, und beides zahlt
jeder Sitz bei jedem Schritt. `WrittenObjects` spart das Schreiben, nicht die Bytes und nicht das
Lesen.

Bereits gut: Das Gateway liest die Frames nicht, gedruckte Karten kommen einmal als eigener Rahmen
(`game_static_envelope`), und die Engine baut je Sitz eine Ansicht, in der Verdecktes nicht
darstellbar ist.

## Das Ziel

Ein Sitz bekommt einmal einen Stand und danach nur noch, was sich geändert hat, binär.

```text
ViewFrame  = Snapshot { seq, view }
           | Patch { seq, base, upserts, removed, fields, digest }
```

- **Der Diff entsteht aus der fertigen Ansicht je Sitz,** nie aus dem Spielzustand. Was im
  `PlayerView` nicht steht, kann in keinem Patch stehen. Die Regel „Verdecktes ist nicht darstellbar“
  bleibt also ohne neue Prüfung gültig.
- **`diff(alt, neu)` und `apply(alt, patch)` leben in `baylee-view`,** an einer Stelle, für Server,
  Client, LLM-Sitz und Netz-Sitz. Alles darüber arbeitet weiter auf einem ganzen `PlayerView`. Der
  Client und die Sitze ändern sich damit kaum.
- **Objekte sind die Einheit:** ein neues oder geändertes Objekt geht ganz, ein gelöschtes als ID,
  die übrigen Felder des Views einzeln. Tiefer zu diffen lohnt erst nach einer Messung.
- **`digest` über die Ansicht** nach dem Patch: Weicht er ab, verlangt der Client einen Snapshot.
  Ein Fehler wird so zu einem unsichtbaren Neuladen statt zu einem falschen Brett.
- **Snapshot bei Beitritt, Wiederverbinden und Lücke in `seq`.** Zuschauer laufen genauso.
- **Binär mit `postcard`** über die vorhandenen Serde-Typen, für Snapshot, Patch und Aktionen des
  Clients. Protobuf bleibt der Rahmen. `bitcode` wird daneben gemessen; `rkyv` scheidet aus, weil es
  fremde Bytes ohne Kopie liest.
- **Danach entfallen** `WrittenObjects` und das JSON im Spielpfad. JSON bleibt für Werkzeuge,
  Aufzeichnungen und `--verify`, wo es nicht im Spiel läuft.

## Sicherheit

Decoder lesen fremde Bytes, auch in der Engine: Das Gateway reicht Frames ungelesen durch. Darum:
harte Grenzen für Längen und Tiefe vor jeder Belegung, kein `unsafe` im Decoder, Fuzz-Tests für beide
Richtungen, und ein Patch, der auf eine unbekannte ID zeigt, führt zum Snapshot, nicht zur Panik.

## Beweis

- Eigenschaftstest: `apply(alt, diff(alt, neu)) == neu` über aufgezeichnete Partien, je Sitz.
- `engine_perf --verify` vergleicht die Ansicht, die ein Empfänger aus Patches aufbaut, mit der
  ganzen Ansicht, Bit für Bit, auf allen Messlasten.
- Vorher und nachher: Bytes je Partie und Sitz, CPU auf dem Server, CPU auf dem Client.

## Reihenfolge

1. **Messen:** Bytes und Frames je Partie heute.
2. **`diff` und `apply`** mit Eigenschaftstests, ohne Änderung am Draht.
3. **Patches im Spielpfad,** noch als JSON, mit Digest und Snapshot.
4. **Binär** für Snapshot, Patch und Aktionen, dann Vergleich `postcard` und `bitcode`.
5. **`WrittenObjects` und JSON-Pfad entfernen,** `VIEW_VERSION` und `docs/protocol.md` nachziehen.
6. **Sicherheitsprüfung** der neuen Decoder, Fuzz-Lauf, Sichtprüfung im Client.

## Darüber hinaus

Nach den Patches, in dieser Reihenfolge, jeweils nur mit Messung:

1. **Die Engine merkt sich, was sich geändert hat.** Heute baut sie jede Ansicht ganz neu
   (`public_object` sind 23 bis 40 % der Messlasten) und ein Diff müsste sie dann noch vergleichen.
   Über Versionszähler je Objekt werden nur berührte Objekte neu gebaut und der Diff kostet so viel wie
   die Änderung, nicht wie das Brett. Wechselt die Sichtbarkeit eines Objekts für einen Sitz, gilt es
   als berührt. Im Debug-Build vergleicht jeder Schritt mit dem vollen Neubau.
2. **Der Client zeichnet nur, was der Patch nennt.** Die Liste geänderter IDs geht an den Tisch, der
   heute das ganze Brett prüft. Das hilft bei großen Brettern und Stacks am meisten.
3. **Weniger Frames.** Zwischen zwei Fragen an einen Sitz reicht oft ein Patch; Animationen dürfen
   dabei nichts verlieren. Auch `LegalActions` wird gediffed, wenn es einen großen Anteil hat.
4. **Öffentlicher Teil einmal.** Zuschauer sehen alle dasselbe: ein Patch für alle statt einer je
   Zuschauer. Für Spieler nur, wenn bewiesen ist, dass der geteilte Teil nichts Verdecktes trägt.
5. **Felder statt Objekte,** falls Objekte nach allem noch die meisten Bytes sind.

Lockstep, also dass jeder Client selbst simuliert, scheidet aus: Dafür müsste er verdeckte Karten und
den Zufall kennen.

## Entschieden (war offen)

Die Zahlen stehen in `docs/perf-baseline.md`.

- **Aufzeichnungen und Replays speichern keine Patches.** Eine Aufzeichnung hält Eingaben und gar keine
  Ansicht, sie ändert sich nicht. Ein Band hält ganze Ansichten als JSON, denn ein Leser springt zu
  jedem Augenblick, und ein Augenblick darf von keinem anderen abhängen
  (`Session::tell_views_whole`). Ein Band wird nur auf Anfrage gebaut und liegt nirgends.
- **Das Log bleibt, wie es ist.** Es wird nur angehängt und schon je `from` geschickt, also gibt es
  nichts zu diffen. Es war 0,2 bis 1,3 % der Bytes und ist mit Patches 8 bis 12 %. Was sich ändert:
  Ein Rahmen, der nur Log trägt, wiederholt die Ansicht nicht mehr.
- **Der Gewinn ist gemessen.** 14- bis 57-mal weniger Bytes je Ansicht als JSON, 27-mal auf gehosteten
  Partien und 154-mal auf der größten. Ganze Rahmen fallen 8- bis 35-mal, weil anderes bleibt.
- **Neu dazugekommen:** `GameStatic` geht bei jeder neuen Druckversion ganz und ist danach ein Viertel
  dessen, was bleibt. Das gehört in dieselbe Reihe.
