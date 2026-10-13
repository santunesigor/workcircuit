#!/bin/sh
# Install the prebuilt release without downloading or building anything.
set -eu

release_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
prefix=${PREFIX:-"$HOME/.local"}
stage=${DESTDIR:-}

case "$prefix" in
    /*) ;;
    *) printf '%s\n' 'PREFIX must be an absolute path.' >&2; exit 1 ;;
esac

install -Dm755 "$release_dir/workcircuit" "$stage$prefix/bin/workcircuit"
desktop_dir="$stage$prefix/share/applications"
icon_dir="$stage$prefix/share/icons/hicolor/scalable/apps"
install -d "$desktop_dir" "$icon_dir"

# Account for desktop-entry string escaping, Exec quoting and field codes.
exec_path=$(printf '%s' "$prefix/bin/workcircuit" | sed 's/\\/\\\\\\\\/g; s/["`$]/\\\\&/g; s/%/%%/g')
while IFS= read -r line; do
    case "$line" in
        Exec=*) printf 'Exec="%s"\n' "$exec_path" ;;
        *) printf '%s\n' "$line" ;;
    esac
done < "$release_dir/data/io.github.workcircuit.WorkCircuit.desktop" \
    > "$desktop_dir/io.github.workcircuit.WorkCircuit.desktop"
chmod 644 "$desktop_dir/io.github.workcircuit.WorkCircuit.desktop"
for icon in "$release_dir"/data/*.svg; do
    install -m644 "$icon" "$icon_dir/"
done

if [ -z "$stage" ]; then
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$prefix/share/icons/hicolor" >/dev/null 2>&1 || :
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$desktop_dir" >/dev/null 2>&1 || :
    fi
fi
printf 'Installed WorkCircuit in %s\n' "$stage$prefix"
