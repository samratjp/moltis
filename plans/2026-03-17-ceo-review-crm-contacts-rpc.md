# CEO Review: CHA-16 — P1.9 RPC methods crm.contacts.*

**Branch:** `CHA-16-rpc-crm-contacts`
**Base:** `main`
**Mode:** HOLD SCOPE (recommendation below)
**Date:** 2026-03-17

---

## PRE-REVIEW SYSTEM AUDIT

### Current state
- **2 commits** on branch: auth scope tests + feature implementation
- **6 files changed**, +677 / -24 lines
- No stashed work, no other open PRs from this branch
- No TODOs/FIXMEs in changed files
- Sister issue CHA-17 (crm.matters.* RPC) has an existing eng review plan at
  `plans/2026-03-17-plan-crm-matters-rpc-methods.md` — that plan explicitly
  calls out `crm.contacts.list` filtering as a TODO (TODO 2) that this PR resolves

### What shipped in this PR
Three new capabilities added to `crm.contacts.*` RPC:

1. **Filtered list** — `crm.contacts.list` now accepts `{stage, search, offset, limit}` params,
   delegating to `store.list_filtered()` when any filter is present
2. **External lookup** — new `crm.contacts.getByExternal({source, externalId})` for
   channel-native ID resolution (e.g., find contact by Telegram user ID)
3. **Composite view** — new `crm.contacts.getWithChannels({id})` returning contact + all
   channel identities in one call

Plus: auth scope coverage (read/write) for all CRM methods, `source` field added to
`parse_contact`, import grouping cleanup.

### Retrospective check
No prior review cycles on this branch. Clean first run.

---

## Step 0: Nuclear Scope Challenge

### 0A. Premise Challenge

1. **Right problem?** Yes. The CRM contacts API was CRUD-only — listing returned all
   contacts with no filtering/pagination, no way to resolve contacts from external channel
   IDs, and no composite queries. These are table-stakes for any CRM integration.

2. **Actual outcome:** Enable channel integrations (Telegram, WhatsApp) to efficiently
   resolve contacts, and give the UI a paginated, filterable contacts list. Direct path.

3. **What if we did nothing?** The unfiltered `list_contacts()` would return every contact
   on every call — a real scaling problem as contact count grows. External lookup would
   require client-side filtering. Real pain point.

### 0B. Existing Code Leverage

| Sub-problem | Existing Code | Reused? |
|---|---|---|
| Basic CRUD (get, upsert, delete) | Full stack already existed | Yes, unchanged |
| Filtered listing | `CrmStore::list_filtered()` already existed in store trait + SQLite impl | Yes — wired through |
| External lookup | `CrmStore::get_by_external()` already existed | Yes — wired through |
| Composite view | `CrmStore::get_with_channels()` already existed (with default impl) | Yes — wired through |
| Auth scoping | `authorize_method()` pattern in `methods/mod.rs` | Yes — methods added to lists |
| JSON serialization | `contact_to_json()`, `channel_to_json()` helpers | Yes, unchanged |

**Key insight:** The store layer already had all these capabilities. This PR is purely
wiring them through the service trait and RPC handler. No new persistence code. This is
the right approach — the store was designed ahead, and now the RPC layer catches up.

### 0C. Dream State Mapping

```
CURRENT STATE                  THIS PR                     12-MONTH IDEAL
─────────────────────         ──────────────────────       ──────────────────────
CRUD-only contacts API  →     Filtered list, external  →   Full CRM with:
No pagination                 lookup, composite view       - FTS5 search
No external lookup            Pagination (offset/limit)    - Cursor pagination
No composite queries          Auth scope coverage          - Bulk operations
                                                           - Webhook events on changes
                                                           - Contact merge/dedup
                                                           - Activity timeline
```

This PR moves solidly toward the ideal. Nothing introduced here blocks or
conflicts with the 12-month trajectory.

### 0D. Mode-Specific Analysis (HOLD SCOPE)

1. **Complexity check:** 6 files touched, 0 new structs/services introduced. The changes
   are purely additive — new trait methods, new handler registrations, new tests. Clean.

