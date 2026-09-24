#!/usr/bin/env bash
#
# Build the Clorraine demo story.
#
# It records one clip per chapter, adds explanatory cards and a caption on each
# chapter, and writes:
#
#   demo/demo-full.mp4   the whole story
#   demo/demo.gif        a short opening loop (intro cards + hello demo)
#   demo/emulator.mp4    the string-parser emulator chapter on its own
#
# The story text lives in this file (the variables below), so it is edited in
# one place. Intermediates are written under build/ (gitignored).
#
# Requirements:
#   - Docker with the SDK image built: make -C docker image
#   - VHS v0.11.0, ttyd, ffmpeg, xwininfo on PATH
#   - For the emulator chapters: an X display (docker/.env DISPLAY and
#     XAUTHORITY) and a real Kickstart in docker/.env
#
# Usage: ./demo/story.sh

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# --- Cards -----------------------------------------------------------------
CARD_DURATION="${CARD_DURATION:-7}"

# Opening: introduce Clorraine first.
CLO_TITLE="Clorraine"
CLO_SUB="A safe, C-like language for the classic AmigaOS and the future LorraineOS"
CLO_L1="Makes programming the Amiga easy"
CLO_L2="Rust-style ownership · compiles to plain C99"

# Shown between the hello source and the emulator run.
HELLO_EXPLAIN_TITLE="C-like syntax, checked like Rust"
HELLO_EXPLAIN_SUB="Ownership and borrows are checked at compile time"
HELLO_EXPLAIN_L1="The compiler emits plain C99 the Amiga toolchains accept"

# Marks the move from hello to a real program.
REAL_TITLE="Now a real program"
REAL_SUB="A parser that sums numbers and formats the result"

# Introduces the ownership chapter.
OWN_TITLE="Ownership, checked"
OWN_SUB="Every value has one owner; using it after it moves is a compile error"
OWN_L1="This catches memory bugs a C compiler cannot"

# Introduces the build/run chapter.
BUILD_TITLE="One command"
BUILD_SUB="Clorraine emits plain C99, then builds and runs it"
BUILD_L1="Here for the hosted-amiga profile"

# Introduces the graphics chapter.
DRAW_TITLE="Now: draw"
DRAW_SUB="A custom Amiga screen with colors, bars, and lines"

# Closing: LorraineOS, the project Clorraine belongs to.
LOS_TITLE="LorraineOS"
LOS_SUB="Planned: a clean-room implementation for the m68k Amiga"
LOS_L1="Clorraine is part of that effort"
LOS_L2="Very early in design"

OUTRO_TITLE="Clorraine · early development"
OUTRO_L1="spec frozen · compiler MVP working · corpus green"
OUTRO_L2="github.com/OpenLorraine"
OUTRO_L3="Contact: github.com/blendsdk"

# --- Chapter captions ------------------------------------------------------
CAPTION_HELLO="The smallest Clorraine program"
CAPTION_HELLO_EMU="Running under FS-UAE (Amiga emulator)"
CAPTION_SOURCE="Real Clorraine source — C-like syntax, ownership checked at compile time"
CAPTION_OWNERSHIP="Use after move? The compiler rejects it (E10020) — C cannot do this"
CAPTION_RUN="One command: build the m68k binary and run it (hosted-amiga profile)"
CAPTION_EMULATOR="The same program under FS-UAE — 68k, Kickstart 3.1"
CAPTION_GRAPHICS="Drawing on an Amiga screen — under FS-UAE (Amiga emulator)"

# --- Paths and fonts -------------------------------------------------------
raw_dir="build/demo/raw"
cap_dir="build/demo/captioned"
text_dir="build/demo/text"
mkdir -p "$raw_dir" "$cap_dir" "$text_dir"

font_regular=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
font_bold=/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf

# VHS is commonly installed under ~/go/bin by `go install`.
export PATH="$HOME/go/bin:$HOME/.local/bin:$PATH"

for tool in vhs ttyd ffmpeg xwininfo docker; do
    command -v "$tool" >/dev/null 2>&1 || { echo "error: $tool is not on PATH" >&2; exit 1; }
done

# --- Helpers ---------------------------------------------------------------
write_text() {
    printf '%s' "$2" > "${text_dir}/$1.txt"
}

record_tape() {
    local tape="$1" out="$2"
    if [ -s "$out" ] && [ "$out" -nt "$tape" ]; then
        echo "reusing ${out} (newer than ${tape})"
        return
    fi
    rm -f "$out"
    echo "recording ${tape} ..."
    vhs "$tape"
    if [ ! -s "$out" ]; then
        echo "error: $out was not produced; VHS v0.12.0 is broken, use v0.11.0" >&2
        exit 1
    fi
}

