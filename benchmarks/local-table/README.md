# Local table

A 256-entry `i32` sine table declared inside a function, filled once from a
complete constant list and scanned once. The in-repo `cube` example uses the
same shape for its rotation table; this benchmark makes the initialization
cost visible in the m68k instruction and `.text` budgets.

The program is deterministic and uses only `std.io`. The checksum covers every
entry, so a skipped, zeroed, or partially copied table changes the output.
