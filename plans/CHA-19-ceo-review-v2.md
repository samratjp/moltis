# CHA-19: P1.12 Web UI — CRM Tab | CEO Review v2

**Mode: HOLD SCOPE** (eng review already right-sized this; focus on making it bulletproof)
**Branch:** CHA-19-web-ui-crm-tab
**Date:** 2026-03-17
**Prior reviews:** CEO v1 (SCOPE EXPANSION), Eng Review (BIG CHANGE, scope-corrected)

---

## System Audit Findings

- **Branch:** CHA-19-web-ui-crm-tab — only change is 4 lines in `templates.rs` (SpaRoutes, GonData, NavCounts fields added)
- **No stashes, no other PRs in flight**
- **Prior CEO review was wrong about REST** — recommended ~15 new REST handlers. The eng review correctly identified that every page in this codebase uses WebSocket RPC via `sendRpc()`. Zero pages use REST for CRUD. The channels page (1342 lines, closest analog) proves the RPC pattern works at scale.
- **CRM backend is complete:** 14 RPC methods, SQLite store with tests, PII handling, feature gating — all done

### Taste Calibration

**Well-designed patterns to emulate:**
1. `page-channels.js` — clean signal-based state, consistent RPC error handling, 1342 lines with clear component boundaries
2. `nav-counts.js` — elegant pub/sub with `updateNavCount(key, n)` + `gon.onChange()` for reactive badges
3. `router.js` — `registerPrefix()` is exactly right for `/crm` + `/crm/:contactId` sub-routing

**Anti-pattern to avoid:**
1. The CEO v1 review itself — recommending REST endpoints when the entire codebase uses RPC. This is what happens when you don't read the actual code before prescribing architecture.

---

## Step 0: Premise Validation + Mode Selection

### Is this the right problem?
**Yes, emphatically.** The CRM crate has 14 RPC methods, full domain types (Contact, Matter, Interaction, ContactChannel with 5 enums), a tested SQLite store, and PII protection — all invisible. Without a UI, this is dead code that adds compile time. Shipping the UI completes the feature.

### Do-nothing cost
Every contact auto-created from channel messages is invisible. Matters and interactions cannot be viewed or managed. The product feels like a chatbot, not a practice management tool. The backend work is wasted without this.

### CEO v1 → Eng Review → CEO v2 scope evolution

```
  CEO v1 (wrong)               Eng Review (right)           CEO v2 (validated)
  ┌──────────────────┐    ┌──────────────────────┐    ┌──────────────────────┐
  │ ~15 REST handlers │    │ 0 new Rust handlers  │    │ 0 new Rust handlers  │
  │ crm-store.js      │───▶│ Inline signals       │───▶│ Inline signals       │
  │ crm_routes.rs     │    │ sendRpc() only       │    │ sendRpc() only       │
  │ 16+ files          │    │ 9 files              │    │ 9 files              │
  │ EXPANSION          │    │ BIG CHANGE           │    │ HOLD SCOPE           │
  └──────────────────┘    └──────────────────────┘    └──────────────────────┘
```

### Mode Selection: HOLD SCOPE

The eng review already right-sized this to 9 files (4 new, 5 modified). The architecture decisions are correct. My job is to make the execution plan bulletproof — catch UX gaps, validate the implementation order, and ensure the first impression of CRM is excellent.

---

## Section 1: Architecture — Validated

The eng review's architecture is correct. I'll add one refinement:

### Corrected Architecture Diagram (RPC, not REST)

```
  Browser (SPA)                    Gateway (WebSocket)              SQLite
  ─────────────                   ──────────────────              ──────
  page-crm.js
    │
    ├─ signals: contacts, matters, interactions, loading, error
    │
    ├─ sendRpc("crm.contacts.list", {stage, search, offset, limit})
    │                                    → MethodRegistry.dispatch()
    │                                        → LiveCrmService.list_contacts()
    │                                            → SqliteCrmStore.list_filtered()
    │                              ←─── {ok: true, payload: [...]}
    ├─ contacts.value = res.payload
    │  (Preact auto-rerenders)
    │
    ├─ sendRpc("crm.contacts.get", {id})
    │                                    → LiveCrmService.get_contact()
    │                                        → SqliteCrmStore.get_with_channels()
    │                              ←─── {ok: true, payload: {contact, channels}}
    ├─ render ContactDetail with tabs
    │
    ├─ sendRpc("crm.contacts.upsert", {name, email, phone, stage, ...})
    │                                    → LiveCrmService.upsert_contact()
    │                              ←─── {ok: true, payload: {id}}
    ├─ re-fetch list + updateNavCount("crm", contacts.value.length)
```

