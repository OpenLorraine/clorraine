# Lookup table

A 4096-entry `i32` sine table (scaled by 32767 and offset by 32768 to 1..65535) is
initialized once at module load and scanned once. Precomputed sine, palette,
and audio tables are common in the Amiga programs this compiler targets; the
in-repo `cube` example uses a 256-entry sine table. The table is large enough
for the initialization cost to be visible in the m68k instruction and `.text`
budgets.

The offset makes the checksum non-zero, so a skipped, zeroed, or partially
copied template changes the output. The program is deterministic and uses only
`std.io`.
