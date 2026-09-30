# Splitter

High-performance, lightweight expense splitting server and clients built with **Rust (Axum + SQLx SQLite)**, featuring administrative user management, multi-group expense splitting, debt simplification (min-cash-flow algorithm), idempotency-backed offline queuing, a minimalist web dashboard, and an Android client architecture.

---

## Features

- **Blazing Fast & Resource Efficient**: Written in Rust using Axum 0.7 and Tokio.
- **Embedded SQLite (Zero-Ops)**: Runs in WAL mode with connection pooling and automated schema migrations.
- **Authentication & RBAC**:
  - Secure password hashing using Argon2id.
  - JWT Bearer tokens with 30-day expiration.
  - Admin bootstrap via `.env` or auto-promotion of the first registered user.
  - Dedicated admin controls to view system stats, promote/demote users, and deactivate accounts.
- **Groups & Expenses**:
  - Multi-group support with customizable currency (`USD`, `INR`, `EUR`, `GBP`, etc.).
  - Add members by registered email.
  - Split options: Equal auto-split (with exact penny remainder distribution) or Exact amounts.
  - Category tags (Food, Transportation, Housing, Entertainment, General).
- **Automated Debt Simplification**:
  - Computes net member balances.
  - Applies a greedy bipartite min-cash-flow algorithm to reduce complex N-way debts to the absolute minimum direct payments.
  - One-click debt settlements.
- **Offline-First Queuing (Web & Android)**:
  - Client-generated UUID `idempotency_key` on mutating requests.
  - Server deduplication and response caching in `idempotency_records`.
  - Batch mutation sync endpoint (`POST /api/sync/batch`).
  - Incremental changes endpoint (`GET /api/sync/changes?group_id=<ID>&since=<TIMESTAMP>`).
- **Minimal Web Dashboard**:
  - Single-page application embedded and served directly at `http://127.0.0.1:8088/`.
  - Space Mono accents, crisp dark aesthetic, and browser offline queue support.
- **Android Client**:
  - WorkManager + Room offline queuing architecture with optimistic UI updates in `android/`.

---

## Quickstart

### 1. Requirements
- Rust 1.75+ (installed via `rustup`)

### 2. Configure Environment
```bash
cp .env.example .env
```

### 3. Run Server
```bash
cargo run
```
The server will start at `http://127.0.0.1:8088`. Open it in any browser to access the web interface.

### 4. Run Tests
```bash
cargo test
```

---

## API Reference

### Authentication
- `POST /api/auth/register`: `{ "email": "...", "password": "...", "display_name": "..." }`
- `POST /api/auth/login`: `{ "email": "...", "password": "..." }` -> `{ "token": "...", "user": { ... } }`
- `GET /api/auth/me`: Get authenticated user profile.

### Admin (`AdminUser` only)
- `GET /api/admin/users`: List all platform users.
- `PATCH /api/admin/users/:id/role`: `{ "is_admin": true|false }`
- `PATCH /api/admin/users/:id/status`: `{ "is_active": true|false }`
- `GET /api/admin/stats`: Get platform metrics (users, groups, expenses, total volume, settlements).

### Groups
- `POST /api/groups`: `{ "name": "...", "description": "...", "default_currency": "USD" }`
- `GET /api/groups`: List groups current user belongs to.
- `GET /api/groups/:id`: Group details and member list.
- `POST /api/groups/:id/members`: `{ "email": "..." }` Add member.
- `DELETE /api/groups/:id/members/:user_id`: Remove member.

### Expenses & Settlements
- `POST /api/groups/:id/expenses`: Create expense with equal or exact splits and optional `idempotency_key`.
- `GET /api/groups/:id/expenses`: List group expenses.
- `DELETE /api/groups/:id/expenses/:expense_id`: Delete expense.
- `GET /api/groups/:id/balances`: Get raw net balances and simplified min-cash-flow debts.
- `POST /api/groups/:id/settlements`: Record a settlement payment with optional `idempotency_key`.
- `GET /api/groups/:id/settlements`: List settlement history.

### Offline Sync
- `POST /api/sync/batch`: `{ "mutations": [ { "idempotency_key": "...", "action": "...", "group_id": "...", "payload": { ... } } ] }`
- `GET /api/sync/changes?group_id=<ID>&since=<TIMESTAMP>`: Pull delta changes since timestamp.
