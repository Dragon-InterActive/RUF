# Release: shipping for Linux, Windows and macOS

🇩🇪 [Deutsche Fassung](../de/release.md)

RUF programs use GTK 4 on all systems. On Linux GTK is part of the system (package
dependency `gtk4`). **On Windows and macOS the release ships the GTK runtime** – the
scripts in `tools/` do that.

| System | Build on | Package | Script |
|---|---|---|---|
| Linux | Linux | binary (+ dependency `gtk4` ≥ 4.12) | – |
| Windows | Windows 10/11, MSYS2 "UCRT64" | folder/ZIP with `bin/`, `share/`, `lib/` | `tools/bundle-windows.sh` |
| macOS | Mac with Homebrew | `Name.app` | `tools/bundle-macos.sh` |

## Windows

Once, in the MSYS2 shell "UCRT64":

```bash
pacman -S --needed mingw-w64-ucrt-x86_64-gtk4 mingw-w64-ucrt-x86_64-gtksourceview5 mingw-w64-ucrt-x86_64-pkgconf \
                   mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-adwaita-icon-theme
rustup target add x86_64-pc-windows-gnu
```

`rustup`/`cargo` of the Windows installation must be in the `PATH` of the UCRT64
shell (`MSYS2_PATH_TYPE=inherit`); alternatively `pacman -S mingw-w64-ucrt-x86_64-rust`.

Build and bundle (example CT-Editor; `$RUF` = a checkout of RUF, `../RUF` inside Freya TDE):

```bash
cargo build --release --target x86_64-pc-windows-gnu
$RUF/tools/bundle-windows.sh target/x86_64-pc-windows-gnu/release/ct-editor.exe \
    dist/ct-editor-windows-x86_64 docs LICENSE-MIT LICENSE-APACHE README.md
```

Result: `dist/ct-editor-windows-x86_64/bin/ct-editor.exe` with all DLLs next to it,
plus `share/` (settings schemas, icons) and `lib/` (image formats). GTK finds its data
relative to `bin/` – the folder can be moved and zipped freely. A shortcut to
`bin\ct-editor.exe` makes starting convenient.

## macOS

Once:

```bash
brew install gtk4 gtksourceview5 adwaita-icon-theme dylibbundler
```

Build and bundle:

```bash
cargo build --release
$RUF/tools/bundle-macos.sh target/release/ct-editor dist/CT-Editor.app \
    net.dragon-software.ct-editor assets/ct-editor.icns docs LICENSE-MIT LICENSE-APACHE
```

A `.icns` among the extra files becomes the program icon (Finder, Dock); a Windows
program embeds its `.ico` itself (e.g. `build.rs` with `embed-resource`, see the
CT-Editor).

The script copies all dylibs to `Contents/libs/` and rewrites their paths, puts
schemas and icons into `Contents/Resources/share/` and starts the program through a
small launcher that tells GTK these locations. Without an Apple developer
certificate it signs ad hoc; on first start use right click → "Open" if needed.

## Check before shipping

- Start on a **clean** machine (without MSYS2 or Homebrew) – only then a missing
  DLL/dylib shows up.
- Open the file chooser (needs schemas and icons).
- Check the light/dark switch.

## License of the GTK runtime

GTK, GLib, Pango, Cairo etc. are under LGPL-2.1-or-later or similar free licenses.
Shipping them dynamically linked and unchanged is allowed; the release should include
a note on these libraries and their licenses (e.g. `THIRD-PARTY.txt` with a link to
the sources at MSYS2 or Homebrew).
