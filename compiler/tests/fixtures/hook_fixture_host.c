/* Synchronous C hook fixtures for the v1.1 host behavior tests.
 *
 * Every function calls its callback before returning. `cl_fixture_defer`
 * deliberately stores the callback to prove that an invocation after the hook
 * returns faults with E11215 instead of touching a dead environment.
 */

static int cl_fixture_calls;
static int (*cl_fixture_saved)(int);

int cl_fixture_apply(int (*cb)(int), int value) {
    cl_fixture_calls += 1;
    return cb(value);
}

int cl_fixture_apply2(int (*a)(int), int (*b)(int), int value) {
    return a(value) + b(value);
}

int cl_fixture_repeat(int (*cb)(int), int value) {
    return cb(value) + cb(value);
}

int cl_fixture_reenter(int (*cb)(int), int value) {
    return cb(value);
}

int cl_fixture_apply0(int (*cb)(void), int value) {
    (void)value;
    return cb();
}

int cl_fixture_apply1(int value, int (*cb)(void)) {
    (void)value;
    return cb();
}

void cl_fixture_defer(int (*cb)(int)) {
    cl_fixture_saved = cb;
}

int cl_fixture_run_deferred(int value) {
    return cl_fixture_saved(value);
}

int cl_fixture_count(void) {
    return cl_fixture_calls;
}

static int cl_fixture_a = 7;
static int cl_fixture_b = 9;

void *cl_fixture_ptr_a(void) {
    return &cl_fixture_a;
}

void *cl_fixture_ptr_b(void) {
    return &cl_fixture_b;
}

int cl_fixture_read(void *p) {
    return *(int *)p;
}

int cl_fixture_view(int (*cb)(int), void *p) {
    (void)p;
    return cb(1);
}