**No issues found.** All dependencies are existing modules. Zero new coupling.

### Feature Gate (validated)

```
  Rust (templates.rs)                    JS (page-crm.js)
  ─────────────────                     ──────────────────
  crm_enabled: cfg!(feature = "crm")  → gon.get("crm_enabled")
                                          │
                                          ├─ true  → registerPrefix(routes.crm, ...)
                                          │          + show nav link
                                          └─ false → skip registration
                                                     + hide nav link
```

This matches `graphql_enabled` and `voice_enabled` patterns exactly.

---

## Section 2: Error & Rescue Map — Refined

The eng review's error map is correct but incomplete for the RPC path. Here's the complete map:

```
  RPC METHOD                    | WHAT CAN GO WRONG           | ERROR SHAPE
  ------------------------------|-----------------------------|--------------------------
  crm.contacts.list             | WS disconnected             | {ok:false, error:{code:"UNAVAILABLE"}}
                                | Invalid params              | {ok:false, error:{code:"INVALID_PARAMS"}}
                                | DB error                    | {ok:false, error:{code:"INTERNAL"}}
  crm.contacts.get              | Contact not found           | {ok:true, payload:null}
                                | Invalid ID                  | {ok:false, error:{code:"INVALID_PARAMS"}}
  crm.contacts.upsert           | Empty name                  | {ok:false, error:{code:"VALIDATION"}}
                                | Duplicate external_id       | {ok:false, error:{code:"CONSTRAINT"}}
  crm.contacts.delete           | Non-existent ID             | {ok:true} (idempotent)
                                | Matters cascade to NULL     | {ok:true} (by design)
  crm.matters.upsert            | Non-existent contact_id     | {ok:false, error:{code:"CONSTRAINT"}}
  crm.interactions.record       | Non-existent contact/matter | {ok:false, error:{code:"CONSTRAINT"}}

  ERROR SHAPE                   | RESCUED IN JS?  | USER SEES             | LOGGED?
  ------------------------------|-----------------|----------------------|--------
  {ok:false, code:UNAVAILABLE}  | Y (sendRpc)     | Error toast           | N (client)
  {ok:false, code:INVALID_PARAMS}| Y (res check)  | Validation toast      | N
  {ok:false, code:INTERNAL}     | Y (res check)   | "Something went wrong"| Y (server)
  {ok:false, code:CONSTRAINT}   | Y (res check)   | Specific error toast  | Y (server)
  {ok:true, payload:null}       | MUST HANDLE     | Redirect to list      | N
  Concurrent update             | N ← KNOWN GAP   | Silent overwrite      | N
```

**KNOWN GAP:** Concurrent update → silent overwrite. Acceptable for v1 single-tenant. Tracked.

**Implementation rule:** Every `sendRpc()` call MUST check `res?.ok` before accessing `res.payload`. The channels page does this consistently — follow that pattern exactly.

---

## Section 3: Security — No New Concerns

- **Auth surface:** Zero expansion. All RPC goes through auth-gated WebSocket. No new endpoints.
- **PII:** `contact_to_json()` already calls `expose_secret()` deliberately. Correct for single-tenant.
- **XSS:** Preact auto-escapes. MUST NOT use `innerHTML` or `dangerouslySetInnerHTML`.
- **LIKE wildcards:** `%` and `_` in search pass through unescaped. Cosmetic issue, not security. P3.

No action needed.

---

## Section 4: UX Edge Cases — Product-Level Concerns

This is where the CEO review adds value. The eng review covered technical edge cases; here are the product ones:

### First Impression UX (Critical)

The empty state is the first thing most users will see. If it's a blank page with "No contacts," the feature feels dead on arrival.

**Requirement:** Empty state MUST include:
1. A clear message: "Contacts are created automatically when someone messages you through a connected channel, or you can add one manually."
2. Two prominent CTAs: "Add Contact" button + "Connect a Channel" link (if no channels configured)
3. If channels ARE configured but no contacts exist yet: "Waiting for your first message..."

This is not a "delight opportunity" — it's a functional requirement. Without it, users will think the feature is broken.

### Contact Detail Page Layout

The eng review proposes tabs (channels, matters, interactions) on the contact detail page. This is correct. Recommended order:

