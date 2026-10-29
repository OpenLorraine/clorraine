# Findings Ledger: Zero-Defect Closeout

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)

| Finding | Phase | Severity | Area | Ruling |
|---|---|---|---|---|
| — | — | — | Awaiting execution | — |

## Preflight Rulings

| Ref | Ruling | Date |
|---|---|---|
| AR-3 | `len(cstring)` is supported at runtime; scalar `len` reports `E10200` | 2026-10-28 |
| AR-4 | Add `E10309` for a call of a non-callable value | 2026-10-28 |
| AR-5 | Authorize the bounded instance-bound static-return marker for `DEF-557` | 2026-10-28 |
| AR-6 | `DEF-190` names the call spelling in `E10010` | 2026-10-28 (auto-design) |
| AR-7 | `DEF-191` drops the `unsafe` exemption at the index sites | 2026-10-28 (auto-design) |
| AR-8 | `DEF-193` narrows through short-circuit right operands and loop heads | 2026-10-28 (auto-design) |
| AR-9 | `DEF-263` evaluates a folded `len` receiver once | 2026-10-28 (auto-design) |
| AR-20 | `DEF-169` escapes the type-key components (the user-approved ABI change) | 2026-10-28 |
| AR-21 | `DEF-252` adds `E10044` for a move out of a `const` binding | 2026-10-28 |

## Phase 0 — Revalidation and Baseline (2026-10-28)

HEAD `2aa138c`; every row was re-run with the driver at HEAD. The three
read-only reports are summarized here: `confirmed` rows proceed to their
batch, `stale`/`incidental` rows close with HEAD evidence, `manual` rows
need a repro re-derivation in their batch, and `needs-ruling` rows stop for
a user decision.

