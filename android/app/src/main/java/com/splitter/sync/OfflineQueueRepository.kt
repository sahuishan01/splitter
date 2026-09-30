package com.splitter.sync

import android.content.Context
import kotlinx.coroutines.flow.Flow
import org.json.JSONObject
import java.util.UUID

class OfflineQueueRepository(
    private val context: Context,
    private val queueDao: SyncQueueDao
) {
    fun observePendingQueueSize(): Flow<Int> = queueDao.observePendingCount()

    suspend fun queueCreateExpense(
        groupId: String,
        description: String,
        amountCents: Long,
        paidByUserId: String,
        splitType: String = "EQUAL",
        participants: List<String>
    ): String {
        val idempotencyKey = UUID.randomUUID().toString()

        val payload = JSONObject().apply {
            put("description", description)
            put("amount_cents", amountCents)
            put("paid_by", paidByUserId)
            put("split_type", splitType)
            put("participants", org.json.JSONArray(participants))
            put("idempotency_key", idempotencyKey)
        }

        val mutation = QueuedMutationEntity(
            idempotencyKey = idempotencyKey,
            action = "create_expense",
            groupId = groupId,
            payloadJson = payload.toString()
        )

        queueDao.enqueue(mutation)
        SyncQueueWorker.scheduleImmediateSync(context)

        return idempotencyKey
    }

    suspend fun queueRecordSettlement(
        groupId: String,
        payerId: String,
        payeeId: String,
        amountCents: Long
    ): String {
        val idempotencyKey = UUID.randomUUID().toString()

        val payload = JSONObject().apply {
            put("payer_id", payerId)
            put("payee_id", payeeId)
            put("amount_cents", amountCents)
            put("idempotency_key", idempotencyKey)
        }

        val mutation = QueuedMutationEntity(
            idempotencyKey = idempotencyKey,
            action = "record_settlement",
            groupId = groupId,
            payloadJson = payload.toString()
        )

        queueDao.enqueue(mutation)
        SyncQueueWorker.scheduleImmediateSync(context)

        return idempotencyKey
    }
}
