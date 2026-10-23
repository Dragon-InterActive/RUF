#!/usr/bin/env bash
# GTK-4-Laufzeit neben ein RUF-Programm legen (Windows, MSYS2 UCRT64).
# Bundle the GTK 4 runtime next to a RUF program (Windows, MSYS2 UCRT64).
#
# Voraussetzung / prerequisites (MSYS2-Shell "UCRT64"):
#   pacman -S --needed mingw-w64-ucrt-x86_64-gtk4 mingw-w64-ucrt-x86_64-pkgconf \
#                      mingw-w64-ucrt-x86_64-adwaita-icon-theme
#   rustup target add x86_64-pc-windows-gnu
#   cargo build --release --target x86_64-pc-windows-gnu
#
# Aufruf / usage:
#   tools/bundle-windows.sh <programm.exe> <zielordner> [weitere Dateien …]
#
# Ergebnis / result (GTK findet seine Daten relativ zu bin/):
#   <ziel>/bin/<programm>.exe + alle benötigten DLLs
#   <ziel>/share/glib-2.0/schemas/gschemas.compiled
#   <ziel>/share/icons/{Adwaita,hicolor}
#   <ziel>/lib/gdk-pixbuf-2.0/…   (Bildformate)
set -euo pipefail

exe=${1:?Programm (.exe) angeben / give the program}
dest=${2:?Zielordner angeben / give the target directory}
shift 2
prefix=${MINGW_PREFIX:-/ucrt64}
[ -d "$prefix/bin" ] || { echo "MSYS2-Präfix $prefix fehlt – in der UCRT64-Shell starten." >&2; exit 1; }

mkdir -p "$dest/bin"
cp "$exe" "$dest/bin/"

# Alle DLLs, die aus dem MSYS2-Präfix stammen (rekursiv über ldd).
ldd "$exe" | awk '{print $3}' | grep -i "^$prefix/bin/" | sort -u | while read -r dll; do
    cp -u "$dll" "$dest/bin/"
done
# gdbus wird von GLib für einige Dienste gestartet.
[ -f "$prefix/bin/gdbus.exe" ] && cp -u "$prefix/bin/gdbus.exe" "$dest/bin/"

# Einstellungsschemata (Dateiauswahl u. a.).
mkdir -p "$dest/share/glib-2.0/schemas"
cp "$prefix"/share/glib-2.0/schemas/*.xml "$dest/share/glib-2.0/schemas/" 2>/dev/null || true
glib-compile-schemas "$dest/share/glib-2.0/schemas"

# Symbole.
mkdir -p "$dest/share/icons"
for theme in Adwaita hicolor; do
    [ -d "$prefix/share/icons/$theme" ] && cp -r "$prefix/share/icons/$theme" "$dest/share/icons/"
done

# Bildformat-Module samt ihrer DLLs; der Cache nutzt relative Pfade.
if [ -d "$prefix/lib/gdk-pixbuf-2.0" ]; then
    mkdir -p "$dest/lib"
    cp -r "$prefix/lib/gdk-pixbuf-2.0" "$dest/lib/"
    for loader in "$dest"/lib/gdk-pixbuf-2.0/2.10.0/loaders/*.dll; do
        ldd "$loader" | awk '{print $3}' | grep -i "^$prefix/bin/" | while read -r dll; do
            cp -u "$dll" "$dest/bin/"
        done
    done
    sed -i "s|$(cygpath -m "$prefix")/||g; s|$prefix/||g" \
        "$dest/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache" || true
fi

# Weitere Dateien (Lizenzen, Doku, locales …) ins Wurzelverzeichnis.
for extra in "$@"; do
    cp -r "$extra" "$dest/"
done

echo "Fertig / done: $dest (Start: bin/$(basename "$exe"))"
