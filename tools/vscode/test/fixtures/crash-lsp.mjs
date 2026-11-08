// A server that exits immediately, used to prove pending requests reject.

process.stderr.write("fake crash\n");
process.exit(1);
