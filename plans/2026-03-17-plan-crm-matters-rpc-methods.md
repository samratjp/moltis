# Plan: CRM Matters RPC Methods (CHA-17)

**Issue:** CHA-17 — P1.10: RPC methods crm.matters.*
**Date:** 2026-03-17
**Review mode:** BIG CHANGE (scope held)

---

## Executive Summary

The `crm.matters.*` RPC methods exist with basic CRUD but lack filtering,
pagination, and search. This plan extends the existing methods to be
production-usable, following the established `crm.contacts.*` patterns.

**Files to modify (6):**
1. `crates/crm/src/store.rs` — extend trait with richer `list_matters_filtered` signature
2. `crates/crm/src/store_sqlite.rs` — implement new filters + pagination SQL
3. `crates/crm/src/store_memory.rs` — match new trait signature for tests
4. `crates/service-traits/src/lib.rs` — change `list_matters()` → `list_matters(params: Value)`
5. `crates/gateway/src/crm_service.rs` — parse filter params, call filtered store method
6. `crates/gateway/src/methods/services.rs` — pass `ctx.params` to `list_matters`

**New migration (1):**
7. `crates/crm/migrations/YYYYMMDDHHMMSS_matters_filter_indexes.sql` — add phase + practice_area indexes

---

## Architecture

```
  Client (WebSocket)
      │
      ▼
  ws.rs ─── GatewayFrame::Request { method: "crm.matters.list", params }
      │
      ▼
  methods/mod.rs ─── authorize → dispatch
      │
      ▼
  methods/services.rs ─── handler passes ctx.params to service
      │
      ▼
  CrmService::list_matters(params) ─── parse filters from JSON
      │
      ├─ params null/empty ──▶ store.list_matters() ──▶ all records (backward compat)
      │
      └─ params has filters ──▶ store.list_matters_filtered(
      │                            contact_id, status, phase, practice_area,
      │                            search, offset, limit
      │                         )
      ▼
  SqliteCrmStore ─── Dynamic SQL:
      SELECT ... FROM crm_matters WHERE 1=1
        [AND contact_id = ?]
        [AND status = ?]
        [AND phase = ?]
        [AND practice_area = ?]
        [AND (title LIKE ? OR description LIKE ?)]
      ORDER BY updated_at DESC
      LIMIT ? OFFSET ?
```

---

## Step 0: Scope Challenge

### What already exists

| Sub-problem | Existing Code | Reused? |
|---|---|---|
| Basic CRUD | Full stack: store → service → RPC | Yes, unchanged |
| Filter by contact_id + practice_area | `store.list_matters_filtered()` | Yes, extended |
| Filter by contact only | `store.list_matters_by_contact()` | Subsumed by extended filter |
| Pagination pattern | `store.list_filtered()` for contacts | Yes, pattern ported |
| Text search pattern | contacts `list_filtered` LIKE search | Yes, pattern ported |
| JSON serialization | `matter_to_json()` | Yes, unchanged |

### NOT in scope

| Item | Rationale |
|---|---|
| Status transition validation (e.g., archived → open blocked) | Separate concern; upsert accepts any valid status |
| `crm.matters.stats` aggregate endpoint | Nice-to-have, not blocking core usability |
| `crm.matters.get_with_interactions` composite query | Follow-up work, contacts have `get_with_channels` as precedent |
| Sort parameter (sortBy/sortOrder) | Default `updated_at DESC` is sufficient for now |
| FTS5 for matter search | LIKE search adequate for expected volumes |
| UUID validation on IDs | Pre-existing gap across all CRM entities |
| `crm.contacts.list` filtering (same gap) | Separate issue, same pattern |

---

## Section 1: Architecture Review — 5 issues

### Issue 1: `list_matters()` signature needs params
**Change:** `list_matters(&self) → list_matters(&self, params: Value)`
**Impact:** CrmService trait, NoopCrmService, LiveCrmService, RPC handler
**Risk:** Low — additive change

### Issue 2: Missing DB indexes for phase + practice_area
**Change:** New migration adding two indexes
**Risk:** None — `CREATE INDEX IF NOT EXISTS` is safe

### Issue 3: `list_matters_filtered` needs pagination
**Change:** Add `offset: u64, limit: u64` to store trait + impls
**Risk:** Low — follows contacts pattern exactly

### Issue 4: No search capability for matters
**Change:** Add `search: Option<&str>` with LIKE on title + description
**Risk:** Low — same LIKE pattern as contacts

### Issue 5: Backward compatibility for parameterless calls
**Change:** When params is null/empty, call unfiltered `list_matters()`
**Risk:** None — preserves existing behavior

---

## Section 2: Code Quality Review — 5 issues

### Issue 6: DRY — `list_matters()` redundant with `list_matters_filtered()`
**Fix:** Default impl in trait delegates to filtered method

### Issue 7: DRY — `list_matters_by_contact()` redundant with filtered
**Fix:** Default impl in trait delegates to `list_matters_filtered`

### Issue 8: `now_ms()` duplicated across crates
**Decision:** Skip — trivial function, cross-crate coupling not worth it

### Issue 9: Dynamic SQL string construction
**Decision:** Accept — established pattern, sqlx has no query builder

### Issue 10: No UUID validation on IDs
**Decision:** Skip — pre-existing gap, fix all entities together later

---

## Section 3: Test Review — 14 new tests needed

### Test Diagram