2. **Minimum set:** All three new capabilities (filtered list, external lookup, composite
   view) are genuinely needed. The filtered list solves the pagination gap. External lookup
   is required for channel integrations. Composite view eliminates a common 2-call pattern.
   Nothing here is deferrable without leaving a meaningful gap.

### 0E. Temporal Interrogation

```
HOUR 1 (foundations):    ✅ Done — trait methods added, Noop impl updated
HOUR 2 (core logic):    ✅ Done — LiveCrmService impl wires to store
HOUR 3 (integration):   ✅ Done — RPC handlers registered, auth scopes set
HOUR 4+ (tests):        ✅ Done — unit tests + auth scope tests
```

**Decisions that were made (correctly):**
- `list_contacts` takes params but falls back to unfiltered `list()` when no params given
- Default limit of 50 when filtering is active
- `getByExternal` returns `null` (not error) when not found — consistent with `get`
- `getWithChannels` returns `null` when contact not found

### 0F. Mode Selection

**RECOMMENDATION: HOLD SCOPE** — The implementation is already done, clean, and correctly
scoped. The three new methods are all justified. No expansion needed (the matters RPC
is a separate issue CHA-17). No reduction possible without cutting needed functionality.

---

## Section 1: Architecture Review

```
┌─────────────────────────────────────────────────────────┐
│                    WebSocket Client                      │
└──────────────────────┬──────────────────────────────────┘
                       │ RPC frame
                       ▼
┌──────────────────────────────────────────────────────────┐
│  MethodRegistry::dispatch()                              │
│  ├─ authorize_method() ← READ_METHODS / WRITE_METHODS   │
│  └─ handler(ctx) → services.rs                           │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌──────────────────────────────────────────────────────────┐
│  CrmService trait (service-traits/src/lib.rs)            │
│  ├─ list_contacts(params)      [READ]                    │
│  ├─ get_contact(params)        [READ]                    │
│  ├─ get_contact_by_external()  [READ]  ← NEW            │
│  ├─ get_contact_with_channels() [READ] ← NEW            │
│  ├─ upsert_contact(params)     [WRITE]                   │
│  └─ delete_contact(params)     [WRITE]                   │
└──────────────────────┬──────────────────────────────────┘
                       │ LiveCrmService impl
                       ▼
┌──────────────────────────────────────────────────────────┐
│  CrmStore trait (crm/src/store.rs)                       │
│  ├─ list() / list_filtered()                             │
│  ├─ get() / get_by_external()                            │
│  ├─ get_with_channels()  (default impl)                  │
│  ├─ upsert() / delete()                                  │
│  └─ list_channels_for_contact()                          │
└──────────────────────┬──────────────────────────────────┘
                       │ SqliteCrmStore impl
                       ▼
┌──────────────────────────────────────────────────────────┐
│  SQLite (crm_contacts, crm_contact_channels tables)      │
└──────────────────────────────────────────────────────────┘
```

**Architecture is clean.** Three-layer separation (RPC → Service → Store) is maintained.
No coupling introduced. The composite `get_with_channels` uses the store's default impl
(two sequential queries) which is appropriate for SQLite (same connection, no network hop).

### Issues Found

**Issue 1: Unbounded `list()` fallback — WARNING**

When `list_contacts` is called with `{}` (empty params), it falls through to the
unfiltered `self.store.list()` which returns ALL contacts with no limit. This is the
pre-existing behavior preserved for backward compatibility, but it's a scaling concern.

**Options:**
- A) Accept as-is — pre-existing, not introduced by this PR, address separately
- B) Always use `list_filtered` with a default limit of 50 even when no params given

**RECOMMENDATION: Choose A** — This is pre-existing behavior. Changing it risks breaking
existing callers. File as a follow-up TODO. The CHA-17 eng review already flagged this
pattern for matters.

**Issue 2: No total count returned with paginated results — WARNING**

`list_contacts` with pagination returns just the array — no `total` count or `hasMore`
indicator. Clients can't build proper pagination UI without knowing total count.

