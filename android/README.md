# Splitter Android Client Architecture & Offline Queuing

This module provides the offline-first queuing architecture for the Splitter Android client.

## Core Components

1. **`QueuedMutationEntity`**: Room database entity storing pending mutations (`create_expense`, `record_settlement`) along with a client-generated UUID `idempotencyKey`.
2. **`SyncQueueDao`**: Room DAO to query, update, and clear queued actions.
3. **`SyncQueueWorker`**: Android `WorkManager` CoroutineWorker that runs when network connectivity is available (`NetworkType.CONNECTED`). It flushes pending mutations to the server's `POST /api/sync/batch` endpoint.
4. **`OfflineQueueRepository`**: Higher-level repository that optimistically enqueues mutations locally, schedules `SyncQueueWorker`, and exposes `observePendingQueueSize()` Flow for the UI badge/banner.

## Server Synchronization Contract

- Every mutating action generates a unique UUID `idempotency_key`.
- Even if network drops mid-request or `WorkManager` retries multiple times, the server detects duplicate idempotency keys and returns the original cached response without duplicate transactions.
- Batch mutations are sent to `POST /api/sync/batch`.
- Delta changes are pulled via `GET /api/sync/changes?group_id=<ID>&since=<TIMESTAMP>`.

## Gradle Dependencies (Add to `app/build.gradle.kts`)

```kotlin
dependencies {
    // Room
    val roomVersion = "2.6.1"
    implementation("androidx.room:room-runtime:$roomVersion")
    implementation("androidx.room:room-ktx:$roomVersion")
    ksp("androidx.room:room-compiler:$roomVersion")

    // WorkManager
    val workVersion = "2.9.1"
    implementation("androidx.work:work-runtime-ktx:$workVersion")

    // OkHttp & Coroutines
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
}
```