```
  ┌────────────────────────────────────────────────────────┐
  │ ← Back to Contacts                                     │
  │                                                         │
  │  [Initials]  John Doe                    [Edit] [Delete]│
  │              Lead · john@example.com · +1234567890      │
  │                                                         │
  │  ┌─────────┬──────────┬──────────────┐                 │
  │  │ Channels│ Matters  │ Interactions │                  │
  │  ├─────────┴──────────┴──────────────┤                 │
  │  │                                    │                 │
  │  │  [Tab content here]                │                 │
  │  │                                    │                 │
  │  └────────────────────────────────────┘                 │
  └────────────────────────────────────────────────────────┘
```

### Stage Pipeline (Deferred but Important)

The eng review correctly deferred kanban. But the contact list SHOULD show stage distribution at a glance. Recommendation: add stage filter buttons (not a dropdown) above the list:

```
  [All (42)] [Lead (15)] [Prospect (8)] [Active (12)] [Inactive (5)] [Closed (2)]
```

This is a visual filter bar, not a kanban board. Achievable within the single `page-crm.js` file. This transforms a data table into a pipeline overview without any architectural cost.

---

## Section 5: Code Quality — Implementation Guardrails

### File Size Expectation

page-channels.js is 1342 lines for 6 channel types with modals. page-crm.js will have:
- Contact list with search/filter/pagination
- Contact detail with 3 tabs
- Contact form modal
- Matter form modal
- Interaction form modal
- Empty states for each section

**Estimated: 1500-2000 lines.** This is fine — matches the codebase pattern. Do NOT split prematurely.

### DRY Rules for Implementation

1. **Enum labels:** Define once as `const STAGE_LABELS = { lead: "Lead", prospect: "Prospect", ... }` with colors. Use everywhere.
2. **RPC wrapper:** Don't repeat the `sendRpc → check ok → update signal → handle error` pattern. Create a local helper:
   ```javascript
   function rpc(method, params, onSuccess, errorKey) {
     return sendRpc(method, params).then(res => {
       if (!res?.ok) { showToast(t(errorKey || "crm:errors.generic"), "error"); return; }
       onSuccess(res.payload);
     });
   }
   ```
3. **Timestamp display:** Use existing `time-format.js` — do NOT write custom formatting.
4. **Confirmation dialogs:** Use existing `requestConfirm()` from `ui.js`.

---

## Section 6: Test Review

The eng review's 13 E2E specs are comprehensive. I'll add the critical path test:

**Ship-confidence test (the one test that matters most):**
1. Navigate to `/crm` → empty state shown
2. Create contact "Alice" with email and phone → appears in list, badge shows 1
3. Click contact → detail page loads, channels tab empty
4. Create matter "Contract Review" → appears in matters tab
5. Record interaction "Initial call" → appears in interactions tab
6. Navigate back → contact in list with correct stage
7. Search "Alice" → found
8. Delete contact → gone from list, badge shows 0
9. No JS console errors throughout

This tests the entire happy path in one flow.

---

## Section 7: Performance — No Concerns

- N+1: Contact list is flat. Detail loads channels/matters/interactions via `Promise.all()`. Correct.
- Pagination: Backend supports offset/limit. Default 25. Server-side filtering only.
- Debounce: 300ms on search. Standard.
- NavCounts: `list_contacts().len()` is fine for v1. No user will have 1000+ contacts initially.

---

## Section 8: Observability — Minimal Additions

Since all data flows through existing RPC infrastructure, the existing gateway logging and tracing applies automatically. No new observability code needed in JS.

Server-side: RPC method dispatch already logs at `debug!` level. Error paths log at `warn!`.

---

## Section 9: Deployment — Trivial

- Pure additive change. Git revert is the rollback.
- No migrations. CRM tables exist.
- Feature-gated. If `crm` feature is disabled, the UI silently hides.
- Post-deploy: visit `/crm`, create a contact, verify badge.

---

## Section 10: Long-Term Trajectory

- **Reversibility: 5/5** — pure addition
- **Tech debt: 0** — follows all existing patterns
- **Path dependency: Positive** — establishes CRM UI foundation for contact-aware chat, document generation, client portal
- **Knowledge concentration: Low** — follows existing patterns, any engineer who can read page-channels.js can modify page-crm.js

---

## Decisions — Resolved

