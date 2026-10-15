# Graph Report - splitter  (2026-10-13)

## Corpus Check
- 39 files · ~35,401 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 337 nodes · 736 edges · 33 communities (21 shown, 12 thin omitted)
- Extraction: 95% EXTRACTED · 5% INFERRED · 0% AMBIGUOUS · INFERRED: 35 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `74fbbe68`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- MainActivity.kt
- models.rs
- check_group_membership
- auth.rs
- expenses.rs
- SyncQueueDao
- admin_reset_user_password
- jwt.rs
- get_changes
- setup_test_app
- ocr.rs
- upload_attachment
- SplitterDatabase
- SyncQueueWorker
- bootstrap_admin_if_configured
- gradlew
- Result
- AppError
- Group
- Json
- Option
- splitter
- SqlitePool
- Self
- Path
- Vec
- State
- Value

## God Nodes (most connected - your core abstractions)
1. `check_group_membership()` - 18 edges
2. `MainActivity` - 16 edges
3. `create_expense()` - 15 edges
4. `update_expense()` - 15 edges
5. `create_settlement()` - 14 edges
6. `log_activity()` - 13 edges
7. `add_member()` - 13 edges
8. `get_group_balances()` - 13 edges
9. `list_expenses()` - 13 edges
10. `list_settlements()` - 13 edges

## Surprising Connections (you probably didn't know these)
- `setup_test_app()` --calls--> `init_pool()`  [INFERRED]
  tests/api_tests.rs → src/db.rs
- `setup_test_app()` --calls--> `create_app()`  [INFERRED]
  tests/api_tests.rs → src/lib.rs
- `get_group_balances()` --calls--> `compute_direct_debts()`  [INFERRED]
  src/routes/expenses.rs → src/services/settlement.rs
- `get_group_balances()` --calls--> `simplify_debts()`  [INFERRED]
  src/routes/expenses.rs → src/services/settlement.rs
- `create_expense()` --calls--> `log_activity()`  [INFERRED]
  src/routes/expenses.rs → src/services/activity.rs

## Import Cycles
- None detected.

## Communities (33 total, 12 thin omitted)

### Community 0 - "MainActivity.kt"
Cohesion: 0.12
Nodes (19): MainActivity, ConnectivityManager, WebChromeClient, WebViewClient, WebAppInterface, AppCompatActivity, Bundle, FileChooserParams (+11 more)

### Community 1 - "models.rs"
Cohesion: 0.12
Nodes (33): From, HashMap, Self, ActivityLog, ActivityLogDetail, Attachment, AttachmentResponse, Expense (+25 more)

### Community 2 - "check_group_membership"
Cohesion: 0.18
Nodes (28): E, Error, add_member(), AddMemberRequest, check_group_membership(), create_group(), CreateGroupRequest, get_group() (+20 more)

### Community 3 - "auth.rs"
Cohesion: 0.19
Nodes (23): AuthUser, hash_password(), Result, String, verify_password(), AuthResponse, change_password(), ChangePasswordRequest (+15 more)

### Community 4 - "expenses.rs"
Cohesion: 0.33
Nodes (25): compute_splits(), create_expense(), create_settlement(), CreateExpenseRequest, CreateSettlementRequest, delete_expense(), format_money_str(), get_group_balances() (+17 more)

### Community 5 - "SyncQueueDao"
Cohesion: 0.11
Nodes (11): Flow, OfflineQueueRepository, MutationStatus, APPLIED, FAILED, PENDING, SYNCING, QueuedMutationEntity (+3 more)

### Community 6 - "admin_reset_user_password"
Cohesion: 0.27
Nodes (20): AdminUser, Path, admin_reset_user_password(), AdminResetPasswordRequest, AdminStats, get_admin_stats(), list_users(), AppError (+12 more)

### Community 7 - "jwt.rs"
Cohesion: 0.19
Nodes (16): FromRequestParts, Parts, Rejection, Response, S, AdminUser, AuthUser, Claims (+8 more)

### Community 8 - "get_changes"
Cohesion: 0.21
Nodes (19): Query, batch_sync(), BatchSyncRequest, BatchSyncResponse, get_changes(), GroupChangesResponse, MutationResult, QueuedMutation (+11 more)

### Community 9 - "setup_test_app"
Cohesion: 0.16
Nodes (15): Client, Router, init_pool(), Result, SqlitePool, create_app(), SqlitePool, TempDir (+7 more)

### Community 10 - "ocr.rs"
Cohesion: 0.26
Nodes (16): Regex, extract_final_amount(), extract_merchant(), OcrResult, parse_line_amount(), parse_receipt_text(), Option, Path (+8 more)

### Community 11 - "upload_attachment"
Cohesion: 0.26
Nodes (15): IntoResponse, Multipart, get_attachment_file(), get_attachment_info(), AppError, AuthUser, Json, Path (+7 more)

### Community 12 - "SplitterDatabase"
Cohesion: 0.24
Nodes (6): SplitterApplication, Context, SplitterDatabase, Application, RoomDatabase, SyncQueueDao

### Community 13 - "SyncQueueWorker"
Cohesion: 0.33
Nodes (4): Context, SyncQueueWorker, CoroutineWorker, Result

### Community 14 - "bootstrap_admin_if_configured"
Cohesion: 0.70
Nodes (4): bootstrap_admin_if_configured(), main(), Result, SqlitePool

### Community 15 - "gradlew"
Cohesion: 0.83
Nodes (3): gradlew script, die(), warn()

## Knowledge Gaps
- **5 isolated node(s):** `splitter`, `APPLIED`, `FAILED`, `PENDING`, `SYNCING`
  These have ≤1 connection - possible missing edges or undocumented components.
- **12 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppError` connect `jwt.rs` to `check_group_membership`, `upload_attachment`?**
  _High betweenness centrality (0.130) - this node is a cross-community bridge._
- **Why does `log_activity()` connect `check_group_membership` to `expenses.rs`?**
  _High betweenness centrality (0.113) - this node is a cross-community bridge._
- **Why does `issue_token()` connect `jwt.rs` to `auth.rs`?**
  _High betweenness centrality (0.076) - this node is a cross-community bridge._
- **Are the 10 inferred relationships involving `check_group_membership()` (e.g. with `compute_splits()` and `create_expense()`) actually correct?**
  _`check_group_membership()` has 10 INFERRED edges - model-reasoned connections that need verification._
- **Are the 3 inferred relationships involving `create_expense()` (e.g. with `check_group_membership()` and `log_activity()`) actually correct?**
  _`create_expense()` has 3 INFERRED edges - model-reasoned connections that need verification._
- **Are the 3 inferred relationships involving `update_expense()` (e.g. with `check_group_membership()` and `log_activity()`) actually correct?**
  _`update_expense()` has 3 INFERRED edges - model-reasoned connections that need verification._
- **What connects `splitter`, `APPLIED`, `FAILED` to the rest of the system?**
  _5 weakly-connected nodes found - possible documentation gaps or missing edges._