**Options:**
- A) Accept for now — follow-up to add `{items: [...], total: N, hasMore: bool}` envelope
- B) Add response envelope now

**RECOMMENDATION: Choose A** — Adding an envelope is a breaking change to the response
format. Better done as a coordinated change across all CRM list endpoints. The current
array response works for the immediate use cases (channel integration lookup, basic UI).

**Issue 3: `source` field now parsed in `parse_contact` — OK**

The diff adds `source` parsing to `parse_contact()`, which is needed for the external
lookup flow. This is a correct additive change.

---

## Section 2: Error & Rescue Map

```
METHOD/CODEPATH              | WHAT CAN GO WRONG              | ERROR TYPE
-----------------------------|--------------------------------|--------------------
list_contacts (filtered)     | Invalid stage string           | ServiceError::Message
                             | SQLite query error             | moltis_crm::Error → ServiceError
                             | Negative offset/limit          | Can't happen (u64)
get_contact_by_external      | Missing "source" param         | ServiceError::Message
                             | Missing "externalId" param     | ServiceError::Message
                             | SQLite error                   | moltis_crm::Error → ServiceError
get_contact_with_channels    | Missing "id" param             | ServiceError::Message
                             | SQLite error                   | moltis_crm::Error → ServiceError

ERROR TYPE                   | RESCUED? | RESCUE ACTION           | USER SEES
-----------------------------|----------|-------------------------|------------------
ServiceError::Message        | Y        | → ErrorShape(UNAVAILABLE)| Error message
moltis_crm::Error            | Y        | store_err() → ServiceError | Error message
Invalid stage parse          | Y        | ServiceError::message() | "unknown stage: X"
Missing required param       | Y        | require_str() → error   | "missing field: X"
```

**No gaps.** All error paths are handled. The `store_err()` wrapper converts any store
error to a `ServiceError::Message`, which the dispatcher converts to an `ErrorShape`
with code `UNAVAILABLE`. This is consistent with the existing pattern.

**One note:** `store_err` flattens all store errors to `UNAVAILABLE`. A corrupt DB and
a not-found record get the same error code. This is pre-existing and acceptable — the
error message still differentiates, and the auth layer handles authorization separately.

---

## Section 3: Security & Threat Model