| # | Decision | Resolution | Rationale |
|---|----------|-----------|-----------|
| 1 | REST vs RPC | **RPC (sendRpc)** | Every page uses RPC. REST would be 15 new handlers duplicating existing functionality. Eng review is correct. |
| 2 | Top-level route | **`/crm` with registerPrefix** | CRM is core product, not settings. Matches `/projects`, `/monitoring`. |
| 3 | Feature gating | **`crm_enabled` in GonData** | Matches `graphql_enabled`, `voice_enabled` patterns exactly. |
| 4 | Single file | **Single `page-crm.js`** | Matches `page-channels.js` pattern. Split only if >3000 lines. |
| 5 | PII display | **Show directly (no masking v1)** | Single-tenant app. Authenticated user IS the data owner. Masking adds complexity without security benefit. |
| 6 | Store file | **Skip — inline signals** | Channels page uses inline signals. CRM data isn't shared across pages. |

---

## Open Questions for Human Review

### Q1: Stage filter bar vs dropdown
**Context:** The contact list needs stage filtering. Two options:
- **A) Filter buttons** above the list: `[All (42)] [Lead (15)] [Prospect (8)] ...` — more visual, shows pipeline distribution at a glance, slightly more code
- **B) Dropdown** select: compact, standard, but hides distribution data
**Recommendation:** A — the filter bar transforms a data table into a pipeline overview. Worth the 50 extra lines.

### Q2: Contact detail sub-routing
**Context:** When viewing a contact detail at `/crm/:contactId`, should the URL include the tab?
- **A) `/crm/:id`** only — tabs are client-side state, simpler
- **B) `/crm/:id/matters`** — deep-linkable tabs, more complex routing
**Recommendation:** A — matches how chat sessions work. Tab state is ephemeral.

### Q3: Nav position for CRM link
**Context:** Where in the sidebar does the CRM link go?
- **A) After Channels** — CRM is tightly related to channels (contacts come from channels)
- **B) After Chat** — CRM is a primary feature, should be prominent
- **C) In its own section** — separate from both chat and settings
**Recommendation:** A — CRM is the "who" to channels' "how." They belong together.

### Q4: Empty state for CRM-disabled builds
**Context:** If someone navigates to `/crm` but the feature is disabled (route not registered):
- **A) 404 page** — standard behavior when route doesn't match
- **B) Redirect to `/`** — seamless
**Recommendation:** A — the route simply won't be registered, so the existing 404 handling applies. No special code needed.

---

## What Already Exists (Validated)

| Component | Location | Status |
|-----------|----------|--------|
| CRM domain types | `crates/crm/src/types.rs` | Complete — 5 enums, 4 domain structs |
| CRM store trait | `crates/crm/src/store.rs` | Complete — full CRUD + filtered list |
| SQLite store | `crates/crm/src/store_sqlite.rs` | Complete — tested |
| LiveCrmService | `crates/gateway/src/crm_service.rs` | Complete — PII exposure, JSON serialization |
| 14 RPC methods | `crates/gateway/src/methods/services.rs` | Complete — all CRUD operations |
| Feature gating | `#[cfg(feature = "crm")]` | Complete — gateway + CLI |
| SpaRoutes.crm | `crates/web/src/templates.rs` | **Started** — field added, not yet wired |
| GonData.crm_enabled | `crates/web/src/templates.rs` | **Started** — field added, not yet wired |
| NavCounts.crm | `crates/web/src/templates.rs` | **Started** — field added, not yet wired |

---

## NOT in Scope

| Item | Rationale |
|------|-----------|
| REST API endpoints | RPC methods exist — REST is pure duplication |
| crm-store.js | Inline signals suffice (channels pattern) |
| Contact merge/dedup | Complex UX, future phase |
| Matter kanban | Future phase, v1 uses table with status badges |
| Optimistic locking | Backend change, v1 single-user is fine |
| Bulk import/export | Separate feature |
| E2E Playwright tests | Separate PR — requires E2E infra for CRM feature flag |
| French/Chinese translations | Create files with English keys; i18n pass is separate |

---

## Dream State Delta

```
  CURRENT STATE                  AFTER THIS PR                  12-MONTH IDEAL
  ┌────────────────┐    ┌──────────────────────────┐    ┌──────────────────────────┐
  │ CRM backend    │    │ Full CRM UI:             │    │ Practice management:     │
  │ complete but   │───▶│ - Contact list + search  │───▶│ - Intake forms           │
  │ invisible.     │    │ - Contact detail + tabs  │    │ - Doc generation         │
  │ 14 RPC methods │    │ - Matter CRUD            │    │ - Billing integration    │
  │ going unused.  │    │ - Interaction timeline   │    │ - Client portal          │
  │                │    │ - Stage filter bar       │    │ - Contact-aware AI chat  │
  │                │    │ - Nav badge              │    │ - Analytics dashboard    │
  └────────────────┘    └──────────────────────────┘    └──────────────────────────┘
```

