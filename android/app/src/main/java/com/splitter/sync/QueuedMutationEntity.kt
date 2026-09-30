package com.splitter.sync

import androidx.room.Entity
import androidx.room.PrimaryKey
import java.util.UUID

enum class MutationStatus {
    PENDING,
    SYNCING,
    APPLIED,
    FAILED
}

@Entity(tableName = "queued_mutations")
data class QueuedMutationEntity(
    @PrimaryKey(autoGenerate = true)
    val id: Long = 0,
    val idempotencyKey: String = UUID.randomUUID().toString(),
    val action: String, // "create_expense" or "record_settlement"
    val groupId: String,
    val payloadJson: String,
    val createdAt: Long = System.currentTimeMillis(),
    val status: MutationStatus = MutationStatus.PENDING,
    val retryCount: Int = 0,
    val lastError: String? = null
)
