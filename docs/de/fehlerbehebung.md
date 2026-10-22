# Fehlerbehebung

## Bauen

| Meldung | Ursache | Lösung |
|---|---|---|
| `The system library gtk4 required by crate gdk4-sys was not found` | GTK-Entwicklungsdateien fehlen (Linux) | `sudo pacman -S gtk4` / `sudo apt install libgtk-4-dev` / `sudo dnf install gtk4-devel` |
| `package gtk4 … was not found in the pkg-config search path` | `pkg-config` fehlt oder sucht falsch | `pkg-config` installieren; ggf. `PKG_CONFIG_PATH` setzen |
| `linker x86_64-w64-mingw32-gcc not found` | Windows-Build von Linux ohne MinGW | `sudo pacman -S mingw-w64-gcc` bzw. `gcc-mingw-w64` |
| `error: failed to run custom build command for ring` (Kreuzkompilierung macOS) | C-Code braucht Apples Compiler | auf einem Mac bauen; unter Linux nur `cargo check` für RUF/RNCL |

## Laufzeit

| Meldung / Verhalten | Ursache | Lösung |
|---|---|---|
| `ruf: Vor dem ersten Widget muss eine ruf::Application erzeugt werden` | Komponente vor `Application::new()` erzeugt | `Application` als Erstes anlegen |
| `ruf: Es darf nur eine Application pro Prozess existieren` | zweite `Application` | eine Instanz pro Prozess; in Tests ein eigenes `main()` (`harness = false`) |
| `ruf: GTK konnte nicht initialisiert werden` | Linux ohne grafische Sitzung | `xvfb-run -a …`; unter Wayland zusätzlich `GDK_BACKEND=x11` |
| Fenster erscheint nicht | `RnclWindow` wurde gedroppt | Fenster-Variable bis `exec()` am Leben halten |
| Widget wird nicht kleiner (Linux) | GTK unterschreitet nie die natürliche Größe | Text kürzen, Umbruch (`set_word_wrap`) oder größeren Platz vorsehen |
| `on_resized` meldet erst später (Linux) | GTK verteilt Größen über die Frame-Clock | nach `show()` `Application::process_events()` aufrufen |
| Callback läuft nicht | Signal gibt es für diese Art nicht | Rückgabewert von `on_…` prüfen (`false` = nicht unterstützt) |
| Callback ruft sich nicht selbst erneut auf | Re-Entranz-Schutz | gewollt: ein laufender Handler wird nicht verschachtelt aufgerufen |
| `ruf: Panik in einem Callback abgefangen: …` | Panik in einer Closure | Fehler in der Closure beheben; die Anwendung läuft weiter |
| Labels in Windows grau hinterlegt | Stil ohne `background` | gewollt: Container-Hintergrund des Systems |

## Tests

- **Linux ohne Bildschirm:** `xvfb-run -a cargo test`. In einer Sandbox, die `/tmp/.X11-unix`
  schützt, kann Xvfb seinen Socket nicht anlegen – Tests dann außerhalb der Sandbox starten.
- **Windows unter Wine:** eigenes Präfix verwenden: `WINEPREFIX=/tmp/ruf-wine xvfb-run -a wine test.exe`.
- **Modale Dialoge** (`ruf::dialog`) blockieren – in automatischen Tests nicht aufrufen.
