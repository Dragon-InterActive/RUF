# Release: Ausliefern für Linux, Windows und macOS

🇬🇧 [English version](../en/release.md)

RUF-Programme nutzen GTK 4 auf allen Systemen. Unter Linux ist GTK Teil des Systems
(Paketabhängigkeit `gtk4`). **Unter Windows und macOS liefert das Release die
GTK-Laufzeit mit** – die Skripte in `tools/` erledigen das.

| System | Bauen auf | Paket | Skript |
|---|---|---|---|
| Linux | Linux | Binary (+ Abhängigkeit `gtk4` ≥ 4.12) | – |
| Windows | Windows 10/11, MSYS2 „UCRT64“ | Ordner/ZIP mit `bin/`, `share/`, `lib/` | `tools/bundle-windows.sh` |
| macOS | Mac mit Homebrew | `Name.app` | `tools/bundle-macos.sh` |

## Windows

Einmalig in der MSYS2-Shell „UCRT64“:

```bash
pacman -S --needed mingw-w64-ucrt-x86_64-gtk4 mingw-w64-ucrt-x86_64-gtksourceview5 mingw-w64-ucrt-x86_64-pkgconf \
                   mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-adwaita-icon-theme
rustup target add x86_64-pc-windows-gnu
```

`rustup`/`cargo` aus der Windows-Installation müssen in der UCRT64-Shell im `PATH`
sein (`MSYS2_PATH_TYPE=inherit`); alternativ `pacman -S mingw-w64-ucrt-x86_64-rust`.

Bauen und bündeln (Beispiel CT-Editor; `$RUF` = ein Checkout von RUF, in Freya TDE `../RUF`):

```bash
cargo build --release --target x86_64-pc-windows-gnu
$RUF/tools/bundle-windows.sh target/x86_64-pc-windows-gnu/release/ct-editor.exe \
    dist/ct-editor-windows-x86_64 docs LICENSE-MIT LICENSE-APACHE README.md
```

Ergebnis: `dist/ct-editor-windows-x86_64/bin/ct-editor.exe` mit allen DLLs daneben,
dazu `share/` (Einstellungsschemata, Symbole) und `lib/` (Bildformate). GTK findet
seine Daten relativ zu `bin/` – der Ordner kann beliebig verschoben und gezippt
werden. Mit einer Verknüpfung auf `bin\ct-editor.exe` startet man bequem.

## macOS

Einmalig:

```bash
brew install gtk4 gtksourceview5 adwaita-icon-theme dylibbundler
```

Bauen und bündeln:

```bash
cargo build --release
$RUF/tools/bundle-macos.sh target/release/ct-editor dist/CT-Editor.app \
    net.dragon-software.ct-editor assets/ct-editor.icns docs LICENSE-MIT LICENSE-APACHE
```

Eine `.icns` unter den weiteren Dateien wird das Programmsymbol (Finder, Dock);
ein Windows-Programm bettet seine `.ico` selbst ein (z. B. `build.rs` mit `embed-resource`,
siehe CT-Editor).

Das Skript kopiert alle dylibs nach `Contents/libs/` und schreibt die Pfade um,
legt Schemata und Symbole nach `Contents/Resources/share/` und startet das Programm
über ein kleines Startskript, das GTK diese Orte nennt. Ohne Apple-Entwickler-
zertifikat wird ad hoc signiert; beim ersten Start ggf. Rechtsklick → „Öffnen“.

## Prüfen vor der Auslieferung

- Auf einem **sauberen** Rechner (ohne MSYS2 bzw. Homebrew) starten – nur so fällt
  eine fehlende DLL/dylib auf.
- Dateiauswahl öffnen (braucht Schemata und Symbole).
- Hell/Dunkel-Umschaltung prüfen.

## Lizenz der GTK-Laufzeit

GTK, GLib, Pango, Cairo usw. stehen unter LGPL-2.1-or-later bzw. vergleichbaren
freien Lizenzen. Dynamisch gelinkt und unverändert mitgeliefert ist das erlaubt;
dem Release sollte ein Hinweis auf diese Bibliotheken und ihre Lizenzen beiliegen
(z. B. `THIRD-PARTY.txt` mit Link zu den Quellen von MSYS2 bzw. Homebrew).
