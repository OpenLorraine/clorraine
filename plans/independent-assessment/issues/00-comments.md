## #20

Measured in the independent assessment (baseline `e24a8752`, A500 shipped lens, `vamos` 68000 instruction counts): `-O2 -flto` saves 280–512 B of `.text` and cuts `container` by 7.3%; `-O3` cuts `strings` by 25.1% and `container` by 18.3% for +270–2,216 B (`plans/independent-assessment/findings/WS4-code-quality.md`, ASM-WS4-006; record `findings/records/WS4-flags.log`). The earlier rejection of `-O3` measured the calibration program only.

## #15

Data point from the independent assessment (baseline `e24a8752`): a 68020 probe build (`-m68020`, `vamos -C 68020`) of the benchmarks gains −16.3% net instructions on `strings` (hardware divide) and stays within ±5% elsewhere, while `container` regresses by +21.3% (ASM-WS7-003, `plans/independent-assessment/findings/WS7-performance.md`). The report's recommendation is not to add a 68020 profile on these numbers alone; the `strings` gain is available on every CPU through a 16-bit formatting divide.
