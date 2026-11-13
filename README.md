# Minecraft Builder

Baue **Minecraft-Plugins (Paper)** und **Mods (Fabric)**, indem du Blöcke verbindest – ganz ohne Java-Kenntnisse.
Komplett in **Rust** geschrieben (egui), läuft als einzelne **`.exe` unter Windows**.

![Minecraft Builder](docs/screenshot.png)

## Was kann es?

- **Node-Editor**: Events, Aktionen und Werte als Blöcke auf einer Fläche verbinden (Zoom, Verschieben, Raster, Notiz-Rahmen, Mehrfachauswahl, Kopieren/Einfügen, Rückgängig).
- **Zwei Plattformen** aus demselben Projekt:
  - **Paper-Plugin** (Minecraft 26.x und 1.21.11) – für Server, Spieler brauchen nichts zu installieren.
  - **Fabric-Mod** (Minecraft 26.x) – für Fabric-Server oder Einzelspieler.
- **74 Blöcke** in 10 Kategorien: Events (Betreten, Verlassen, Chat, Block abbauen/setzen, Tod, Schaden, Rechtsklick, Befehle, Timer), Nachrichten (Chat, Titel, Actionbar, Broadcast), Spieler (Teleport, Heilen, Items, Effekte, Spielmodus, Kick …), Welt (Blöcke setzen, Blitz, Explosion, Mobs spawnen, Uhrzeit), Ablauf (Wenn, Zufall, Wiederholen, Für jeden Spieler, Warten), Logik, Mathe, Text und **dauerhaft gespeicherte Daten** (pro Spieler oder global, bleiben nach Neustart erhalten).
- **Ein Klick auf „▶ Bauen“ (F5)** erzeugt die fertige `.jar` – inklusive Gradle-Projekt, das auch in IntelliJ/VS Code weiterbearbeitet werden kann.
- **Live-Java-Vorschau** des erzeugten Codes, **Probleme-Liste** mit Sprung zum Block, Konsole mit Filtern.
- Deutsch und Englisch umschaltbar.

## Bauen der `.exe`

Voraussetzung: [Rust](https://rustup.rs/).

```bash
cargo build --release
# Ergebnis: target/release/minecraft-builder.exe
```

Von Linux aus für Windows:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
cargo build --release --target x86_64-pc-windows-gnu
```

Bei jedem Push baut GitHub Actions automatisch die Windows-`.exe` (Artefakt „minecraft-builder-windows-x64“) und prüft, dass das erzeugte Plugin und der Mod wirklich kompilieren. Bei einem Tag `v*` wird die `.exe` als Release hochgeladen.

Für alte Grafiktreiber gibt es einen OpenGL-Build: `cargo build --release --no-default-features --features glow`.

## Benutzung

1. **Datei → Neues Projekt**: Plugin (Paper) oder Mod (Fabric) und eine Vorlage wählen.
2. **Rechtsklick** oder **Leertaste** auf der Fläche öffnet die Block-Suche. Alternativ aus der Block-Bibliothek (Seitenleiste) ziehen.
3. Jeder Ablauf startet bei einem **Event-Block**. Die **grünen Pfeile** bestimmen die Reihenfolge, die **farbigen Punkte** sind Werte (Spieler, Text, Zahl, Position …).
4. Ein Kabel **ins Leere ziehen** zeigt nur passende Blöcke und verbindet sie automatisch.
5. Nicht verbundene Spieler-Eingänge nehmen automatisch den Spieler aus dem Event.
6. Farbcodes in Texten: `&a` grün, `&c` rot, `&e` gelb, `&l` fett …
7. **▶ Bauen** (F5). Die `.jar` kommt bei Paper nach `plugins/`, bei Fabric nach `mods/` (zusammen mit der Fabric API). Unter „Bauen & Export“ kann ein Test-Server-Ordner eingetragen werden – die `.jar` wird dann automatisch dorthin kopiert.

**Zum Bauen muss Java installiert sein** (Java 21 oder neuer, z. B. von [adoptium.net](https://adoptium.net/)).
Der erste Build lädt Gradle, das passende JDK (25 für Minecraft 26.x) und die Minecraft-Bibliotheken automatisch herunter und dauert daher einige Minuten. Danach geht es schnell.

### Tastenkürzel

| Taste | Aktion |
|---|---|
| Rechtsklick / Leertaste | Block hinzufügen |
| Entf | Auswahl löschen |
| Strg+C / Strg+V / Strg+D | Kopieren / Einfügen / Duplizieren |
| Strg+Z / Strg+Y | Rückgängig / Wiederholen |
| Strg+S / Strg+O / Strg+N | Speichern / Öffnen / Neu |
| F5 | Bauen |
| F | Alles anzeigen |
| Mausrad | Zoomen |
| Ziehen auf leerer Fläche / mittlere Maustaste | Ansicht verschieben |
| Shift + Ziehen | Mehrere Blöcke auswählen |
| Rechtsklick auf Pin | Verbindungen trennen |

## Projektaufbau

| Datei | Inhalt |
|---|---|
| `src/blocks.rs` | Alle Block-Definitionen mit Java-Vorlagen für Paper und Fabric |
| `src/codegen.rs` | Wandelt die Block-Graphen in ein Gradle-Projekt mit Java-Code um |
| `src/build.rs` | Export, Gradle-Aufruf, Java-Suche, Online-Versionsabfrage |
| `src/model.rs` | Projektformat (`.mcb`, JSON) |
| `src/ui/` | Oberfläche: Canvas, Seitenleisten, Inspector, Konsole, Dialoge |
| `assets/templates/` | Java-Hilfsklassen, die in jedes Plugin/jeden Mod kommen |

Kommandozeile (für Tests/Skripte):

```bash
minecraft-builder projekt.mcb                       # Projekt öffnen
minecraft-builder --export projekt.mcb ausgabe/     # nur exportieren
minecraft-builder --selftest paper|fabric ausgabe/  # Testprojekt mit allen Blöcken
```

Nicht offiziell von Mojang oder Microsoft. Die Oberfläche ist von node-basierten Bot-Buildern wie XC Circuit inspiriert, der Code ist eigenständig.
