# Graph Report - splitter  (2026-09-30)

## Corpus Check
- 34 files · ~13,547 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 242 nodes · 520 edges · 21 communities (19 shown, 2 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 20 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `f5f8638e`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- SyncQueueDao
- models.rs
- AppError
- jwt.rs
- AuthUser
- get_changes
- login
- update_user_role
- setup_test_app
- bootstrap_admin_if_configured
- splitter
- MainActivity.kt
- SplitterDatabase.kt
- gradlew
- Result
- SyncQueueWorker

## God Nodes (most connected - your core abstractions)
1. `AppError` - 29 edges
2. `AuthUser` - 19 edges
3. `check_group_membership()` - 15 edges
4. `MainActivity` - 13 edges
5. `create_expense()` - 13 edges
6. `create_settlement()` - 13 edges
7. `list_expenses()` - 13 edges
8. `list_settlements()` - 13 edges
9. `get_changes()` - 13 edges
10. `add_member()` - 12 edges

## Surprising Connections (you probably didn't know these)
- `setup_test_app()` --calls--> `init_pool()`  [INFERRED]
  tests/api_tests.rs → src/db.rs
- `setup_test_app()` --calls--> `create_app()`  [INFERRED]
  tests/api_tests.rs → src/lib.rs
- `get_group_balances()` --calls--> `simplify_debts()`  [INFERRED]
  src/routes/expenses.rs → src/services/settlement.rs
- `create_expense()` --calls--> `check_group_membership()`  [INFERRED]
  src/routes/expenses.rs → src/routes/groups.rs
- `create_settlement()` --calls--> `check_group_membership()`  [INFERRED]
  src/routes/expenses.rs → src/routes/groups.rs

## Import Cycles
- None detected.

## Communities (21 total, 2 thin omitted)

### Community 0 - "SyncQueueDao"
Cohesion: 0.11
Nodes (11): Flow, OfflineQueueRepository, MutationStatus, APPLIED, FAILED, PENDING, SYNCING, QueuedMutationEntity (+3 more)

### Community 1 - "models.rs"
Cohesion: 0.16
Nodes (23): From, Expense, ExpenseDetail, ExpenseSplit, ExpenseSplitDetail, Group, GroupBalanceSummary, GroupMemberRow (+15 more)

### Community 2 - "AppError"
Cohesion: 0.19
Nodes (25): Error, IntoResponse, Response, AppError, String, add_member(), AddMemberRequest, check_group_membership() (+17 more)

### Community 3 - "jwt.rs"
Cohesion: 0.29
Nodes (10): Parts, Rejection, S, Claims, get_jwt_secret(), issue_token(), Result, Self (+2 more)

### Community 4 - "AuthUser"
Cohesion: 0.34
Nodes (19): AuthUser, create_expense(), create_settlement(), CreateExpenseRequest, CreateSettlementRequest, delete_expense(), get_group_balances(), list_expenses() (+11 more)

### Community 5 - "get_changes"
Cohesion: 0.23
Nodes (17): Query, batch_sync(), BatchSyncRequest, BatchSyncResponse, get_changes(), GroupChangesResponse, MutationResult, QueuedMutation (+9 more)

### Community 6 - "login"
Cohesion: 0.22
Nodes (15): hash_password(), Result, String, verify_password(), AuthResponse, login(), LoginRequest, me() (+7 more)

### Community 7 - "update_user_role"
Cohesion: 0.27
Nodes (16): FromRequestParts, AdminUser, AdminStats, get_admin_stats(), list_users(), Json, Path, Result (+8 more)

### Community 8 - "setup_test_app"
Cohesion: 0.17
Nodes (12): Client, Router, init_pool(), Result, SqlitePool, create_app(), SqlitePool, TempDir (+4 more)

### Community 9 - "bootstrap_admin_if_configured"
Cohesion: 0.70
Nodes (4): bootstrap_admin_if_configured(), main(), Result, SqlitePool

### Community 13 - "MainActivity.kt"
Cohesion: 0.15
Nodes (15): MainActivity, ConnectivityManager, WebChromeClient, WebViewClient, AppCompatActivity, Bundle, Network, ProgressBar (+7 more)

### Community 14 - "SplitterDatabase.kt"
Cohesion: 0.24
Nodes (6): SplitterApplication, Context, SplitterDatabase, Application, RoomDatabase, SyncQueueDao

### Community 15 - "gradlew"
Cohesion: 0.83
Nodes (3): gradlew script, die(), warn()

### Community 20 - "SyncQueueWorker"
Cohesion: 0.33
Nodes (4): Context, SyncQueueWorker, CoroutineWorker, Result

## Knowledge Gaps
- **5 isolated node(s):** `splitter`, `APPLIED`, `FAILED`, `PENDING`, `SYNCING`
  These have ≤1 connection - possible missing edges or undocumented components.
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppError` connect `AppError` to `jwt.rs`, `AuthUser`, `get_changes`, `login`, `update_user_role`?**
  _High betweenness centrality (0.142) - this node is a cross-community bridge._
- **Why does `AuthUser` connect `AuthUser` to `AppError`, `jwt.rs`, `get_changes`, `login`, `update_user_role`?**
  _High betweenness centrality (0.049) - this node is a cross-community bridge._
- **Why does `get_changes()` connect `get_changes` to `AppError`, `AuthUser`?**
  _High betweenness centrality (0.036) - this node is a cross-community bridge._
- **Are the 7 inferred relationships involving `check_group_membership()` (e.g. with `create_expense()` and `create_settlement()`) actually correct?**
  _`check_group_membership()` has 7 INFERRED edges - model-reasoned connections that need verification._
- **What connects `splitter`, `APPLIED`, `FAILED` to the rest of the system?**
  _5 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `SyncQueueDao` be split into smaller, more focused modules?**
  _Cohesion score 0.10507246376811594 - nodes in this community are weakly interconnected._