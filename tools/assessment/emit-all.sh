#!/usr/bin/env bash
# Baseline emission record for a Clorraine assessment run (measurement only;
# it records outcomes and carries no expectation).
#
# Usage (inside the SDK container, from the repository root):
#   emit-all.sh <release-clc> <debug-clc> <out-dir>
#
# For every spec corpus program, every benchmark, and every examples/amiga
# program, on every profile, it emits C twice with the release clc and once
# with the debug clc, records acceptance (exit status and the first
# diagnostic code), whether the two release emissions are byte-identical,
# whether the debug emission equals the release one, and the compile-only
# (-c) wall time of every accepted emission on each C compiler of that
# profile: host cc for host; m68k-amigaos-gcc for the Amiga profiles and
# freestanding; vbcc additionally for hosted-amiga and hosted-amiga-13.
# Output: <out-dir>/record.tsv plus the emitted files and logs.
set -uo pipefail

if [ "$#" -ne 3 ]; then
  echo "usage: $0 <release-clc> <debug-clc> <out-dir>" >&2
  exit 2
fi
clc=$1
dclc=$2
out=$3
mkdir -p "$out"
ulimit -f 1048576

host_flags=(-std=c99 -O2 -ffunction-sections -fdata-sections)
nix20=(-std=c99 -mcrt=nix20 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections)
nix13=(-std=c99 -mcrt=nix13 -m68000 -fwrapv -O2 -ffunction-sections -fdata-sections)
free=(-std=c99 -ffreestanding -fno-builtin -nostdlib -m68000 -fwrapv -O2)
vbcc=(+aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include)

now() { date +%s%N; }

# compile <log> <cmd...>: run a compile-only command, print "ok <ms>" or "fail <ms>".
compile() {
  local log=$1
  shift
  local t0 t1 rc
  t0=$(now)
  timeout 600 "$@" > "$log" 2>&1
  rc=$?
  if [ "$rc" -ne 0 ] && grep -q 'No config file!' "$log"; then
    t0=$(now)
    timeout 600 "$@" > "$log" 2>&1
    rc=$?
  fi
  t1=$(now)
  if [ "$rc" -eq 0 ]; then
    echo "ok $(( (t1 - t0) / 1000000 ))"
  else
    echo "fail $(( (t1 - t0) / 1000000 ))"
  fi
}

programs=()
for f in spec/tests/corpus/*/program.clo benchmarks/*/program.clo examples/amiga/*.clo; do
  programs+=("$f")
done

printf 'program\tprofile\taccepted\tdiag\tc_lines\tdeterministic\tdebug_equal\tcc\tcc_ms\tgcc\tgcc_ms\tvbcc\tvbcc_ms\n' > "$out/record.tsv"
for src in "${programs[@]}"; do
  id=$(echo "$src" | sed 's#/program.clo$##; s#\.clo$##; s#/#__#g')
  for profile in host hosted-amiga hosted-amiga-13 freestanding; do
    dir="$out/$id/$profile"
    mkdir -p "$dir"
    timeout 120 "$clc" --profile "$profile" -o "$dir/a.c" "$src" > "$dir/emit-a.log" 2>&1
    rc=$?
    if [ "$rc" -ne 0 ]; then
      diag=$(grep -o -m1 -E '\b[EW][0-9]{5}\b' "$dir/emit-a.log" || true)
      printf '%s\t%s\tno\t%s\t-\t-\t-\t-\t-\t-\t-\t-\t-\n' "$src" "$profile" "${diag:-rc$rc}" >> "$out/record.tsv"
      continue
    fi
    timeout 120 "$clc" --profile "$profile" -o "$dir/b.c" "$src" > "$dir/emit-b.log" 2>&1
    timeout 300 "$dclc" --profile "$profile" -o "$dir/debug.c" "$src" > "$dir/emit-debug.log" 2>&1
    det=no; cmp -s "$dir/a.c" "$dir/b.c" && det=yes
    deq=no; cmp -s "$dir/a.c" "$dir/debug.c" && deq=yes
    lines=$(wc -l < "$dir/a.c")
    cc_r=-; gcc_r=-; vbcc_r=-
    case "$profile" in
      host)
        cc_r=$(compile "$dir/cc.log" cc "${host_flags[@]}" -c -o "$dir/a.o" "$dir/a.c") ;;
      hosted-amiga)
        gcc_r=$(compile "$dir/gcc.log" m68k-amigaos-gcc "${nix20[@]}" -c -o "$dir/a.o" "$dir/a.c")
        vbcc_r=$(compile "$dir/vbcc.log" vc "${vbcc[@]}" -c -o "$dir/a-vbcc.o" "$dir/a.c") ;;
      hosted-amiga-13)
        gcc_r=$(compile "$dir/gcc.log" m68k-amigaos-gcc "${nix13[@]}" -c -o "$dir/a.o" "$dir/a.c")
        vbcc_r=$(compile "$dir/vbcc.log" vc "${vbcc[@]}" -c -o "$dir/a-vbcc.o" "$dir/a.c") ;;
      freestanding)
        gcc_r=$(compile "$dir/gcc.log" m68k-amigaos-gcc "${free[@]}" -c -o "$dir/a.o" "$dir/a.c") ;;
    esac
    printf '%s\t%s\tyes\t-\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$src" "$profile" "$lines" "$det" "$deq" \
      "${cc_r% *}" "${cc_r#* }" "${gcc_r% *}" "${gcc_r#* }" "${vbcc_r% *}" "${vbcc_r#* }" >> "$out/record.tsv"
  done
  echo "emit-all: $src"
done
echo "emit-all: wrote $out/record.tsv"
