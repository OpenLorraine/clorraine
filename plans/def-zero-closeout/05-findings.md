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

## Phase 2 — Null and Type Rules

## Phase 3 — Calls and Resolution

## Phase 4 — Borrow and Loans

## Phase 5 — Emission and ABI

## Phase 6 — Initializers and Carriers

## Phase 7 — Final Verification and Close