This PR unlocks the entire CRM surface. Every future CRM feature builds on this foundation.

---

## Implementation Order (Validated from Eng Review)

```
  Phase 1: Wire the route (Rust + JS glue) — ~20 lines Rust, ~10 lines JS
  ├── 1. templates.rs: Wire crm_enabled + NavCounts.crm in build functions
  ├── 2. app.js: Import page-crm.js
  ├── 3. index.html: Add modulepreload + nav link
  ├── 4. nav-counts.js: Add crm badge ID
  └── 5. i18n.js: Register crm namespace

  Phase 2: Build the page (JS) — ~1500-2000 lines
  ├── 6. page-crm.js: Contact list with search + stage filter bar
  ├── 7. page-crm.js: Contact detail with channels/matters/interactions tabs
  ├── 8. page-crm.js: CRUD modals (contact, matter, interaction)
  └── 9. locales: en/crm.js (+ fr/zh with English fallbacks)

  Phase 3: Polish — ~200 lines
  ├── 10. Empty states (list + each detail tab)
  ├── 11. Stage color badges
  └── 12. Initials avatars (colored circles)
```

---

## Failure Modes Registry

```
  CODEPATH                 | FAILURE MODE          | RESCUED? | TEST? | USER SEES?      | LOGGED?
  -------------------------|-----------------------|----------|-------|-----------------|--------
  sendRpc contacts.list    | WS disconnected       | Y        | N     | Error toast     | N
  sendRpc contacts.list    | CRM disabled (Noop)   | Y        | N     | Empty list      | N
  sendRpc contacts.upsert  | Empty name            | Y        | N     | Error toast     | Y
  sendRpc contacts.upsert  | Duplicate external_id | Y        | N     | Error toast     | Y
  sendRpc contacts.get     | Non-existent ID       | MUST     | N     | Redirect list   | N
  sendRpc contacts.delete  | Has linked matters    | Y        | N     | Matters nulled  | N
  Contact search           | % _ wildcards         | N        | N     | Wider match     | N
  NavCounts CRM            | Feature disabled      | Y        | N     | Badge hidden    | N
  Page mount               | Route not registered  | Y        | N     | Tab not shown   | N
  Concurrent update        | Last-write-wins       | N ← GAP  | N     | Silent overwrite| N
```

**0 CRITICAL GAPS.** The concurrent update is a known limitation, not a silent failure — it's the expected behavior for v1 single-tenant.

---

## Completion Summary

```
  +====================================================================+
  |            CEO REVIEW v2 — COMPLETION SUMMARY                       |
  +====================================================================+
  | Mode selected        | HOLD SCOPE (eng review right-sized)          |
  | System Audit         | CEO v1 REST recommendation was wrong;        |
  |                      | eng review corrected to RPC — validated       |
  | Step 0               | Scope validated at 9 files, 0 new Rust CRUD  |
  | Section 1  (Arch)    | 0 issues — RPC pattern proven by channels    |
  | Section 2  (Errors)  | 10 error paths mapped, 0 CRITICAL GAPS       |
  | Section 3  (Security)| 0 issues — auth via existing WS gate          |
  | Section 4  (Data/UX) | 1 requirement added (empty state is critical) |
  | Section 5  (Quality) | 4 DRY rules for implementation               |
  | Section 6  (Tests)   | 1 ship-confidence test spec added            |
  | Section 7  (Perf)    | 0 issues                                     |
  | Section 8  (Observ)  | 0 additions needed — existing RPC logging     |
  | Section 9  (Deploy)  | 0 risks — pure addition                      |
  | Section 10 (Future)  | Reversibility: 5/5, debt: 0                  |
  +--------------------------------------------------------------------+
  | NOT in scope         | 8 items                                      |
  | What already exists  | 9 components (3 started in this branch)      |
  | Dream state delta    | written — unlocks entire CRM surface          |
  | Error/rescue registry| 10 paths, 0 CRITICAL GAPS                    |
  | Failure modes        | 10 total, 0 CRITICAL GAPS                    |
  | Open questions       | 4 for human reviewer                         |
  | Diagrams produced    | 3 (arch, feature gate, contact detail)       |
  | Stale diagrams found | 0                                            |
  +====================================================================+
```