| Row | Status | HEAD evidence |
|---|---|---|
| DEF-169 | needs-ruling | `struct ClWrap__fn_borrow_i32__i32` redefinition: the two func-type keys collide |
| DEF-173 | confirmed | false `E10020 'temp'` supplied by a foreign `mark(take String)` |
| DEF-174 | incidental | `E10039` now reported (fixed by `memory-critical-residue`, `53c152d`) |
| DEF-190 | confirmed | `let g: str = get();` (call returning `String?`) accepted, no `E10010` |
| DEF-191 | confirmed | inside `unsafe`: index and `str` member read accepted; the narrowing conversion reports `E10010` |
| DEF-192 | confirmed | `v.len` and `v.ptr` on `u8[]?` accepted, no `E10010` |
| DEF-193 | confirmed | `s != null && len(s) > 0` reports `E10010` twice (chain + body) |
| DEF-194 | confirmed | `E(def194); S(def194)` EMIT 0; CC 1: `comparison of unsigned expression in '< 0' is always false [-Werror=type-limits]` at `slice(v,0,0)`; emitted line 599 `if ((usize)(0) > (usize)((v).len)) cl_trap(...)` \| repro: `func useView(v: u8[]): i32 { let w: u8[] = slice(v,0,0); return i32(len(w)); }` called with `slice(xs,0,3)` |
| DEF-195 | confirmed | `len(usize)` and `len(cstring)` accepted, both emit `(i32)(0)` |
| DEF-198 | confirmed | initializer `f(take ...)` accepted with no `E10039` |
| DEF-199 | confirmed | `E(def199); S(def199)` EMIT 0; CC 1: `incompatible type for argument 1 of 'cl_lambda_0'`; parameter `ClStr`, argument `ClString {aka void *}`; emitted `n = cl_lambda_0(__cl_owned0);` \| repro: `let n: i32 = ((s: str) => i32(len(s)))(stringOf("hello"));` |
| DEF-200 | incidental | `E(def200)` with `def200.clo` + `def200_ab.clo`; `S(def200); R(def200)` EMIT 0; emitted `f = cl_A_B_helper;` (line 561), no dotted path; CC 0; RUN 0 \| repro: `module A.B; export func helper(): i32 { return 5; }` + `let f: func(): i32 = A.B.helper;` |
| DEF-201 | confirmed | `E(def201); S(def201)` EMIT 0; CC 1: `division by zero [-Werror=div-by-zero]`; emitted `return (1 / 0);` (line 560) in `cl_lambda_0` \| repro: `let f: func(): i32 = () => 1 / 0; return f();` |
| DEF-202 | incidental | `E(def202b); S(def202b); R(def202b)`; `E(def202c); S(def202c); R(def202c)` array of funcs emits `static i32 (*cl_probe_def202b_fs[2])(void) = {0};` (line 542); func returning an array emits `static void (*cl_probe_def202c_fs)(i32 __cl_ret[2])`; both CC 0, RUN 0. The row's exact type `func(): i32[2]` is a function returning `i32[2]`, no… |
| DEF-203 | incidental | `E10038 cannot borrow global 'glob'` (fixed by `checker-conformance`, `88b73b4`) |
| DEF-205 | confirmed | emits `((ClSlice_unknown){ ((3).ptr) + ... })`, strict build fails |
| DEF-206 | confirmed | `E(def206b); S(def206b)` module initializer emits the division guard twice: lines 559 and 561 both `if (d == 0 \ \| repro: \ |
| DEF-208 | incidental | `E(def208); S(def208); R(def208)`; same for `def208b` EMIT 0; emitted `x = (c ? a : cl_string_view(b));`; CC 0; RUN 0; both arm orders clean. Row's "accepted, strict build fails" is gone \| repro: `let a: str = "ab"; let b: String = stringOf("cd"); let x: str = c ? a : b;` |
| DEF-210 | confirmed | `E(def210); A(def210); R(def210)` EMIT 0; CC 0; RUN exit 1 with `LeakSanitizer: 11 byte(s) leaked in 1 allocation(s)`; stack `cl_init_probe_def210` → `two()` → `stringOf` (element 1 is the residual sibling) \| repro: `let g: i32 = consume(two()[0]);` with `two(): String[2]` and `consume(take xs: String)` |
| DEF-212 | confirmed | `W10000 unused local 'i'` on a store index |
| DEF-214 | needs-ruling | `s.x(3)` emits `x(s, 3)`; strict build fails; code choice unassigned |
| DEF-217 | incidental | `E(def217b); S(def217b); R(def217b)` typedef now `typedef struct { i32 (ptr)(i32); usize len; } ClSlice_fn_i32__i32;` (line 538); `sizeof((func(i32): i32)[])` is `sizeof(ClSlice_fn_i32__i32)`; CC 0, RUN 0 \| repro: `func use(fs: (func(i32): i32)[]): i32 { return fs[0](1); }` + `sizeof((func(i32): i32)[])` |
| DEF-218 | incidental | `E(def218b); S(def218b); R(def218b)`; `def218d` same discarded `slice<i32>(xs,0,2);` and `slice<i32[3]>(grid,0,2);` emit no `slice(` call (no undeclared intrinsic), CC 0, RUN 0 \| repro: `slice<i32>(xs, 0, 2);` as a statement |
| DEF-221 | incidental | `E(def221); S(def221); R(def221)` emitted `__cl_tmp0[0] = 10; … g = __cl_tmp0[pos];` (lines 555–558), not `{{10,20,30}}[pos]`; CC 0, RUN 0 \| repro: `let pos: usize = 1; let g: i32 = [10,20,30][pos];` |
| DEF-222 | confirmed | `E(def222); S(def222)` EMIT 0; CC 1: `unknown type name 'ClPair__i32'` at `ClPair__i32 p;` (line 548). Direct `let b: Box<i32> = …` now builds (def222b CC 0) \| repro: `struct Pair<T> { n: T; } struct Box<T> { p: Pair<T>; } func sizeOf<T>(): usize { return sizeof(T[]); } sizeOf<Box<i32>>()` |
| DEF-223 | confirmed | `E(def223); S(def223)` EMIT 0; CC 1: `unknown type name 'ClWrap__i32'`; `typedef struct { ClWrap__i32 *ptr; usize len; } ClSlice_Wrap__i32;` (line 538) \| repro: `struct Wrap<T> { n: T; } let n: usize = sizeof(Wrap<i32>[]);` at module level |
| DEF-224 | confirmed | `E(def224c); S(def224c)`; `E(def224); E(def224b)` `.len` on `String`/`cstring` is now rejected `E10212` (that half is stale), but `len(cstring)` is accepted and emits `n = 0;` (line 570) with no use of `s`; CC 1 `variable 's' set but not used [-Werror=unused-but-set-variable]` \| repro: `let s: cstring = toCString("ab"); let n: usize = … |
| DEF-225 | incidental | `E(def225b); E(def225c)` Both the generic instance (`Box<T>` with concrete `s7/s3`) and the concrete spelling now report one `E10200: cannot convert 'i32[3]' to 'i32[7]'`; no accepted 0 fold \| repro: `len(flag ? b.s7 : b.s3)` over `{ s7: i32[7]; s3: i32[3]; }` |
| DEF-226 | incidental | `E(def226)` EMIT 1: `E10212: 'foo' is not a member of 'Inner'` at `x.v.foo`; no invalid C \| repro: `struct Inner { bar: i32; } struct Outer { v: Inner; } … x.v.foo` |
| DEF-227 | incidental | `E(def227b); S(def227b); R(def227b)` environment emits `{ { xs[0], xs[1], xs[2], xs[3] } }` (line 571), CC 0, RUN 0 \| repro: `func eachLambda<T>(xs: T): i32 { return ([xs] () => i32(len(xs)))(); } eachLambda<i32[4]>(a)` |
| DEF-228 | confirmed | `E(def228); S(def228); R(def228)`; `E(def228b)` generic raw instance accepted and emits `return x->foo;` (line 573), CC 0, RUN 0; the concrete `p.foo` twin is rejected `E10600: dereference of a raw pointer requires 'unsafe'` \| repro: `func get<T>(x: T): i32 { return x.foo; } get<raw S>(p)` with `let p: raw S = unsafe &s;` |
| DEF-234 | incidental | `F(def234); cc -std=c99 -ffreestanding -fno-builtin -nostdlib -O0 -g -Wall -Wextra -Werror …; R(def234)`; also `-O2` EMIT 0; CC 0; RUN 0 at `-O0` and `-O2`; no segfault for the `str[2]` fold unit \| repro: `let a: str[2] = ["ab","cd"]; let n: usize = len(a);` under the freestanding profile |
| DEF-236 | confirmed | func-value `f(1, 2)` emits `f(1, 2)`, C arity error |
| DEF-239 | confirmed | `xs.arrayNew<i32>()` emits `cl_arrayNew__i32(xs)`, C arity error |
| DEF-240 | stale | `std.text.toCString(s, 1)` reports `E10308`; `std.io.Open(str, 0)` reports `E10200` |
| DEF-241 | confirmed | raw receiver emits `p.foo(1)`, C wants `p->foo(1)` |
| DEF-244 | confirmed | uncalled `ffi` `i32[0]` parameter accepted; ordinary declaration reports `E10226` |
| DEF-246 | confirmed | `P { n: null }` and `i32[2] = [1, null]` accepted (the conditional arm already reports `E10012`) |
| DEF-247 | confirmed | `x.len` and `x.foo = 1` on `let x = null;` accepted; emits `(i32)(x.len)` on a `void *` |
| DEF-248 | confirmed | `x + 1` and `x == 5` on the null shape accepted; emits `x + 1` on `void *` |
| DEF-249 | confirmed | `if (x)` on the null shape accepted; emits `if (x)` on `void *` |
| DEF-250 | confirmed | `E10034 's'` at the let site instead of `E10401` |
| DEF-252 | needs-ruling | move out of `const` accepted; no registry code for the rule |
| DEF-253 | incidental | `E(def253); S(def253); R(def253)` `[s];` lowered to `__cl_tmp0 = s; s = NULL; FreeString(__cl_tmp0);` (lines 574–578), CC 0, RUN 0; no `expected ';' before '}'` \| repro: `let s: String = stringOf("ab"); [s];` |
| DEF-254 | confirmed | `--max-errors 0` exits 0 and writes a 18,635-byte TU for a rejected program |
| DEF-263 | incidental | `E(def263c); A(def263c); R(def263c)` the receiver is evaluated at HEAD: emitted `__cl_discard_1 = cl_probe_def263c_mkR(1);` (line 585); the planted null deref in `mkR` makes RUN exit 139 (receiver ran). `len` is still the constant `2u` (line 592), which the row accepts as the length rule \| repro: `(c ? mkR(1) : mkR(2)).xs.len()` with a… |
| DEF-311 | confirmed | LSan: 11 B leak (operand position), 12 B leak (guarded read) |
| DEF-312 | confirmed | false `E10031 cannot modify 'b'` after `Holder { names: [b.w] }` |
| DEF-313 | incidental | `E(def313d); S(def313d); R(def313d)` `slot = c ? null : null;` is accepted, CC 0, RUN 0 (no `incompatible type for argument 1`) \| repro: `func fill(c: bool, borrow slot: str?) { slot = c ? null : null; }` |
| DEF-314 | incidental | `E(def314); A(def314); R(def314)` module view binding of a conditional member: EMIT 0, CC 0, RUN 0, ASAN/LSan clean (no 11 B leak) \| repro: `let g: str = true ? mk().v : mk2().v;` |
| DEF-316 | confirmed | `E(def316b); A(def316b); R(def316b)` EMIT 0; CC 0; RUN exit 1 with `LeakSanitizer: 11 byte(s) leaked in 1 allocation(s)` for the duplicable scalar member argument under `&&` (the residual `String` field is the leak) \| repro: `struct Box { n: i32; v: String; }` + `if (g && use(mk().n) == 1) …` |
| DEF-317 | incidental | `E(def317b); A(def317b); R(def317b)`; `def317c` (loop) owning literal carrier with `tag: String`, both conditional arms and the 3-iteration loop: EMIT 0, CC 0, RUN 0, ASAN/LSan clean (no 22 B / 66 B leak) \| repro: `H { names: [mk().v], tag: stringOf("t") }` local, conditional, and loop |
| DEF-318 | confirmed | `E(def318c); A(def318c); R(def318c)` module initializer view of a literal carrier element: EMIT 0, CC 0, RUN 1 with `LeakSanitizer: 11 byte(s) leaked in 1 allocation(s)` (the residual sibling). Direct `let h: H` at module level is now rejected `E11106` because H is owning \| repro: `let g: str = H { names: [mk().v], tag: stringOf("t") }… |
| DEF-319 | incidental | `E(def319)` EMIT 1: `E10030: borrow of a compiler-generated owner does not live long enough` at the direct literal carrier return; no accepted leak \| repro: `func make(): H { return H { names: [mk().v] }; }` |
| DEF-320 | incidental | `E(def320_260); S(def320_260); R(def320_260)` (also 257, 261) nested conditional carrier (`g ? Holder { names: [c ? id(mk().v) : id(mk().w)] } : Holder { names: ["zz"] }`) builds and runs 0; also `Holder { names: [c ? (d ? mk().v : mk2().v) : mk3().v] }` (def320b) CC 0 \| repro: nested conditional in a carrier view position |
| DEF-321 | incidental | `E(def321)`; `def321b`; `def321d/e` `let g: str = mkH().names[0];` (module and local), the direct read, and the element through a fresh `str[2]` return all report `E10030: borrow of a compiler-generated owner does not live long enough`; no accepted 22 B leak \| repro: `let g: str = mkH().names[0];` with `mkH(): H { names: str[2] }` |
| DEF-324 | manual | `E(def324a); S(def324a)` … `def324g` Three derived guarded-literal spellings (guarded `len(Holder{…}.names[0])`, guarded conditional-root field with scalar and array fields, guarded element conditional with call arms, guarded ternary literals) all build clean at HEAD. The row's original probes are not in the tree (their `build/memory-a… |
| DEF-325 | confirmed | `E(def325); S(def325); R(def325)` call fill emits `xs[0] = f(); xs[1] = f(); xs[2] = f();` (lines 562–564); RUN exit 20 (`calls == 3`, one call per element) instead of 0 \| repro: `let xs: i32[3] = [; f()];` with `f()` counting calls |
| DEF-326 | confirmed | `E(def326b); S(def326b); R(def326b)` `[; i++]` in `take3([; i++])` emits three `i++` stores (line 592); RUN exit 3 (the update ran once per element), plain prefixed fill (`def326`) is pinned once \| repro: `take3([; i++])` and the guarded twin |
| DEF-327 | confirmed | `E(def327); S(def327)` EMIT 0; CC 1 on host GCC 15.2: `operation on '__cl_tmp0' may be undefined [-Werror=sequence-point]` (line 16) for the place-borrowing loop receiver \| repro: `while (i < k && (c ? mkH() : a).call(5) > 0)` with a `take` H place arm |
| DEF-328 | incidental | `E(def328); S(def328); R(def328)` emitted `*(&((*(p)).n)) = 5;` (line 565); `s.n` is 5 at run, RUN 0; the store is not dropped \| repro: `let p: raw S = unsafe &s; unsafe { (*p).n = 5; }` |
| DEF-330 | confirmed | `E(def330); S(def330)` EMIT stderr: `warning W10000: unused local 'r'` at line 7 although the emitted store reads `*(&(*(q))) = *(r);` (line 559) \| repro: `let q: raw i32 = unsafe &a; let r: raw i32 = unsafe &b; unsafe { *q = *r; }` |
| DEF-334 | manual | — The row is the J5C-1 bounded-confirmation subject-only defect (a rolled-back deferred owner id re-taken by a later binding). The J5C-1 probe source is not in the tree; no accepted/rejected shape can be derived reliably from the row \| repro: missing: the J5C-1 probe sequence (rollback-time generated owner + later re-taking binding) |
| DEF-349 | confirmed | `usize(s.ptr)` accepted; host C fails `-Werror=pointer-to-int-cast` |
| DEF-352 | confirmed | `E(def352b); S(def352b)` EMIT 0 (with `unsafe` prefix on `.ptr`/`.len`); CC 1: `request for member 'ptr' in something not a structure or union` and same for `len`; emitted `q = p->ptr;` (line 557), `n = p->len;` (line 559); `.len` also folds to the empty length \| repro: `let p: raw u8 = unsafe &a[0]; let q: raw u8 = unsafe p.ptr; let n… |
| DEF-354 | stale | one `E10401` for the two-arm conditional take (the one-report rule holds) |
| DEF-355 | manual | `E(def355b…k)`; `S(...)`; hosted-amiga `m68k-amigaos-gcc … def355g.amiga.c` Every derived `Box { f: n }` spelling over a `File?` field (`null`, local `File?`, `openFile` result, call argument, return, nested literal, `Box { f: null }`) builds clean at HEAD on host; the nested literal's hosted-amiga emission is also clean under `m68k-am… |
| DEF-358 | confirmed | Source `compiler/tests/def_backlog_closeout_spec_test.rs:146-174`; replica `rustc pipe_demo.rs` + run `run_with_timeout` pipes stdout/stderr and only calls `wait_with_output()` after `try_wait` reports exit; a 300 KiB writer blocks. The replica of that exact loop printed `deadlock: child blocked in write on the undrained pipe` (5 s dea… |
| DEF-370 | manual | no spurious `W10001` on any String-field twin of w2/w4/u3/d8 |
| DEF-382 | confirmed | three `E10200` at the generic literal arguments |
| DEF-383 | confirmed | `pair(s, null)` rejected `E10012` (1) while `pair<str?>(s, null)` accepts |
| DEF-387 | confirmed | `s.foo` accepted where `foo(x: str)` exists; C `ClStr` member error |
| DEF-388 | confirmed | `null.foo` accepted; emits `NULL.foo` |
| DEF-389 | confirmed | module-path dot-call reports `E10212`, not `E10219` |
| DEF-392 | confirmed | `E(def392)`, `E(def392b)`, `E(def392c)`, `E(def392d)` All four constant contexts report `E10224: expression is not constant`: `sizeof(E)` (enum), `sizeof(S)` with an enum field, `sizeof(File)` (handle), `sizeof(u8[])` (view) \| repro: `enum E : u8 { A, B } const n: usize = sizeof(E);` etc. |
| DEF-393 | confirmed | `E(def393c); S(def393c)`; `E(def393d); S(def393d); R(def393d)`; `E(def393)` Value position emits `usize x = 0;` (line 563) and marks the field-name binding `b` unused; folded `const k = offsetof(S,b)` emits `static usize k = 0;` (line 553) though `b` follows an `i32` at offset 4; `offsetof(S,b)` without a binding `b` reports `E10219: '… |
| DEF-404 | confirmed | `1.put("x")` reports `E10200` (dot path); qualified path now accepts |
| DEF-405 | confirmed | `id<u8>(7)`, `pair(a, 6)`, `id<i8>(300)` all report `E10200` |
| DEF-414 | stale | one `E10011`, zero `E10012`, both the `let y: T?` and the conditional shape |
| DEF-417 | confirmed | `enum E: i32?`, `(x: i32?)` lambda, `func(i32?)`, `ffi` param, `sizeof(i32?)` all accepted |
| DEF-419 | confirmed | `import { Array as A }` emits `i32 xs`, implicit `cl_arrayNew__i32` |
| DEF-420 | confirmed | `let x: f32 = -5.0;` rejected `E10200`; `5.0` accepts |
| DEF-422 | confirmed | `E10600` on `unsafe p.f = 7` with `p` from `getp()` |
| DEF-434 | confirmed | overwritten `[a.v, "EE"]` still reports through the fallback (`E10035`) |
| DEF-438 | manual | `E(def438…g)` The row's exact `g(idx, take *nextp(p,q,&idx))` is rejected at HEAD with `E10200: cannot convert 'raw i32' to 'i32' implicitly` (deref of a call-result raw pointer), so the argument-order behavior cannot be observed. A local raw deref (`take *p`) builds, so the gap is the call-result deref spelling \| repro: missing: a com… |
| DEF-444 | confirmed | no report; emitted take clears the pointee to `NULL`; run exit 139 |
| DEF-463 | confirmed | `W10000 unused local 'q'` on the raw RHS pointer |
| DEF-467 | confirmed | `unsafe { p = *(&p); }` in an out-slot accepted |
| DEF-474 | incidental | generic `borrow T` view return now accepts (`159fd67`, def-critical-major-closeout) |
| DEF-482 | incidental | `E(def482b); S(def482b); R(def482b)` emitted per-arm materialization `v = (g ? (((ok ? ((__cl_tmp0 = mkInner())) : n2(__cl_tmp0.row)), __cl_tmp0.row[0][0]) : 0);` (line 594); CC 0, RUN 0; no mismatched ternary arm types \| repro: `g ? (ok ? mkInner() : Inner { row: n2() }).row[0][0] : 0` |
| DEF-483 | confirmed | `len(u8)` accepted, emits `(i32)(0)` |
| DEF-484 | confirmed | static loop-alias store rejected `E10033 'slot'` (recorded `E10030 'hs'`) |
| DEF-490 | confirmed | static inline literal store rejected `E10033 'slot'` (recorded `E10030` temp) |
| DEF-491 | confirmed | extra `E10033 'names'` + `E10031` on the conditional arm; plain twin has only `E10031` |
| DEF-492 | confirmed | static factory carrier store rejected `E10033 'slot'` (recorded `E10030`) |
| DEF-494 | stale | taken view parameter reports once (`E10030 'p'`), not twice |
| DEF-500 | confirmed | `E(def500_nullc); S(def500_nullc)`; `E(def500_embed)` `let x = null; return i32(x);` is accepted and emits `return (i32)(x);` with `x` a `void *`; CC 1 `cast from pointer to integer of different size [-Werror=pointer-to-int-cast]`. The `embed` golden class is stale: `E10213_embed_outside_const.clo` now reports `E10213` \| repro: `let x … |
| DEF-524 | confirmed | generic body 1× `E10020`, concrete twin 2× |
| DEF-530 | stale | generic 2× `E10030`, concrete 2× (both return sites) |
| DEF-542 | confirmed | `copy<str>(borrow v, borrow slot)` accepted; emits `void cl_copy__str(ClStr v, ClStr slot)`; write lost (slot stays `"zz"`); concrete twin reports `E10033` |
| DEF-544 | confirmed | raw-deref destination store `unsafe { (*h).v = c ? p : q; }` accepted; ASAN double-free; emits `*(&((*(h)).v)) = p; p = NULL;` |
| DEF-545 | confirmed | two takes of one saved destination accepted; ASAN double-free |
| DEF-546 | confirmed | `take h: T` + concrete lend `s` returned through `c ? h : s` at `T=String` accepted; ASAN double-free |
| DEF-547 | confirmed | array-literal element `[c ? v : s, v]` at a `T=str` instance reports `E10034` (1) while the concrete twin accepts |
| DEF-548 | confirmed | runtime-index store of a struct literal with a fixed-array call field emits `mkArr()` with no hidden destination; strict C: `too few arguments` |
| DEF-549 | confirmed | `(unsafe (*g.p).n) / 0` accepted, emits `((*(g.p)).n / 0)`; strict C: `-Werror=div-by-zero` |
| DEF-550 | manual | 11 spellings all report the twin's own duty (`E10033`/`E10034`/`E10031`) or hit another rule; exact RV-005 probe not derivable |
| DEF-551 | confirmed | `[; mk()]` evaluates once per element in the statement, guarded, argument, and nested-row positions (3/3/2/6 calls for 3/3/2/2×3 slots) |
| DEF-552 | confirmed | `useTwo(a(), Outer { xs: [mk(), 8], n: 2 })` traces `213`, not `123` |
| DEF-553 | confirmed | LSan: `Direct leak of 11 byte(s)` from a store-needing init literal's owning field |
| DEF-554 | stale | 5 store spellings materialize `__cl_owned0` and release it on every exit; ASAN/LSan clean |
| DEF-555 | confirmed | nested fixed-array call field in a module initializer traces `21`, not `12` |
| DEF-556 | confirmed | `f<T>(s: str, t: str): T` at `T=str` accepted; ASAN heap-use-after-free; concrete twin reports `E10036` |
| DEF-557 | stale | generic and concrete `viewTag` both accepted (exit 0); the `E10036` rejection is gone |

**Counts:** 75 confirmed, 30 stale/incidental (closed), 6 manual (DEF-550, 370, 324, 334, 355, 438), 3 needs-ruling (DEF-169, 214, 252). DEF-214 is already resolved by AR-4 (`E10309`); DEF-169 and DEF-252 need a new ruling before Phase 3/4.

**Stale/incidental closes:** DEF-554, DEF-557, DEF-414, DEF-174, DEF-203, DEF-240, DEF-354, DEF-474, DEF-494, DEF-530, DEF-200, DEF-202, DEF-208, DEF-217, DEF-218, DEF-221, DEF-225, DEF-226, DEF-227, DEF-234, DEF-253, DEF-263, DEF-313, DEF-314, DEF-317, DEF-319, DEF-320, DEF-321, DEF-328, DEF-482.

**Manual repros:** DEF-550 (the RV-005 view-destination loan probe must be re-derived from `own/place.rs`/`own/statement` code), DEF-370 (the exact `w2`/`w4`/`u3`/`d8` probe), DEF-324 (the guarded literal spellings), DEF-334 (the J5C-1 rollback/re-take sequence), DEF-355 (the exact `Box { f: n }` null local), DEF-438 (the carrier/argument order). Each is carried into its batch and cannot be closed without a red pin.

## Phase 1 — Critical/Major Backlog

- `DEF-549` fixed at `2cf9c5e`: a literal zero divisor in a non-constant
  division emits the E11210 trap and never the bare division; the
  `def_zero_critical_spec_test` case is red before the fix and the full
  `make check` is green (75 targets, 110 goldens).
- `DEF-548` fixed at the runtime-index store path: a struct literal with a
  fixed-array call field is built in a statement temporary, stored, and
  emptied; the strict build and the stored value are pinned in
  `def_zero_critical_spec_test`; `make check` is green.
- `DEF-544` fixed: a conditional stored through a raw-deref destination judges
  every arm as a named destination does; a plain (borrow) parameter arm reports
  one E10034 and is not moved. `make check` is green.
- `DEF-551` fixed: an impure array fill evaluates once into a temporary and the
  temporary is copied to every covered element; a pure fill keeps the inline
  spelling. The statement and nested-row counts are pinned; the emission pins
  stay green.
- `DEF-552` fixed: a later argument that needs a statement fill pins every
  earlier argument whose evaluation has an effect, so the fill follows that
  effect instead of hoisting above it. The cross-argument order is pinned in
  `def_zero_critical_spec_test`; `make check` is green.
- `DEF-555` fixed: an effectful initializer field stores into an init local at
  its source position when a later field needs a statement, so the later fill
  cannot hoist above it. The 1-2 order is pinned; `make check` is green.
- `DEF-553` fixed: a store-needing initializer literal registers its local for
  release; a view destination that reads one member empties that member first,
  so the residual siblings free and the view keeps the member's storage. The
  case is strict-build and LSan clean under `RDC_ASAN=1`; `make check` is
  green.
- `DEF-545` fixed: each consumed subrange is cleared in the same comma sequence
  as its move-out, so a second take of one saved destination cannot overwrite
  the first range. The case is ASAN clean; `make check` is green.
- `DEF-546` fixed: a maybe-owning return threads its destination into the move
  judgment; an owning instance reports the concrete lend arm's `E10034` once
  per return site, while a view instance copies the arm. `make check` is
  green.
- `DEF-547` fixed: the literal field/element visitor, the take-argument path,
  and the read materialization thread the destination type into the move
  judgment, so a view instance copies a concrete lend arm instead of
  reporting the owning-instance move. The view element, take, and read cases
  are pinned; `make check` is green.
- `DEF-556` fixed: a type-parameter return records a view-return candidate
  duty; at a view instance the substituted return applies the concrete
  one-candidate rule, while a single candidate or a static return stays
  accepted. `make check` is green.
- `DEF-550` fixed: a generic store into a borrowed slot whose instance is a
  view records the escape duty, so a foreign lend stored into the caller's
  view slot reports the concrete `E10033`. The re-derived RV-005 probe is the
  borrowed view-slot store; the generic and concrete twins are pinned and
  `make check` is green. **Phase 1 now has no open critical or major row.**

**Phase 1 review (emission lens).** One critical finding: `DEF-555`'s new
source-order store passed an earlier impure **fixed-array** initializer field
to `store_impure_init_operand`, whose plain local assignment cannot spell a
brace list or an arm-selected array, so a module initializer emitted invalid
C. Fixed by a fixed-array branch in `materialize_init_local`: the local is
declared zeroed and filled element-wise at the field's position (guarded when
the operand is guarded). The re-audit over 4,084 programs found 4,082
byte-identical emissions, the two intended new spec sources, zero acceptance
changes, and clean host/amiga/freestanding strict builds; the new spec case is
red on the pre-fix compiler. Report-only pre-existing shapes kept out of scope:
a whole member of a call result as a fixed-array field (`mkw().rows`) still
fails all three profiles (it never compiled on base), and a guarded pure
bounds check can be emitted twice in the fixed-array fill path.

## Phase 2 — Null and Type Rules

## Phase 3 — Calls and Resolution

## Phase 4 — Borrow and Loans

## Phase 5 — Emission and ABI

## Phase 6 — Initializers and Carriers

## Phase 7 — Final Verification and Close

**Phase 1 review (correctness/semantics and memory lenses).** The independent
reviews found eleven findings in the Phase 1 surface. Fixed in the review-fix
batch:

- Type-parameter view returns: a direct local owner reports `E10035` at a view
  instance; a `take`-only or two-candidate return reports `E10036`; a
  parameter-backed conditional records the mixed candidate duty. A local copy
  of a parameter stays accepted, so the identity take/view round trip and the
  narrowed-local case keep their acceptance.
- Raw conditional stores: a type-parameter arm stored through a raw pointer
  records the destination-aware duty (one report per raw store site), so a
  concrete owning destination reports the same `E10034` as the concrete twin.
- Owned local/temporary/take sources widened into a borrowed view slot report
  `E10030`; the nested initializer literal registers every store-needing
  literal and resolves a member read through the inner local, moving the read
  member out before release.
- Self-read runtime-index literal stores use the statement temporary; a cast
  literal zero divisor (`i32(0)`, `%` included) emits the `E11210` trap; an
  impure fixed-array initializer field fills element-wise; pure earlier
  initializer fields and arguments pin before a later statement fill.
- The nested-row fill evaluates the fill expression once for the whole fill
  (the spec rule), and a conditional fill of store-needing struct literals
  fills through the per-arm struct store.

A user ruling resolved the one spec-test conflict: the parameter half of
`should_accept_a_narrowed_nullable_arm_in_every_instance` now expects
`E10036`, because `String?` and `str` are two view-like lend candidates
(EL-1/EL-3) and the concrete twin already reports it. The local half stays
accepted. Two pre-existing shapes remain report-only: a slice taken over a
fixed-array field of an initializer temporary (`slice(S{...}.xs, ...)`) points
into the init function's stack storage, and a view-array initializer of
per-element literal projections leaks its element roots; both are base == head
and belong to the Phase 6 initializer/carrier batch. A whole member of a call
result as a fixed-array field (`mkw().rows`) and a duplicated pure bounds check
in the fixed-array fill path are also pre-existing emission shapes; the emission
re-audit classed them below the critical/major bar for this batch.

Verification after the batch: `make check` green (75 targets, 110 goldens) and
`make verify: all legs passed`. The Phase 1 re-review is the remaining gate.

**Phase 1 review, second round (memory and emission lenses).** The re-audits
found regressions in the first review-fix batch, all fixed:

- Registering every initializer literal freed a view backing widened from a
  literal member; only the resolved read root registers now, and a containing
  literal is materialized with its field stores so its other initializers
  (including effects) still run. The conditional nested-read path rebuilds the
  read with the resolved accessors, matching its move-out.
- `literal_zero_value` now evaluates integer constants, including casts and
  folded arithmetic: `u8(256)`, `i32(0.0)`, and `2 - 2` trap, while `~0` and a
  float zero divide normally. Cast and `%` forms are pinned.
- A fresh owned temporary stored into a borrowed view slot reports `E10030`;
  a raw out-slot stays accepted (it is duplicable, not a view slot).
- A pure dot receiver and a pure written argument pin before a later
  **effectful** statement fill; a pure materialization does not change the
  emission. A callable-field holder keeps its direct call spelling.
- The identity `take`/view round trip stays accepted per the spec identity
  test; the local-owner view escape remains a separate row for the ruling
  below.

Second-round verification: `make check` green (75 targets, 110 goldens) and
`make verify: all legs passed`; the new ASAN cases (widened literal member,
conditional nested read) are clean.

**Phase 1 review, third round (local view returns).** A user ruling resolved the
last spec-test conflict: a narrowed nullable **local** returned as a
type-parameter view reports the concrete `E10035` (spec/06 FN-5), while the
nullable **parameter** half reports `E10036` (two view-like candidates). The
local-owner duty now covers a direct local, a member, an element, and a
conditional with a local owner; a fresh owned temporary reports `E10036`; a
local copy of a parameter keeps the spec's identity take/view round trip
accepted. The semantic-parity test was renamed to
`should_judge_a_narrowed_nullable_arm_per_instance` and both halves now pin the
rejections; member/element/conditional/fresh cases are in
`def_zero_critical_spec_test.rs`.

**Phase 1 result.** All 12 live critical/major rows are fixed and verified.
The three-lens review found and the batch fixed: initializer literal
over-registration and conditional nested reads (two heap-use-after-frees),
nested containing-literal effects, constant integer zero divisors, statement
temporaries and raw slots in the view-slot rule, pure receiver/argument order,
local/member/element/conditional/fresh type-parameter view returns, and the
earlier direct-local/take/two-candidate rules. Report-only, pre-existing and
outside the 114-row scope: a module-level `slice` over a fixed-array
initializer temporary is stack-backed; a view-array initializer of per-element
literal projections leaks its element roots; a non-store literal projection
leaks its unread sibling; a `take` view identity can leave an 11-byte backing
(the spec's identity test requires acceptance); `mkw().rows` never compiled on
any revision; and a duplicated pure bounds check remains. These are recorded
for a later initializer/carrier batch, not reopened as Phase 1 rows.

Verification at the Phase 1 close: `make check` green (75 targets, 110
goldens); `make verify: all legs passed`; the focused `RDC_ASAN=1` spec suite
is clean (20/20).

**Phase 2 implementation (null and type rules).** The null/type batch is
implemented and the new `def_zero_null_spec_test.rs` (14 cases) is green:

- `DEF-190`: a nullable call result widened into a non-nullable view reports
  `E10010` naming the call (`value 'get()' may be null here`).
- `DEF-191`: the index and store-index null checks no longer exempt `unsafe`.
- `DEF-192`: a qualified read checks the null state of every hop's base, so
  `v.len`/`v.ptr` on a nullable slice reports `E10010`.
- `DEF-193`: `&&`/`||` evaluate their right operand under the left comparison's
  narrowing fact; the true edge of `&&` and the false edge of `||` narrow the
  body, and a loop head narrows the loop body.
- `DEF-195`/`DEF-483`: `len(cstring)` emits `cl_cstring_len`; `len` on
  `usize`/`u8` reports `E10200`. A `cstring` can hold the empty pointer without
  a `?`, so `len(c)` requires a `!= null` narrowing and the unchecked spelling
  reports `E10010` (the runtime helper is not null-safe). The frozen
  raw-pointer empty-length fallback stays accepted. A generic `len(x)` records
  a per-instance duty: a scalar substitution reports `E10200`, and a
  `cstring` substitution keeps the `E10010` null requirement unless the
  declaration already narrowed the operand.
- `DEF-246`: a literal `null` in a struct field or array element reports
  `E10012`; the established null-shape binding lowering stays accepted.
- `DEF-247`: a read or store through a null-shaped head reports `E10212`.
- `DEF-248`: a null-shaped binding in an arithmetic or comparison operand
  reports `E10012`; the legal null comparison against a view and the
  uncontracted-ffi null report keep their frozen paths.
- `DEF-249`: a null-shaped condition reports `E10218`.
- `DEF-383`: an inferred generic call with a null argument uses the resolved
  nullable parameter, so `pair(s, null)` with `s: str?` keys `cl_pair__strQ`
  and accepts exactly as `pair<str?>(s, null)`.
- `DEF-417`: TY-4 recurses through function signatures, array elements, and raw
  pointees and now covers enum backing, lambda parameters, ffi parameters, and
  `sizeof`/`offsetof` written types.
- `DEF-420`: a negated float literal takes its f32 destination context.
- `DEF-467`: a raw self-copy of a nullable out-slot borrow reports the direct
  spelling's `E10020`; the non-nullable borrow keeps its frozen `E10034`.
- `DEF-542`: at HEAD the generic `copy<str>(borrow v, borrow slot)` already
  reports `E10033` exactly as the concrete twin, so the row is already green;
  the new spec case pins both spellings. `DEF-414` was closed stale in Phase 0
  and needs no Phase 2 work.

`def_zero_null_impl_test.rs` pins the runtime cstring length call, the inferred
nullable instance, and the narrowed loop-head C condition. `make check` is
green (77 targets, 110 goldens) and `make verify` passes all legs.

**Phase 2 review (emission, memory, semantics).** The three lenses reviewed
`1ff9ccf` and found further critical and major shapes. Fixed in the review
batch:

- Fixed arrays are no longer treated as null-capable destinations; a literal or
  null-shaped value at a fixed-array destination reports `E10012` in locals,
  assignments, returns, fields, elements, fills, and module initializers.
- Module initializers run a null/aggregate walk: a nullable call result, a
  nullable global read, a fixed-array null field/element, and a module
  `len(cstring)` report before startup. The walk also applies conditional-arm
  narrowing.
- Module bindings now have null narrowing state and null checks: global raw
  index/store/member reads, global cstring lengths, and global qualified reads
  report `E10010`; a `!= null` guard narrows a global for the guarded body.
- Short-circuit evaluation keeps the right operand's moves and writes while
  removing only its transient narrowing fact; a `borrow`/`take` argument or a
  write in the right operand drops the left fact before the guarded body.
  `&&`/`||` chains narrow through every left fact, `for`-C heads narrow the
  body, and `?:` arms carry their edge facts.
- Dot `len` requires the cstring null check; generic dot `len` records the
  per-instance length duty; the duty key includes the source file and function,
  so two same-offset sites report separately.
- Nullable raw store targets (index and member) report `E10010`.
- FFI parameter and return types are validated at the declaration, including
  unused declarations; an invalid nullable return types as unresolved so its
  use does not add a dependent `E10010`.
- A call result with a nameless callee (lambda, closure, or conditional value)
  reports `E10010` with the `call()` fallback.

One specification-test conflict was resolved by user ruling (A): the nullable
global `s` in `module_initializer_integrity_spec_test` now checks `s == null`
before `len(s)`, because the global TY-6 rule applies to module bindings as
well. The implementation test's matching source received the same guard.

Review-batch verification: `make check` green (77 targets, 110 goldens); `make
verify: all legs passed`; focused ASAN/LSan runs of the narrowed global,
module-conditional, and cstring paths are clean.

**Phase 2 review, second round (re-audit).** The re-audits found residual
spellings of the same rules; all are fixed:

- Global narrowing now lives in the flow state, so branch joins, else edges,
  and loop exits keep or drop a module-binding fact exactly as a local one.
  A user or `ffi` call is treated as a possible global write; an unmarked
  `borrow` argument is recognized from the declaration rather than the optional
  call-site marker.
- Module-initializer reads now cover the dot `len` spelling, `&&`/`||` right
  operands, nested call arguments, template placeholders, and the unsafe gate
  for raw index/member/address-of reads.
- A transient `&&`/`||` edge restores facts that were already true, so an outer
  local or global narrowing survives an inner short-circuit.
- The module initializer spec test's nullable global gained the user-approved
  `s == null` guard; the matching implementation test was updated.

Second-round verification: `make check` green (77 targets, 110 goldens); `make
verify: all legs passed`; focused ASAN/LSan probes of the newly narrowed and
rejected global/module paths are clean.

**Phase 2 review, third round (final residuals).** The remaining criticals and
major call-shape findings were fixed:

- A callable value (local, parameter, field, or module func-typed binding)
  resolves its own signature before any same-named module function, so an
  unmarked `borrow`/`write` through a value clears the fact; a func-typed call
  also invalidates module narrowing.
- A fact restored after a short-circuit right operand is dropped when that
  operand writes the owner, including the module-initializer forms.
- A member of a call result used as a dot `len` receiver reports `E10010`;
  the non-place member has no binding to narrow.
- Module generic calls substitute their explicit type arguments and dot-method
  arguments check the written parameters.
- Module raw arithmetic, raw-pointer casts, and volatile intrinsics keep the
  `unsafe` gate.
- Transient short-circuit restoration preserves outer local/global facts only
  when the right operand did not write them.

Report-only, pre-existing: a dot `len` on a module-qualified value
(`probe.n.c.len()` / `probe.n.c.len`) is rejected by the qualified path while
the free `len(probe.n.c)` spelling works; this spelling is outside the row
surface and unchanged by the Phase 2 fixes.

Verification: `make check` green (77 targets, 110 goldens); `make verify: all
legs passed`.

**Phase 2 review, fourth round (non-place cstring and raw aliases).** The last
critical shapes were fixed:

- A raw-pointer argument to a call is treated as a possible write to a local
  owner, so a nulled cstring cannot be read after the call.
- A non-place cstring root (conditional, dereference, call-result element,
  array-literal element) reports `E10010` for a dot or free `len`/index use.
- A sibling field write is path-sensitive and no longer drops a different
  field's narrowing.
- The module array-literal index form is rejected instead of emitting invalid
  C.

Report-only precision trade-offs, not DEF rows: a no-argument user call is
conservatively treated as writing every module binding, so a different global
or a pure function-pointer call may be falsely rejected after a global guard;
and the qualified module-value dot `len` spelling (`probe.n.c.len`) remains
unsupported while the free spelling works. Pre-existing conditional
`String`/raw null-dereference shapes are unchanged at base and head.

Final Phase 2 verification: `def_zero_null_spec_test` 43 cases and the two
implementation pins are green; `make check` green (77 targets, 110 goldens);
`make verify: all legs passed`; ASAN/LSan probes of the accepted controls are
clean.

**Phase 3 implementation (calls and resolution).** The call/resolution batch
is implemented with `def_zero_calls_spec_test.rs` (22 cases) and
`def_zero_calls_impl_test.rs` (two emission pins):

- `DEF-169`: user-derived type-key components are escaped (`u<length>_` for a
  component with a separator/`Q`/reserved spelling, `p<length>_` for a
  generic parameter), and the C type definitions use the same component, so a
  crafted struct name cannot redefine a func-type instance. `spec/17` records
  the ABI spelling.
- `DEF-173`: a module-level func-typed global is resolved before a same-named
  function in another loaded module, so the global's own modes type the call.
- `DEF-198`: the module-initializer call walk runs the written-marker check, so
  `f(take ...)` reports `E10039` exactly as the body twin does.
- `DEF-205`: a func-typed local, parameter, or module binding named like a view
  builder is called through its value; the intrinsic gate does not intercept
  it.
- `DEF-212`: a store index is evaluated as a read for the unused-local marker.
- `DEF-214`: a call of a non-callable field or value reports the approved
  `E10309` (registry row and compile-fail golden added).
- `DEF-236`: a func-value call reports `E10308` for an arity mismatch.
- `DEF-239`: a zero-parameter free function reached with the dot spelling
  reports `E10308` instead of emitting a receiver argument.
- `DEF-241`: a callable field through a raw receiver renders the implied
  `->`.
- `DEF-254`: `--max-errors 0` is rejected as an invalid limit and writes no
  translation unit.
- `DEF-349`: a raw-pointer-to-integer cast routes through `uintptr_t`, so the
  sanctioned `usize(p)` spelling stays host-strict clean.
- `DEF-382`/`DEF-405`: explicit and inferred generic type arguments type a
  fitting literal before judging it, and an unfit literal reports `E10202`.
- `DEF-387`: a bare member read no longer borrows a free function's spelling;
  only the dot-call path reaches the function candidate.
- `DEF-389`: a module-path receiver resolves its callable field, the target
  module's free function, or the intrinsic `len`; a resolved borrowing method
  reports the ownership rule rather than `E10212`.
- `DEF-404`: the dot-call path accepts a static string literal for a `cstring`
  parameter exactly as the direct spelling does.
- `DEF-419`: type resolution honors a module's import aliases, including
  struct fields and globals, so `Array as A` reaches the declared instance.

`DEF-174`, `DEF-203`, and `DEF-240` were already Done in Phase 0; the batch
adds regression pins for their fixed behavior. `DEF-244` is already fixed by
the Phase 2 FFI declaration validation (`E10226` on an unused declaration),
and `DEF-388` is already fixed by the Phase 2 null-member rule (`E10212` on
`null.foo`); both are pinned by the batch's stale cases.

Verification: `make check` green (79 targets, 111 compile-fail goldens) and
`make verify: all legs passed`.

**Phase 3 review (emission).** The emission audit found two critical
accepted-source shapes:

- The raw-to-integer cast named `uintptr_t`, which the freestanding shim did
  not declare. The shim now defines the C99 pointer-sized integer, and the
  spec test pins a freestanding `-nostdlib` compile.
- An import-aliased global was re-resolved by the emitter without the module's
  alias binding. The shared folded resolver now applies import aliases, so
  globals, parameters, returns, locals, arrays, and struct fields all reach the
  declared type; the alias test covers every position.

The emission corpus diff otherwise contains only the intended symbol, arrow,
cast, literal-context, and diagnostic changes; rejected programs write no C,
and the accepted corpus builds under the strict legs.

**Phase 3 review (correctness/semantics).** The semantics review found three
criticals and six majors. Fixed:

- The freestanding shim declares `uintptr_t`; the raw cast builds there.
- Module-path cstring `.len` resolves the receiver's global/projection path in
  bodies and initializers and reports `E10010`; guarded forms stay accepted.
- A null-shaped callee reports `E10309`.
- Import aliases no longer override a type parameter of the same name.
- Reverse generic inference binds a literal from a later non-literal argument.
- Reserved user type names are escaped and a user `struct String` resolves to
  its own struct; `TyKind::Param` uses the same `p<len>_` C spelling as its key.
- Private target-module functions are not reachable through a module-path
  receiver.
- The qualified-arity pin now uses an actual `std.text.toCString` qualified
  call.

Report-only residuals, not DEF rows: a value-headed dot call on a zero-parameter
free function keeps the spec-pinned `E10212` while the Array/module-path arms
report `E10308`; a module-path value receiver is an aggregate lend and keeps
`E10038`; destination-context inference (`let x: u8 = identity(5)`) and generic
cstring literals remain `E10200` because the frozen token-conformance test pins
that route; and qualified module-value cstring indexing is a pre-existing crash
at base and head. The accepted module-path, alias, generic-literal, key-escape,
raw-call, and freestanding probes are ASAN/LSan clean.

Final Phase 3 verification: 30 spec cases and two impl pins; `make check` green
(79 targets, 111 goldens); `make verify: all legs passed`.

**Phase 3 review, final round.** The module-path cstring `.len` null check is
now evaluated in the expression walk where `if`, `&&`, `||`, `while`, and `?:`
narrowing facts are active. Null body/field/nested/initializer uses report one
`E10010`; guarded forms accept and run. The reverse-inference unfit case and
the `Clp1_T` parameter C spelling are pinned.

Report-only residuals: a user `struct String` works standalone but a program
that also imports `std.text` still resolves the shared stdlib handle and
rejects `E10036`; destination-context generic inference and generic `cstring`
literals remain `E10200` because the frozen token-conformance test pins that
route; zero-parameter value-headed dot calls keep `E10212` while Array/module
paths use `E10308`; a module-path aggregate receiver keeps `E10038`; and
qualified module-value cstring indexing is a pre-existing crash at base and
head.

Final verification: 30 spec cases, two impl pins, and the parameter C pin;
`make check` green (79 targets, 111 goldens); `make verify: all legs passed`.

**Phase 4 partial execution (borrow and loans).** Eight rows are fixed and
pinned by `def_zero_loans_spec_test.rs` (eight cases):

- `DEF-250`: a concrete read-only capture moved at a `let` reports one
  `E10401`; the lend-store `E10034` is suppressed only where the capture duty
  records the site.
- `DEF-252`: moving out of a `const` binding reports one `E10023`.
- `DEF-311`: the operand and guarded conditional projection roots bind and
  release on their own paths; the host ASan probes are clean.
- `DEF-370`: a move-only `S[2]` lend no longer warns `W10001`.
- `DEF-422`: an `unsafe` prefix on a raw member store supplies the unsafe
  context; the store builds and reads back.
- `DEF-434`: an overwritten element clears the literal element loan; the
  overwritten carrier runs clean.
- `DEF-463`: the raw-store right-side pointer binding is marked used, so
  `(*p).items = (*q).items` does not warn `W10000`.
- `DEF-524`: the maybe-use duty is keyed by the enclosing statement span, so
  two later uses in one generic body report twice.

Verification: `make check` green (80 targets, 111 compile-fail goldens) and
`make verify: all legs passed`. Follow-up round:

- `DEF-312`: a view loan now carries its source member paths; a write to a
  different owner member (`b.v` beside a loan through `b.w`) no longer reports
  `E10031`, while the same-member write still does.
- `DEF-444`: a `take` through a raw dereference in a loop condition reports
  `E10020` at the site, preventing the second-iteration double move.
- `DEF-491`: a conditional whose arms are frame-resident places or literals is
  judged as a frame value for a local destination, so the literal arm no longer
  adds `E10033`; the later owner write still reports `E10031`.

Verification after this round: `make check` green (80 targets, 111
compile-fail goldens) and `make verify: all legs passed`. Remaining Phase 4
rows: `DEF-484`, `DEF-490`, `DEF-492`.

The three remaining rows need a sound whole-program take-argument summary, not
a local callee suppression. An attempted call-site audit was reverted after the
phase reviews found two critical holes: a caller-retained slot could keep a
view into a callee-frame temporary (ASan heap-use-after-free), and dot/value
calls were not recorded, so `all()` over a partial call set accepted a dynamic
carrier. The next attempt must fail closed for temporary, parameter, and
unrecorded call forms, and key the judgment by declaration identity rather
than the simple function name.
