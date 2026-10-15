# Graph Report - splitter  (2026-10-15)

## Corpus Check
- 48 files · ~56,745 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 449 nodes · 912 edges · 43 communities (24 shown, 19 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 41 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `67e2dae3`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Coordinator
- MainActivity.kt
- models.rs
- expenses.rs
- API Reference
- auth.rs
- admin.rs
- SyncQueueDao
- jwt.rs
- check_group_membership
- get_attachment_file
- get_changes
- setup_test_app
- ocr.rs
- SplitterDatabase
- manifest.json
- SyncQueueWorker
- bootstrap_admin_if_configured
- gradlew
- AppError
- AuthUser
- Group
- Context
- String
- URL
- Json
- Option
- Path
- splitter
- Result
- SqlitePool
- Self
- UserResponse
- UserResponse
- State
- Value
- Vec
- WebView

## God Nodes (most connected - your core abstractions)
1. `check_group_membership()` - 19 edges
2. `MainActivity` - 17 edges
3. `create_expense()` - 15 edges
4. `update_expense()` - 15 edges
5. `create_settlement()` - 14 edges
6. `Coordinator` - 13 edges
7. `get_attachment_file()` - 13 edges
8. `get_changes()` - 13 edges
9. `get_group_balances()` - 13 edges
10. `list_expenses()` - 13 edges

## Surprising Connections (you probably didn't know these)
- `setup_test_app()` --calls--> `init_pool()`  [INFERRED]
  tests/api_tests.rs → src/db.rs
- `setup_test_app()` --calls--> `create_app()`  [INFERRED]
  tests/api_tests.rs → src/lib.rs
- `.body` --calls--> `WebView`  [INFERRED]
  ios/Splitter/ContentView.swift → ios/Splitter/WebView.swift
- `.body` --calls--> `ContentView`  [INFERRED]
  ios/Splitter/SplitterApp.swift → ios/Splitter/ContentView.swift
- `check_attachment_access()` --calls--> `check_group_membership()`  [INFERRED]
  src/routes/attachments.rs → src/routes/groups.rs

## Import Cycles
- None detected.

## Communities (43 total, 19 thin omitted)

### Community 0 - "Coordinator"
Cohesion: 0.06
Nodes (33): Any, App, Bool, ContentView, .body, .serverConfigSheet, URL, SplitterApp (+25 more)

### Community 1 - "MainActivity.kt"
Cohesion: 0.10
Nodes (18): MainActivity, ConnectivityManager, WebChromeClient, WebViewClient, WebAppInterface, AppCompatActivity, Bundle, FileChooserParams (+10 more)

### Community 2 - "models.rs"
Cohesion: 0.12
Nodes (33): From, HashMap, Self, ActivityLog, ActivityLogDetail, Attachment, AttachmentResponse, Expense (+25 more)

### Community 3 - "expenses.rs"
Cohesion: 0.24
Nodes (30): E, compute_splits(), create_expense(), create_settlement(), CreateExpenseRequest, CreateSettlementRequest, delete_expense(), format_money_str() (+22 more)

### Community 4 - "API Reference"
Cohesion: 0.07
Nodes (28): 1. Dynamic Server URL Architecture, 2. Admin-Only Onboarding & Forced Password Reset, 3. Android Release Path, 4. iOS Release Path, Automated GitHub Actions CI Workflow, Connecting to a Custom Server, Deterministic Persistent Release Signing, Disabling Public Registration (+20 more)

### Community 5 - "auth.rs"
Cohesion: 0.21
Nodes (27): issue_token(), hash_password(), Result, String, verify_password(), AuthConfigResponse, AuthResponse, change_password() (+19 more)

### Community 6 - "admin.rs"
Cohesion: 0.23
Nodes (27): AdminUser, admin_reset_user_password(), AdminResetPasswordRequest, AdminStats, create_user_by_admin(), CreateUserByAdminRequest, CreateUserByAdminResponse, get_admin_stats() (+19 more)

### Community 7 - "SyncQueueDao"
Cohesion: 0.11
Nodes (11): Flow, OfflineQueueRepository, MutationStatus, APPLIED, FAILED, PENDING, SYNCING, QueuedMutationEntity (+3 more)

### Community 8 - "jwt.rs"
Cohesion: 0.15
Nodes (17): Error, FromRequestParts, IntoResponse, Parts, Rejection, Response, S, AdminUser (+9 more)

### Community 9 - "check_group_membership"
Cohesion: 0.28
Nodes (22): add_member(), AddMemberRequest, check_group_membership(), create_group(), CreateGroupRequest, get_group(), GroupDetail, GroupMemberDetail (+14 more)

### Community 10 - "get_attachment_file"
Cohesion: 0.23
Nodes (19): HeaderMap, Multipart, OcrResult, AttachmentQuery, check_attachment_access(), get_attachment_file(), get_attachment_info(), AppError (+11 more)

### Community 11 - "get_changes"
Cohesion: 0.21
Nodes (19): Query, batch_sync(), BatchSyncRequest, BatchSyncResponse, get_changes(), GroupChangesResponse, MutationResult, QueuedMutation (+11 more)

### Community 12 - "setup_test_app"
Cohesion: 0.15
Nodes (16): Client, Router, init_pool(), Result, SqlitePool, create_app(), SqlitePool, TempDir (+8 more)

### Community 13 - "ocr.rs"
Cohesion: 0.26
Nodes (16): Regex, extract_final_amount(), extract_merchant(), OcrResult, parse_line_amount(), parse_receipt_text(), Option, Path (+8 more)

### Community 14 - "SplitterDatabase"
Cohesion: 0.24
Nodes (6): SplitterApplication, Context, SplitterDatabase, Application, RoomDatabase, SyncQueueDao

### Community 15 - "manifest.json"
Cohesion: 0.22
Nodes (8): background_color, description, display, icons, name, short_name, start_url, theme_color

### Community 16 - "SyncQueueWorker"
Cohesion: 0.33
Nodes (4): Context, Result, SyncQueueWorker, CoroutineWorker

### Community 17 - "bootstrap_admin_if_configured"
Cohesion: 0.70
Nodes (4): bootstrap_admin_if_configured(), main(), Result, SqlitePool

### Community 18 - "gradlew"
Cohesion: 0.83
Nodes (3): gradlew script, die(), warn()

## Knowledge Gaps
- **34 isolated node(s):** `splitter`, `WebKit`, `background_color`, `description`, `display` (+29 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **19 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `log_activity()` connect `expenses.rs` to `jwt.rs`, `check_group_membership`?**
  _High betweenness centrality (0.176) - this node is a cross-community bridge._
- **Why does `UserResponse` connect `models.rs` to `auth.rs`, `admin.rs`?**
  _High betweenness centrality (0.119) - this node is a cross-community bridge._
- **Why does `WebView` connect `Coordinator` to `MainActivity.kt`?**
  _High betweenness centrality (0.117) - this node is a cross-community bridge._
- **Are the 11 inferred relationships involving `check_group_membership()` (e.g. with `check_attachment_access()` and `compute_splits()`) actually correct?**
  _`check_group_membership()` has 11 INFERRED edges - model-reasoned connections that need verification._
- **Are the 3 inferred relationships involving `create_expense()` (e.g. with `check_group_membership()` and `log_activity()`) actually correct?**
  _`create_expense()` has 3 INFERRED edges - model-reasoned connections that need verification._
- **Are the 3 inferred relationships involving `update_expense()` (e.g. with `check_group_membership()` and `log_activity()`) actually correct?**
  _`update_expense()` has 3 INFERRED edges - model-reasoned connections that need verification._
- **What connects `splitter`, `WebKit`, `background_color` to the rest of the system?**
  _34 weakly-connected nodes found - possible documentation gaps or missing edges._