# Staged Tests

Tests whose expected diagnostic cannot yet be produced under the frozen spec.
Each one names a feature or harness capability that does not exist in v1.
They live here, outside `tests/compile-fail/`, so the per-code coverage check in
[`check.sh`](../../check.sh) does not count them as live coverage. The codes are
listed in that script's exclusion list.

| File | Code | Why it is staged |
|---|---|---|
| `E90000_generic_staged.clo` | E90000 | The staged fallback now backs one trigger: an `Array` whose element type is not duplicable, whose owning-element drop glue is deferred. |
| `E90003_stores_contract.clo` | E90003 | `stores`/`transfers` callback contracts are deferred to v2; there is no frozen spelling to trigger them. |
| `E90004_pool_api.clo` | E90004 | Pools and arenas are a v2 standard-library API; no pool allocator exists to call. |
| `W10005_user_contract_table.clo` | W10005 | The table needs the undocumented `--contracts=` harness flag and a user contract table file, neither of which exists yet. |

When a staged feature lands or its harness support appears, move the file back to
`tests/compile-fail/`, confirm it still triggers its code, and drop the code from
the exclusion list in `check.sh`.
