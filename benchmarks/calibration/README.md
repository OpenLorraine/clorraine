# Calibration

Imports the console path, prints `calibration: ok`, and exits. The harness
subtracts its instruction count from every scored program, so the reported
`instructions_net` is the benchmark's own work rather than fixed startup.
