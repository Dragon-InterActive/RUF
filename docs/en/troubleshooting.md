# Troubleshooting

## Building

| Message | Cause | Fix |
|---|---|---|
| `The system library gtk4 required by crate gdk4-sys was not found` | GTK development files missing (Linux) | `sudo pacman -S gtk4` / `sudo apt install libgtk-4-dev` / `sudo dnf install gtk4-devel` |
| `package gtk4 … was not found in the pkg-config search path` | `pkg-config` missing or searching the wrong place | install `pkg-config`; set `PKG_CONFIG_PATH` if needed |
| `linker x86_64-w64-mingw32-gcc not found` | Windows build on Linux without MinGW | `sudo pacman -S mingw-w64-gcc` or `gcc-mingw-w64` |
| `error: failed to run custom build command for ring` (cross-compiling to macOS) | C code needs Apple's compiler | build on a Mac; on Linux only `cargo check` RUF/RNCL |

## Runtime

| Message / behaviour | Cause | Fix |
|---|---|---|
| `ruf: Vor dem ersten Widget muss eine ruf::Application erzeugt werden` | component created before `Application::new()` | create the `Application` first |
| `ruf: Es darf nur eine Application pro Prozess existieren` | second `Application` | one instance per process; in tests use an own `main()` (`harness = false`) |
| `ruf: GTK konnte nicht initialisiert werden` | Linux without a graphical session | `xvfb-run -a …`; under Wayland also `GDK_BACKEND=x11` |
| window does not appear | `RnclWindow` was dropped | keep the window variable alive until `exec()` |
| widget does not get smaller (Linux) | GTK never goes below the natural size | shorten text, use wrapping (`set_word_wrap`) or allow more space |
| `on_resized` reports later (Linux) | GTK distributes sizes via its frame clock | call `Application::process_events()` after `show()` |
| callback does not run | signal not available for this kind | check the return value of `on_…` (`false` = not supported) |
| callback is not re-entered | re-entrancy protection | intended: a running handler is not called nested |
| `ruf: Panik in einem Callback abgefangen: …` | panic inside a closure | fix the closure; the application keeps running |
| labels on Windows have a grey background | style without `background` | intended: the system's container background |

## Tests

- **Linux without a display:** `xvfb-run -a cargo test`. Inside a sandbox that protects
  `/tmp/.X11-unix`, Xvfb cannot create its socket – run the tests outside the sandbox.
- **Windows under Wine:** use an own prefix: `WINEPREFIX=/tmp/ruf-wine xvfb-run -a wine test.exe`.
- **Modal dialogs** (`ruf::dialog`) block – don't call them in automated tests.