# Re-encode one chapter to the common format: 30 fps, 1100x700, yuv420p. The
# chapter content sits in the top 1100x620; the bottom 80 px is a strip for the
# caption, so the caption never covers terminal text. The optional extra filter
# is the caption itself.
normalize() {
    local in="$1" out="$2" extra="${3:-}"
    local vf="fps=30,scale=1100:620:force_original_aspect_ratio=decrease,pad=1100:620:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,pad=1100:700:0:0:color=0x1e1e2e"
    [ -n "$extra" ] && vf="${vf},${extra}"
    ffmpeg -v error -y -i "$in" -vf "$vf" -pix_fmt yuv420p -an -c:v libx264 -crf 20 "$out"
}

caption_filter() {
    printf 'drawtext=fontfile=%s:textfile=%s:fontsize=26:fontcolor=white:x=(w-text_w)/2:y=h-52' \
        "$font_regular" "$1"
}

# card OUT "name:bold|regular:size:color:y" ...
card() {
    local out="$1"
    shift
    local filters="" spec name font size color y f
    for spec in "$@"; do
        IFS=':' read -r name font size color y <<<"$spec"
        if [ "$font" = "bold" ]; then f="$font_bold"; else f="$font_regular"; fi
        [ -n "$filters" ] && filters="${filters},"
        filters="${filters}drawtext=fontfile=${f}:textfile=${text_dir}/${name}.txt:fontsize=${size}:fontcolor=${color}:x=(w-text_w)/2:y=${y}"
    done
    ffmpeg -v error -y -f lavfi -i "color=c=0x1e1e2e:s=1100x700:r=30:d=${CARD_DURATION}" \
        -vf "${filters},format=yuv420p" -c:v libx264 -crf 20 -an "$out"
}

# concat OUT INPUT...  (re-encodes so mixed sources stay consistent)
concat() {
    local out="$1"
    shift
    local list="${raw_dir}/concat.txt" f
    : > "$list"
    for f in "$@"; do printf "file '%s/%s'\n" "$repo_root" "$f" >> "$list"; done
    ffmpeg -v error -y -f concat -safe 0 -i "$list" \
        -c:v libx264 -crf 20 -pix_fmt yuv420p -an "$out"
}

# --- Write story text ------------------------------------------------------
write_text los_title "$LOS_TITLE"
write_text los_sub "$LOS_SUB"
write_text los_l1 "$LOS_L1"
write_text los_l2 "$LOS_L2"
write_text clo_title "$CLO_TITLE"
write_text clo_sub "$CLO_SUB"
write_text clo_l1 "$CLO_L1"
write_text clo_l2 "$CLO_L2"
write_text hello_explain_title "$HELLO_EXPLAIN_TITLE"
write_text hello_explain_sub "$HELLO_EXPLAIN_SUB"
write_text hello_explain_l1 "$HELLO_EXPLAIN_L1"
write_text real_title "$REAL_TITLE"
write_text real_sub "$REAL_SUB"
write_text own_title "$OWN_TITLE"
write_text own_sub "$OWN_SUB"
write_text own_l1 "$OWN_L1"
write_text build_title "$BUILD_TITLE"
write_text build_sub "$BUILD_SUB"
write_text build_l1 "$BUILD_L1"
write_text draw_title "$DRAW_TITLE"
write_text draw_sub "$DRAW_SUB"
write_text outro_title "$OUTRO_TITLE"
write_text outro_l1 "$OUTRO_L1"
write_text outro_l2 "$OUTRO_L2"
write_text outro_l3 "$OUTRO_L3"

write_text cap_hello "$CAPTION_HELLO"
write_text cap_hello_emu "$CAPTION_HELLO_EMU"
write_text cap_source "$CAPTION_SOURCE"
write_text cap_ownership "$CAPTION_OWNERSHIP"
write_text cap_run "$CAPTION_RUN"
write_text cap_emulator "$CAPTION_EMULATOR"
write_text cap_graphics "$CAPTION_GRAPHICS"

# --- Record the terminal chapters ------------------------------------------
record_tape demo/tape/hello-source.tape "${raw_dir}/hello-source.mp4"
record_tape demo/tape/source.tape "${raw_dir}/source.mp4"
record_tape demo/tape/ownership.tape "${raw_dir}/ownership.mp4"
record_tape demo/tape/run.tape "${raw_dir}/run.mp4"

# --- Record the emulator chapters ------------------------------------------
echo "recording the hello emulator chapter ..."
./demo/record-emulator.sh examples/hello/hello.clo "${raw_dir}/hello-emulator.mp4"

echo "recording the string-parser emulator chapter ..."
./demo/record-emulator.sh spec/tests/corpus/string-parser/program.clo "${raw_dir}/emulator.mp4"

