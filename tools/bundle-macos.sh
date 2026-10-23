#!/usr/bin/env bash
# Ein RUF-Programm samt GTK-4-Laufzeit als .app bündeln (macOS, Homebrew).
# Bundle a RUF program with the GTK 4 runtime as a .app (macOS, Homebrew).
#
# Voraussetzung / prerequisites:
#   brew install gtk4 adwaita-icon-theme dylibbundler
#   cargo build --release
#
# Aufruf / usage:
#   tools/bundle-macos.sh <programm> <Name.app> [bundle-id] [weitere Dateien …]
#
# Ergebnis / result:
#   Name.app/Contents/MacOS/<programm>      Startskript (setzt GTK-Umgebung)
#   Name.app/Contents/MacOS/<programm>-bin  das Programm
#   Name.app/Contents/libs/                 alle dylibs (Pfade umgeschrieben)
#   Name.app/Contents/Resources/share/…     Schemata, Symbole
set -euo pipefail

bin=${1:?Programm angeben / give the program}
app=${2:?Name.app angeben / give Name.app}
name=$(basename "$bin")
id=${3:-net.dragon-software.$name}
shift 3 2>/dev/null || shift $#
prefix=$(brew --prefix)

contents="$app/Contents"
mkdir -p "$contents/MacOS" "$contents/libs" "$contents/Resources/share/glib-2.0/schemas" "$contents/Resources/share/icons"
cp "$bin" "$contents/MacOS/$name-bin"

# dylibs einsammeln und auf @executable_path/../libs umschreiben.
dylibbundler -od -b -x "$contents/MacOS/$name-bin" -d "$contents/libs/" -p @executable_path/../libs/

# Schemata und Symbole.
cp "$prefix"/share/glib-2.0/schemas/*.xml "$contents/Resources/share/glib-2.0/schemas/" 2>/dev/null || true
glib-compile-schemas "$contents/Resources/share/glib-2.0/schemas"
for theme in Adwaita hicolor; do
    [ -d "$prefix/share/icons/$theme" ] && cp -R "$prefix/share/icons/$theme" "$contents/Resources/share/icons/"
done

# Startskript: GTK soll die mitgelieferten Daten finden.
cat > "$contents/MacOS/$name" <<LAUNCH
#!/bin/sh
here="\$(cd "\$(dirname "\$0")" && pwd)"
res="\$here/../Resources"
export XDG_DATA_DIRS="\$res/share"
export GSETTINGS_SCHEMA_DIR="\$res/share/glib-2.0/schemas"
export GTK_PATH="\$res"
exec "\$here/$name-bin" "\$@"
LAUNCH
chmod +x "$contents/MacOS/$name"

cat > "$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>$name</string>
    <key>CFBundleIdentifier</key><string>$id</string>
    <key>CFBundleName</key><string>$(basename "$app" .app)</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

for extra in "$@"; do
    cp -R "$extra" "$contents/Resources/"
done

# Ohne Apple-Entwicklerzertifikat: ad-hoc signieren, damit Apple Silicon startet.
codesign --force --deep --sign - "$app"
echo "Fertig / done: $app"