```
NEW CODEPATHS IN list_matters_filtered:
  ┌──────────────────────────┬────────────────────────────────────┐
  │ Filter Param             │ SQL Clause                         │
  ├──────────────────────────┼────────────────────────────────────┤
  │ contactId                │ AND contact_id = ?                 │
  │ status (NEW)             │ AND status = ?                     │
  │ phase (NEW)              │ AND phase = ?                      │
  │ practiceArea             │ AND practice_area = ?              │
  │ search (NEW)             │ AND (title LIKE ? OR desc LIKE ?)  │
  │ offset/limit (NEW)       │ LIMIT ? OFFSET ?                   │
  └──────────────────────────┴────────────────────────────────────┘

  Error paths:
    invalid status string  ──▶ parse error ──▶ ServiceError
    invalid phase string   ──▶ parse error ──▶ ServiceError
    invalid practiceArea   ──▶ parse error ──▶ ServiceError
```

### Required Tests

**Store layer (store_sqlite.rs):**
- T1: Filter by status (open/on_hold/closed/archived)
- T2: Filter by phase (intake/discovery/negotiation/resolution/review/closed)
- T3: Search on title (case-insensitive LIKE)
- T4: Search on description
- T5: Pagination (offset + limit, non-overlapping pages)
- T6: All filters combined
- T7: No filters returns all (backward compat via default impl)
- T13: Offset beyond total → empty result
- T14: LIKE special characters (%, _)

**Service layer (crm_service.rs):**
- T8: list_matters with filter params
- T9: list_matters with no params (backward compat)
- T10: list_matters with invalid status string → error
- T11: list_matters with empty search string

**Memory store (store_memory.rs):**
- T12: Verify in-memory filtering matches SQLite behavior

---

## Section 4: Performance Review — 4 issues

### Issue 11: Add indexes for phase + practice_area
New migration needed. SQLite CREATE INDEX is fast and non-blocking.

### Issue 12: LIKE search is O(n) scan
Acceptable for expected volumes. FTS5 available if needed later.

### Issue 13: No N+1 queries
Single SELECT per list call. Clean.

### Issue 14: Pagination defaults
When params provided, default to `limit=50, offset=0`.
When no params, return all (backward compat).

---

## Failure Modes

| Codepath | Failure Mode | Test? | Error Handling? | User Sees? |
|---|---|---|---|---|
| Filter parse | Invalid enum string | T10 | Yes — ServiceError | Clear error message |
| SQL query | SQLite error (corrupt DB) | No | Yes — store_err → ServiceError | "Internal error" |
| Search term | LIKE injection (%, _) | T14 | **No** — passes through to LIKE | Wrong results (not security issue) |
| Pagination | offset > total | T13 | Yes — returns empty vec | Empty array (correct) |
| Large result | No limit on parameterless call | No | **No** — returns all | Slow response |

**Critical gaps:** None. The LIKE special char handling is a correctness issue, not a security one (all values are parameterized). The unbounded parameterless response is a pre-existing condition.

---

## Implementation Checklist

1. [ ] New migration: `crates/crm/migrations/YYYYMMDDHHMMSS_matters_filter_indexes.sql`
2. [ ] Extend `CrmStore::list_matters_filtered` with status, phase, search, offset, limit
3. [ ] Add default impls for `list_matters` and `list_matters_by_contact` in trait
4. [ ] Update `SqliteCrmStore::list_matters_filtered` with new SQL clauses
5. [ ] Update `MemoryCrmStore::list_matters_filtered` to match
6. [ ] Change `CrmService::list_matters` signature to accept `params: Value`
7. [ ] Update `NoopCrmService::list_matters`
8. [ ] Update `LiveCrmService::list_matters` to parse filters and route
9. [ ] Update RPC handler in `services.rs` to pass `ctx.params`
10. [ ] Add 14 new tests across store + service layers
11. [ ] Run `cargo test`, `cargo +nightly-2025-11-30 fmt --all -- --check`, clippy

---

## Completion Summary

```
+====================================================================+
|            ENG REVIEW — COMPLETION SUMMARY                         |
+====================================================================+
| Step 0: Scope         | BIG CHANGE — 6 files, 0 new classes       |
| Architecture (S1)     | 5 issues found, all resolved               |
| Code Quality (S2)     | 5 issues found, 2 DRY fixes, 3 deferred   |
| Test Review (S3)      | Diagram produced, 14 test gaps identified   |
| Performance (S4)      | 4 issues found, 1 migration needed          |
| NOT in scope          | Written — 7 items deferred                  |
| What already exists   | Written — 6 reusable components mapped      |
| TODOS proposed        | 2 items (UUID validation, now_ms DRY)       |
| Failure modes         | 5 mapped, 0 critical gaps                   |
| Diagrams produced     | 2 (system arch, filter codepaths)           |
| Reversibility         | 5/5 — all additive changes                  |
+====================================================================+
```

---

## TODO Items

### TODO 1: UUID validation on CRM entity IDs
**What:** Validate that `id` fields in upsert operations are valid UUID v4 format.
**Why:** Currently any string is accepted as an ID, which could lead to data
integrity issues or confusing debugging.
**Pros:** Catches client bugs early with clear errors.
**Cons:** Breaking change for any clients passing non-UUID IDs (unlikely but possible).
**Context:** All CRM entity parsers (`parse_contact`, `parse_matter`, `parse_interaction`,
`parse_channel`) have this gap. Fix should be applied uniformly.
**Effort:** S (small)
**Priority:** P3
**Depends on:** Nothing

### TODO 2: `crm.contacts.list` filtering parity
**What:** Wire `list_filtered` into `crm.contacts.list` the same way this plan does for matters.
**Why:** Contacts list has the same "returns everything unfiltered" gap.
**Pros:** Feature parity across all CRM entities.
**Cons:** Scope creep if done in this PR.
**Context:** The contacts store already has `list_filtered(stage, search, offset, limit)`.
The service trait and RPC handler just don't pass params through.
**Effort:** S (identical pattern to this plan)
**Priority:** P2
**Depends on:** This plan (establishes the pattern)
