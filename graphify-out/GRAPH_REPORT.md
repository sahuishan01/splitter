# Graph Report - splitter  (2026-09-30)

## Corpus Check
- 34 files · ~13,447 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 237 nodes · 510 edges · 20 communities (18 shown, 2 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 20 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `3734724e`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- SyncQueueDao
- models.rs
- check_group_membership
- AdminUser
- AuthUser
- get_changes
- login
- AppError
- setup_test_app
- bootstrap_admin_if_configured
- splitter
- MainActivity.kt
- SyncQueueWorker
- gradlew
- Result

## God Nodes (most connected - your core abstractions)
1. `AppError` - 29 edges
2. `AuthUser` - 19 edges
3. `check_group_membership()` - 15 edges
4. `create_expense()` - 13 edges
5. `create_settlement()` - 13 edges
6. `list_expenses()` - 13 edges
7. `list_settlements()` - 13 edges
8. `get_changes()` - 13 edges
9. `add_member()` - 12 edges
10. `get_group_balances()` - 12 edges

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

## Communities (20 total, 2 thin omitted)

### Community 0 - "SyncQueueDao"
Cohesion: 0.11
Nodes (11): Flow, OfflineQueueRepository, MutationStatus, APPLIED, FAILED, PENDING, SYNCING, QueuedMutationEntity (+3 more)

### Community 1 - "models.rs"
Cohesion: 0.16
Nodes (23): From, Expense, ExpenseDetail, ExpenseSplit, ExpenseSplitDetail, Group, GroupBalanceSummary, GroupMemberRow (+15 more)

### Community 2 - "check_group_membership"
Cohesion: 0.28
Nodes (20): add_member(), AddMemberRequest, check_group_membership(), create_group(), CreateGroupRequest, get_group(), GroupDetail, GroupMemberDetail (+12 more)

### Community 3 - "AdminUser"
Cohesion: 0.27
Nodes (12): FromRequestParts, Parts, Rejection, S, AdminUser, Claims, get_jwt_secret(), issue_token() (+4 more)

### Community 4 - "AuthUser"
Cohesion: 0.34
Nodes (19): AuthUser, create_expense(), create_settlement(), CreateExpenseRequest, CreateSettlementRequest, delete_expense(), get_group_balances(), list_expenses() (+11 more)

### Community 5 - "get_changes"
Cohesion: 0.23
Nodes (17): Query, batch_sync(), BatchSyncRequest, BatchSyncResponse, get_changes(), GroupChangesResponse, MutationResult, QueuedMutation (+9 more)

### Community 6 - "login"
Cohesion: 0.22
Nodes (15): hash_password(), Result, String, verify_password(), AuthResponse, login(), LoginRequest, me() (+7 more)

### Community 7 - "AppError"
Cohesion: 0.18
Nodes (19): Error, IntoResponse, Response, AppError, String, AdminStats, get_admin_stats(), list_users() (+11 more)

### Community 8 - "setup_test_app"
Cohesion: 0.17
Nodes (12): Client, Router, init_pool(), Result, SqlitePool, create_app(), SqlitePool, TempDir (+4 more)

### Community 9 - "bootstrap_admin_if_configured"
Cohesion: 0.70
Nodes (4): bootstrap_admin_if_configured(), main(), Result, SqlitePool

### Community 13 - "MainActivity.kt"
Cohesion: 0.19
Nodes (13): MainActivity, ConnectivityManager, WebChromeClient, WebViewClient, AppCompatActivity, Bundle, Network, ProgressBar (+5 more)

### Community 14 - "SyncQueueWorker"
Cohesion: 0.14
Nodes (10): SplitterApplication, Context, SplitterDatabase, Context, SyncQueueWorker, Application, CoroutineWorker, Result (+2 more)

### Community 15 - "gradlew"
Cohesion: 0.83
Nodes (3): gradlew script, die(), warn()

## Knowledge Gaps
- **5 isolated node(s):** `APPLIED`, `FAILED`, `PENDING`, `SYNCING`, `splitter`
  These have ≤1 connection - possible missing edges or undocumented components.
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppError` connect `AppError` to `check_group_membership`, `AdminUser`, `AuthUser`, `get_changes`, `login`?**
  _High betweenness centrality (0.148) - this node is a cross-community bridge._
- **Why does `AuthUser` connect `AuthUser` to `check_group_membership`, `AdminUser`, `get_changes`, `login`?**
  _High betweenness centrality (0.052) - this node is a cross-community bridge._
- **Why does `get_changes()` connect `get_changes` to `check_group_membership`, `AuthUser`, `AppError`?**
  _High betweenness centrality (0.037) - this node is a cross-community bridge._
- **Are the 7 inferred relationships involving `check_group_membership()` (e.g. with `create_expense()` and `create_settlement()`) actually correct?**
  _`check_group_membership()` has 7 INFERRED edges - model-reasoned connections that need verification._
- **Are the 2 inferred relationships involving `create_expense()` (e.g. with `check_group_membership()` and `batch_sync()`) actually correct?**
  _`create_expense()` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `APPLIED`, `FAILED`, `PENDING` to the rest of the system?**
  _5 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `SyncQueueDao` be split into smaller, more focused modules?**
  _Cohesion score 0.10507246376811594 - nodes in this community are weakly interconnected._