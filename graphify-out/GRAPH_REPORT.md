# Graph Report - splitter  (2026-10-02)

## Corpus Check
- 39 files · ~28,704 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 334 nodes · 728 edges · 43 communities (20 shown, 23 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 35 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `525ccb64`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- MainActivity.kt
- models.rs
- AppError
- check_group_membership
- auth.rs
- SyncQueueDao
- jwt.rs
- admin_reset_user_password
- get_changes
- setup_test_app
- ocr.rs
- upload_attachment
- SplitterDatabase
- SyncQueueWorker
- bootstrap_admin_if_configured
- gradlew
- Result
- splitter
- Self
- Path
- Vec
- Value
- AppError
- AuthUser
- Json
- Path
- Result
- SqlitePool
- State
- AuthUser
- Json
- Option
- Path
- Result
- SqlitePool
- State
- Value
- Vec

## God Nodes (most connected - your core abstractions)
1. `check_group_membership()` - 18 edges
2. `MainActivity` - 16 edges
3. `create_expense()` - 15 edges
4. `update_expense()` - 15 edges
5. `create_settlement()` - 14 edges
6. `add_member()` - 13 edges
7. `log_activity()` - 13 edges
8. `get_group_balances()` - 13 edges
9. `list_expenses()` - 13 edges
10. `list_settlements()` - 13 edges

## Surprising Connections (you probably didn't know these)
- `setup_test_app()` --calls--> `init_pool()`  [INFERRED]
  tests/api_tests.rs → src/db.rs
- `setup_test_app()` --calls--> `create_app()`  [INFERRED]
  tests/api_tests.rs → src/lib.rs
- `get_changes()` --calls--> `check_group_membership()`  [INFERRED]
  src/routes/sync.rs → src/routes/groups.rs
- `get_group_balances()` --calls--> `compute_direct_debts()`  [INFERRED]
  src/routes/expenses.rs → src/services/settlement.rs
- `get_group_balances()` --calls--> `simplify_debts()`  [INFERRED]
  src/routes/expenses.rs → src/services/settlement.rs

## Import Cycles
- None detected.

## Communities (43 total, 23 thin omitted)

### Community 0 - "MainActivity.kt"
Cohesion: 0.12
Nodes (19): MainActivity, ConnectivityManager, WebChromeClient, WebViewClient, WebAppInterface, AppCompatActivity, Bundle, FileChooserParams (+11 more)

### Community 1 - "models.rs"
Cohesion: 0.12
Nodes (34): From, HashMap, Self, ActivityLog, ActivityLogDetail, Attachment, AttachmentResponse, Expense (+26 more)

### Community 3 - "check_group_membership"
Cohesion: 0.18
Nodes (45): AppError, AuthUser, E, Group, Json, Option, Path, Result (+37 more)

### Community 4 - "auth.rs"
Cohesion: 0.20
Nodes (22): hash_password(), Result, String, verify_password(), AuthResponse, change_password(), ChangePasswordRequest, login() (+14 more)

### Community 5 - "SyncQueueDao"
Cohesion: 0.11
Nodes (11): Flow, OfflineQueueRepository, MutationStatus, APPLIED, FAILED, PENDING, SYNCING, QueuedMutationEntity (+3 more)

### Community 6 - "jwt.rs"
Cohesion: 0.17
Nodes (18): Error, FromRequestParts, IntoResponse, Parts, Rejection, Response, S, AdminUser (+10 more)

### Community 7 - "admin_reset_user_password"
Cohesion: 0.30
Nodes (18): AdminUser, admin_reset_user_password(), AdminResetPasswordRequest, AdminStats, get_admin_stats(), list_users(), AppError, Json (+10 more)

### Community 8 - "get_changes"
Cohesion: 0.20
Nodes (18): Query, batch_sync(), BatchSyncRequest, BatchSyncResponse, get_changes(), MutationResult, QueuedMutation, AppError (+10 more)

### Community 9 - "setup_test_app"
Cohesion: 0.16
Nodes (15): Client, Router, init_pool(), Result, SqlitePool, create_app(), SqlitePool, TempDir (+7 more)

### Community 10 - "ocr.rs"
Cohesion: 0.26
Nodes (16): Regex, extract_final_amount(), extract_merchant(), OcrResult, parse_line_amount(), parse_receipt_text(), Option, Path (+8 more)

### Community 11 - "upload_attachment"
Cohesion: 0.29
Nodes (14): Multipart, get_attachment_file(), get_attachment_info(), AppError, AuthUser, Json, Path, Result (+6 more)

### Community 12 - "SplitterDatabase"
Cohesion: 0.24
Nodes (6): SplitterApplication, Context, SplitterDatabase, Application, RoomDatabase, SyncQueueDao

### Community 13 - "SyncQueueWorker"
Cohesion: 0.40
Nodes (3): Context, SyncQueueWorker, CoroutineWorker

### Community 14 - "bootstrap_admin_if_configured"
Cohesion: 0.70
Nodes (4): bootstrap_admin_if_configured(), main(), Result, SqlitePool

### Community 15 - "gradlew"
Cohesion: 0.83
Nodes (3): gradlew script, die(), warn()

## Knowledge Gaps
- **5 isolated node(s):** `splitter`, `APPLIED`, `FAILED`, `PENDING`, `SYNCING`
  These have ≤1 connection - possible missing edges or undocumented components.
- **23 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AttachmentResponse` connect `models.rs` to `upload_attachment`?**
  _High betweenness centrality (0.101) - this node is a cross-community bridge._
- **Are the 10 inferred relationships involving `check_group_membership()` (e.g. with `compute_splits()` and `create_expense()`) actually correct?**
  _`check_group_membership()` has 10 INFERRED edges - model-reasoned connections that need verification._
- **What connects `splitter`, `APPLIED`, `FAILED` to the rest of the system?**
  _5 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `MainActivity.kt` be split into smaller, more focused modules?**
  _Cohesion score 0.12477718360071301 - nodes in this community are weakly interconnected._
- **Should `models.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.12012012012012012 - nodes in this community are weakly interconnected._
- **Should `SyncQueueDao` be split into smaller, more focused modules?**
  _Cohesion score 0.10507246376811594 - nodes in this community are weakly interconnected._