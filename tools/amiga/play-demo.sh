#!/usr/bin/env bash
# Play a Clorraine showcase demo in a local FS-UAE installation.
#
# The images under site/public/demos are bootable Amiga 500 floppies, the same
# files the website serves. This launcher gives a local, full-speed run without
# the browser emulator. The repository does not ship Kickstart ROMs: point the
# script at your own licensed 1.3 image, or keep it in the default location.
#
# Usage:
#   tools/amiga/play-demo.sh <demo>
#   tools/amiga/play-demo.sh --list
#
# Examples:
#   tools/amiga/play-demo.sh prism
#   tools/amiga/play-demo.sh boing --rom /path/to/amiga-os-130.rom
#
# Quit with F12 (the FS-UAE menu) or by closing the window. While the emulator
# window has focus, the cursor keys move the joystick and right Ctrl or right
# Alt fires. If the display stays black on a remote or virtual desktop, retry
# with:
#   SDL_VIDEODRIVER=x11 LIBGL_ALWAYS_SOFTWARE=1 tools/amiga/play-demo.sh prism
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(dirname "$(dirname "${script_dir}")")"
demos_dir="${repo_dir}/site/public/demos"

# Kickstart 1.3 is a 256 KB image. Cloanto ships the same data in a slightly
# larger encrypted wrapper; FS-UAE needs the adjacent rom.key to read it.
raw_rom_size=262144
encrypted_rom_size=262155
adf_size=901120

usage() {
    cat <<'EOF'
Play a Clorraine showcase demo in a local FS-UAE installation.

Usage:
  tools/amiga/play-demo.sh <demo>
  tools/amiga/play-demo.sh --list

Options:
  --rom FILE   Licensed Kickstart 1.3 image.
               Default: $CLORRAINE_AMIGA_ROM, then
               ~/amiga/roms/amiga-os-130.rom
  --list       Print the available demo names.
  -h, --help   Show this help.

Controls: cursor keys move, right Ctrl or right Alt fires.
Quit: F12 opens the FS-UAE menu; closing the window also quits.
EOF
}

list_demos() {
    local path
    for path in "${demos_dir}"/*.adf; do
        [ -e "${path}" ] || continue
        basename "${path}" .adf
    done
}

demo=""
rom="${CLORRAINE_AMIGA_ROM:-${HOME}/amiga/roms/amiga-os-130.rom}"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --list)
            list_demos
            exit 0
            ;;
        --rom)
            [ "$#" -ge 2 ] || { echo "play-demo: --rom needs a file" >&2; exit 2; }
            rom="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --*)
            echo "play-demo: unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
        *)
            [ -z "${demo}" ] || { echo "play-demo: only one demo at a time" >&2; exit 2; }
            demo="$1"
            shift
            ;;
    esac
done

if [ -z "${demo}" ]; then
    usage >&2
    exit 2
fi

adf="${demos_dir}/${demo}.adf"
if [ ! -f "${adf}" ]; then
    echo "play-demo: no demo named '${demo}'" >&2
    echo "play-demo: available demos:" >&2
    list_demos | sed 's/^/  /' >&2
    exit 2
fi

if ! command -v fs-uae >/dev/null 2>&1; then
    echo "play-demo: fs-uae is not installed" >&2
    echo "play-demo: install it with your package manager, for example:" >&2
    echo "  sudo apt install fs-uae" >&2
    exit 2
fi

if [ ! -f "${rom}" ]; then
    echo "play-demo: missing Kickstart 1.3 ROM: ${rom}" >&2
    echo "play-demo: pass --rom FILE or set CLORRAINE_AMIGA_ROM" >&2
    exit 2
fi

rom_size="$(wc -c < "${rom}")"
if [ "${rom_size}" != "${raw_rom_size}" ] && [ "${rom_size}" != "${encrypted_rom_size}" ]; then
    echo "play-demo: warning: ${rom} is not a 256 KB Kickstart 1.3 image" >&2
fi
if [ "${rom_size}" = "${encrypted_rom_size}" ] && [ ! -f "$(dirname "${rom}")/rom.key" ]; then
    echo "play-demo: warning: the ROM is encrypted but has no rom.key beside it" >&2
fi
if [ "$(wc -c < "${adf}")" != "${adf_size}" ]; then
    echo "play-demo: warning: ${adf} is not a standard 880 KB floppy image" >&2
fi

echo "Demo: ${demo}"
echo "ADF:  ${adf}"
echo "ROM:  ${rom}"
echo "Controls: cursor keys move, right Ctrl or right Alt fires; F12 opens the menu."

# The keyboard joystick is mapped to Amiga port 2 (FS-UAE port 1). Port 1 is
# left to the mouse, so moving the host pointer cannot reach a game that reads
# only the joystick port. Writable floppies are disabled to keep the committed
# ADF untouched.
exec fs-uae \
    --amiga-model=A500 \
    --kickstart-file="${rom}" \
    --floppy-drive-0="${adf}" \
    --writable-floppy-images=0 \
    --joystick-port-0=nothing \
    --joystick-port-1=keyboard \
    --window-width=960 \
    --window-height=720 \
    --title="Clorraine - ${demo}"
