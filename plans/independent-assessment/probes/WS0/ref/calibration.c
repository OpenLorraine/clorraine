/* C calibration: prints the same fixed line as benchmarks/calibration through
 * the same stdio path (fwrite of the bytes, fputc of the newline). Every C
 * reference is netted against this program in each lens cell. */
#include "ref_io.h"

int main(void)
{
    static const char line[] = "calibration: ok";
    ref_write_line(line, sizeof line - 1);
    return 0;
}