echo "recording the graphics emulator chapter ..."
./demo/record-emulator.sh spec/tests/corpus/graphics/program.clo "${raw_dir}/graphics.mp4"

# --- Caption the chapters --------------------------------------------------
normalize "${raw_dir}/hello-source.mp4"  "${cap_dir}/hello-source.mp4"  "$(caption_filter "${text_dir}/cap_hello.txt")"
normalize "${raw_dir}/hello-emulator.mp4" "${cap_dir}/hello-emulator.mp4" "$(caption_filter "${text_dir}/cap_hello_emu.txt")"
normalize "${raw_dir}/source.mp4"        "${cap_dir}/source.mp4"        "$(caption_filter "${text_dir}/cap_source.txt")"
normalize "${raw_dir}/ownership.mp4"     "${cap_dir}/ownership.mp4"     "$(caption_filter "${text_dir}/cap_ownership.txt")"
normalize "${raw_dir}/run.mp4"           "${cap_dir}/run.mp4"           "$(caption_filter "${text_dir}/cap_run.txt")"
normalize "${raw_dir}/emulator.mp4"      "demo/emulator.mp4"            "$(caption_filter "${text_dir}/cap_emulator.txt")"
normalize "${raw_dir}/graphics.mp4"      "${cap_dir}/graphics.mp4"      "$(caption_filter "${text_dir}/cap_graphics.txt")"

# --- Cards -----------------------------------------------------------------
card "${raw_dir}/los.mp4" \
    "los_title:bold:56:white:170" \
    "los_sub:regular:28:0xcccccc:270" \
    "los_l1:regular:24:0xaaaaaa:350" \
    "los_l2:regular:24:0xaaaaaa:400"
card "${raw_dir}/clorraine.mp4" \
    "clo_title:bold:56:white:190" \
    "clo_sub:regular:28:0xcccccc:290" \
    "clo_l1:regular:24:0xaaaaaa:360" \
    "clo_l2:regular:24:0xaaaaaa:400"
card "${raw_dir}/hello-explain.mp4" \
    "hello_explain_title:bold:40:white:220" \
    "hello_explain_sub:regular:24:0xcccccc:320" \
    "hello_explain_l1:regular:24:0xaaaaaa:370"
card "${raw_dir}/real.mp4" \
    "real_title:bold:48:white:230" \
    "real_sub:regular:26:0xcccccc:330"
card "${raw_dir}/own-explain.mp4" \
    "own_title:bold:48:white:220" \
    "own_sub:regular:24:0xcccccc:320" \
    "own_l1:regular:24:0xaaaaaa:370"
card "${raw_dir}/build-explain.mp4" \
    "build_title:bold:48:white:220" \
    "build_sub:regular:24:0xcccccc:320" \
    "build_l1:regular:24:0xaaaaaa:370"
card "${raw_dir}/draw-explain.mp4" \
    "draw_title:bold:48:white:220" \
    "draw_sub:regular:24:0xcccccc:320"
card "${raw_dir}/outro.mp4" \
    "outro_title:bold:44:white:190" \
    "outro_l1:regular:26:0xcccccc:270" \
    "outro_l2:regular:28:0x88ccff:340" \
    "outro_l3:regular:24:0xaaaaaa:410"

# --- Assemble the full story -----------------------------------------------
concat demo/demo-full.mp4 \
    "${raw_dir}/clorraine.mp4" \
    "${cap_dir}/hello-source.mp4" \
    "${cap_dir}/hello-emulator.mp4" \
    "${raw_dir}/hello-explain.mp4" \
    "${raw_dir}/real.mp4" \
    "${cap_dir}/source.mp4" \
    "${raw_dir}/own-explain.mp4" \
    "${cap_dir}/ownership.mp4" \
    "${raw_dir}/build-explain.mp4" \
    "${cap_dir}/run.mp4" \
    "demo/emulator.mp4" \
    "${raw_dir}/draw-explain.mp4" \
    "${cap_dir}/graphics.mp4" \
    "${raw_dir}/los.mp4" \
    "${raw_dir}/outro.mp4"

# --- Short opening loop for the README -------------------------------------
concat "${cap_dir}/opening.mp4" \
    "${raw_dir}/clorraine.mp4" \
    "${cap_dir}/hello-source.mp4" \
    "${cap_dir}/hello-emulator.mp4" \
    "${raw_dir}/hello-explain.mp4"
ffmpeg -v error -y -i "${cap_dir}/opening.mp4" \
    -vf "fps=12,scale=1100:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse" \
    -loop 0 demo/demo.gif

echo "wrote demo/demo-full.mp4, demo/demo.gif, demo/emulator.mp4"