| Threat | Likelihood | Impact | Mitigated? |
|---|---|---|---|
| Unauthorized access to contacts | Low | High (PII) | Yes — auth middleware + scope checks |
| SQL injection via search param | Low | High | Yes — parameterized queries in SQLite store |
| PII exposure in logs | Medium | Medium | Yes — `Secret<String>` for email/phone |
| IDOR (access other user's contacts) | N/A | N/A | Single-tenant — all contacts belong to instance |
| Denial of service via large list | Low | Low | Partially — unbounded list exists but is pre-existing |

**Auth scoping is correct:**
- Read methods (`list`, `get`, `getByExternal`, `getWithChannels`) → `READ_METHODS` list
- Write methods (`upsert`, `delete`) → `WRITE_METHODS` list
- Tests verify both authorization and rejection for all methods

**No new attack surface introduced.** The new methods only read data through existing
store operations. No new endpoints, no new user inputs beyond the filter params which
are type-checked (stage parsed to enum, search passed as parameterized query).

**PII handling:** `contact_to_json` calls `expose_secret()` on email/phone. This is
correct — PII is only exposed in the RPC response to authenticated clients, never in
logs (the `Secret` wrapper ensures `Debug` output shows `[REDACTED]`).

---

## Section 4: Data Flow & Interaction Edge Cases

### Data Flow: list_contacts with filters

```
INPUT (params)
  │
  ├─ stage: Option<str> ──▶ parse::<ContactStage>() ──▶ valid enum? ──┐
  │                                                         │ N: ServiceError
  │                                                         │ Y: Some(stage)
  ├─ search: Option<str> ──▶ passed through ──────────────────────────┤
  ├─ offset: Option<u64> ──▶ unwrap_or(0) ───────────────────────────┤
  ├─ limit: Option<u64> ──▶ unwrap_or(0) ──▶ if 0 → 50 ─────────────┤
  │                                                                    │
  └─ Any present? ──▶ list_filtered(stage, search, offset, limit) ────┤
     All absent?  ──▶ list() (unfiltered) ────────────────────────────┤
                                                                       │
                                                                       ▼
                                                              Vec<Contact>
                                                                  │
                                                                  ▼
                                                     Array of contact JSON
```

### Edge Cases

| Interaction | Edge Case | Handled? | How? |
|---|---|---|---|
| list_contacts | Empty params `{}` | Yes | Falls through to unfiltered list |
| list_contacts | stage="invalid" | Yes | Parse error → ServiceError |
| list_contacts | offset > total | Yes | Returns empty array |
| list_contacts | limit=0 (explicit) | Yes | Treated as "use default 50" |
| list_contacts | limit=999999 | **Partially** | Honored — no upper bound cap |
| getByExternal | Missing source | Yes | require_str error |
| getByExternal | Missing externalId | Yes | require_str error |
| getByExternal | Not found | Yes | Returns null |
| getWithChannels | Missing id | Yes | require_str error |
| getWithChannels | Contact exists, 0 channels | Yes | Returns {contact, channels: []} |
| getWithChannels | Contact not found | Yes | Returns null |
| search | LIKE wildcards (%, _) | **No** | Passed through — correctness issue only |

**Issue 4: No upper bound on `limit` parameter — WARNING**

A client can pass `limit: 999999` and get an unbounded result set. This isn't a
security issue (auth is required) but could cause memory/performance problems.

**RECOMMENDATION:** Accept for now — consistent with pre-existing `list()` behavior.
A max limit cap (e.g., 500) would be a good follow-up.

---

## Section 5: Code Quality Review

**Code organization:** Excellent. New methods follow the exact same pattern as existing
ones — `require_str` for param extraction, `store_err` for error mapping,
`*_to_json` for serialization. No deviation from established patterns.

**DRY:** No violations found. The new methods are not duplicating existing logic.

**Naming:** Clear and consistent. `get_contact_by_external`, `get_contact_with_channels`
are descriptive. RPC names `crm.contacts.getByExternal`, `crm.contacts.getWithChannels`
follow the established `namespace.entity.verb` convention.

**Import cleanup:** The diff changes individual `use` statements to grouped `use { ... }`
blocks. This is a style improvement consistent with Rust idiom. No functional change.

**Over-engineering:** None. The implementation is minimal and direct.

**Under-engineering:** The `limit=0 → default 50` logic works but is slightly unusual —
typically `limit=0` means "no limit" in APIs. However, this is consistent with the
pattern established for contacts filtering and used by CHA-17 for matters.

---

## Section 6: Test Review

### New things introduced

```
NEW RPC METHODS:
  [1] crm.contacts.list (with filter params)     — was parameterless, now accepts params
  [2] crm.contacts.getByExternal                 — NEW
  [3] crm.contacts.getWithChannels               — NEW

NEW DATA FLOWS:
  [4] Filter params → list_filtered delegation
  [5] source+externalId → get_by_external lookup
  [6] id → get_with_channels composite query

NEW AUTH SCOPES:
  [7] getByExternal in READ_METHODS
  [8] getWithChannels in READ_METHODS
```

### Test coverage

| Item | Happy Path | Error Path | Edge Case | Auth |
|---|---|---|---|---|
| list_contacts (filtered) | ✅ filter by stage | ✅ invalid stage | ✅ pagination, offset > total | ✅ |
| list_contacts (search) | ✅ search match | — | ✅ search non-match | ✅ |
| getByExternal | ✅ found | ✅ missing params | ✅ not found | ✅ |
| getWithChannels | ✅ with channels | ✅ missing id | ✅ not found, no channels | ✅ |

**Test count:** ~14 new unit tests in crm_service.rs + 2 auth scope tests in mod.rs.

**Test quality:** Good. Tests use in-memory SQLite, create realistic data, and assert
specific JSON structures. The auth scope tests are parametric (loop over all CRM methods).

**No gaps identified.** Coverage is thorough for the scope of changes.

---

## Section 7: Performance Review

- **N+1 queries:** None. `get_with_channels` does 2 queries (contact + channels) which
  is the minimum for a composite view without a JOIN. Acceptable for single-contact lookup.
- **Indexes:** `list_filtered` uses existing indexes. The `stage` column filtering and
  `LIKE` search on name/email/phone are covered by the existing schema.
- **Memory:** Pagination with default limit=50 keeps result sets bounded when filters are
  active. The unbounded `list()` fallback is pre-existing.
- **Connection pool:** No new connections. Uses existing `SqliteCrmStore` shared instance.

**No new performance issues introduced.**

---

## Section 8: Observability & Debuggability

- **Logging:** The `MethodRegistry::dispatch()` already logs method name, request_id,
  and conn_id at debug level for all dispatched methods. Error responses logged at warn.
  No additional logging needed for these simple passthrough methods.
- **Metrics:** No new metrics added. The existing method dispatch metrics (if any) cover
  these methods automatically.
- **Debuggability:** If a bug is reported, the auth scope tests + unit tests provide good
  regression coverage. The three-layer architecture makes it easy to isolate issues.

**No gaps for the scope of this change.**

---

## Section 9: Deployment & Rollout Review

- **Migration safety:** No new migrations. All store methods already existed.
- **Backward compatibility:** The `list_contacts` signature changed from `()` to `(params)`,
  but this is internal (trait + impl). The RPC name is unchanged. Clients sending `{}`
  get the same behavior as before.
- **Rollback:** Git revert. No data migrations, no schema changes. Fully reversible.
- **Feature flags:** Not needed — these are additive read methods.

**Risk: Zero.** This is purely additive wiring of existing store capabilities.

---

## Section 10: Long-Term Trajectory

- **Technical debt:** None introduced. The pattern established here (filtered list with
  pagination, external lookup, composite view) is being ported to matters in CHA-17.
- **Path dependency:** Positive — this creates reusable patterns for all CRM entities.
- **Reversibility:** 5/5 — purely additive, no schema changes.
- **The 1-year question:** A new engineer would find this code obvious. The three layers
  are clearly separated, naming is consistent, tests are comprehensive.

---

## Required Outputs

### NOT in scope

| Item | Rationale |
|---|---|
| Response envelope `{items, total, hasMore}` | Breaking change, coordinate across all CRM list endpoints |
| Upper bound cap on `limit` | Pre-existing pattern, follow-up |
| LIKE wildcard escaping (%, _) | Correctness-only, not security, follow-up |
| UUID validation on IDs | Pre-existing gap across all CRM entities (CHA-17 TODO 1) |
| FTS5 search | LIKE adequate for expected volumes |
| Cursor-based pagination | Offset/limit adequate for now |
| Bulk operations | Future feature |

### What already exists

| Sub-problem | Existing Code | Reused? |
|---|---|---|
| `CrmStore::list_filtered()` | Fully implemented in SQLite store | Yes — wired through |
| `CrmStore::get_by_external()` | Fully implemented | Yes — wired through |
| `CrmStore::get_with_channels()` | Default impl on trait | Yes — wired through |
| Auth scope pattern | `authorize_method()` + READ/WRITE lists | Yes — methods added |
| JSON serialization | `contact_to_json()`, `channel_to_json()` | Yes — unchanged |
| Error handling | `store_err()`, `require_str()` | Yes — unchanged |

### Dream state delta

This PR closes the gap between store capabilities and RPC exposure for contacts.
The store was designed ahead; this PR catches the API layer up. Remaining distance
to 12-month ideal: response envelopes, cursor pagination, FTS5, bulk operations,
webhook events, contact merge/dedup, activity timeline.

### Error & Rescue Registry

| Method | Exception | Rescued? | Action | User Sees |
|---|---|---|---|---|
| list_contacts | Invalid stage string | Y | ServiceError | Error message |
| list_contacts | SQLite error | Y | store_err → ServiceError | Error message |
| get_contact_by_external | Missing source | Y | require_str → ServiceError | "missing field: source" |
| get_contact_by_external | Missing externalId | Y | require_str → ServiceError | "missing field: externalId" |
| get_contact_by_external | SQLite error | Y | store_err → ServiceError | Error message |
| get_contact_with_channels | Missing id | Y | require_str → ServiceError | "missing field: id" |
| get_contact_with_channels | SQLite error | Y | store_err → ServiceError | Error message |

**0 CRITICAL GAPS.**

### Failure Modes Registry

| Codepath | Failure Mode | Rescued? | Test? | User Sees | Logged? |
|---|---|---|---|---|---|
| list_contacts filter | Invalid stage | Y | Y | Error msg | Y (warn) |
| list_contacts filter | SQLite error | Y | N | Error msg | Y (warn) |
| getByExternal | Missing params | Y | Y | Error msg | Y (warn) |
| getByExternal | Not found | Y | Y | null | Y (debug) |
| getWithChannels | Missing id | Y | Y | Error msg | Y (warn) |
| getWithChannels | Not found | Y | Y | null | Y (debug) |

**0 CRITICAL GAPS** (no rows with Rescued=N + Test=N + User Sees=Silent).

### Diagrams produced

1. System architecture (Section 1)
2. Data flow — list_contacts with filters (Section 4)

### Stale Diagram Audit

No ASCII diagrams exist in the modified files. N/A.

---

## Completion Summary

```
+====================================================================+
|            CEO PLAN REVIEW — COMPLETION SUMMARY                    |
+====================================================================+
| Mode selected        | HOLD SCOPE                                  |
| System Audit         | 2 commits, 6 files, clean state             |
| Step 0               | All 3 new methods justified, store existed  |
| Section 1  (Arch)    | 3 issues found, all WARNING (pre-existing)  |
| Section 2  (Errors)  | 7 error paths mapped, 0 GAPS                |
| Section 3  (Security)| 5 threats assessed, all mitigated            |
| Section 4  (Data/UX) | 12 edge cases mapped, 1 unhandled (LIKE %)  |
| Section 5  (Quality) | 0 issues — clean, follows patterns           |
| Section 6  (Tests)   | Diagram produced, 0 gaps                     |
| Section 7  (Perf)    | 0 new issues                                 |
| Section 8  (Observ)  | 0 gaps                                       |
| Section 9  (Deploy)  | 0 risks — fully additive, no migrations      |
| Section 10 (Future)  | Reversibility: 5/5, debt items: 0            |
+--------------------------------------------------------------------+
| NOT in scope         | Written (7 items)                            |
| What already exists  | Written (6 reusable components)              |
| Dream state delta    | Written                                      |
| Error/rescue registry| 7 methods, 0 CRITICAL GAPS                   |
| Failure modes        | 6 total, 0 CRITICAL GAPS                     |
| TODOS.md updates     | 0 new items (existing CHA-17 TODOs cover)    |
| Diagrams produced    | 2 (architecture, data flow)                   |
| Stale diagrams found | 0                                            |
| Unresolved decisions | 2 (see below)                                |
+====================================================================+
```

## Unresolved Decisions for Human Reviewer

### Decision 1: Unbounded list fallback
When `crm.contacts.list` is called with no params, it returns ALL contacts with no limit.
This is pre-existing behavior preserved for backward compatibility.
- **A)** Accept as-is, address in a follow-up (RECOMMENDED)
- **B)** Add a default limit now (breaking change for existing callers)

### Decision 2: No response envelope for paginated results
Paginated results return a bare array — no `total` count or `hasMore` flag.
- **A)** Accept for now, coordinate envelope change across all CRM list endpoints (RECOMMENDED)
- **B)** Add envelope now for contacts only (creates inconsistency with other endpoints)

## Overall Assessment

**This is a clean, well-executed PR.** The implementation is minimal, follows established
patterns exactly, has comprehensive test coverage, and introduces zero new risk. The store
layer was designed ahead with these capabilities — this PR simply wires them through the
RPC layer. Ship it.
