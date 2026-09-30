package com.splitter.sync

import android.content.Context
import androidx.work.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.TimeUnit

class SyncQueueWorker(
    appContext: Context,
    workerParams: WorkerParameters,
    private val queueDao: SyncQueueDao,
    private val okHttpClient: OkHttpClient,
    private val serverBaseUrl: String,
    private val getAuthToken: suspend () -> String?
) : CoroutineWorker(appContext, workerParams) {

    override suspend fun doWork(): Result {
        val pending = queueDao.getPendingMutations()
        if (pending.isEmpty()) {
            return Result.success()
        }

        val token = getAuthToken() ?: return Result.retry()

        // Construct JSON batch payload
        val batchJson = JSONObject()
        val mutationsArray = JSONArray()

        for (item in pending) {
            queueDao.updateStatus(item.id, MutationStatus.SYNCING)
            val mutObj = JSONObject().apply {
                put("idempotency_key", item.idempotencyKey)
                put("action", item.action)
                put("group_id", item.groupId)
                put("payload", JSONObject(item.payloadJson))
            }
            mutationsArray.put(mutObj)
        }
        batchJson.put("mutations", mutationsArray)

        val request = Request.Builder()
            .url("$serverBaseUrl/api/sync/batch")
            .addHeader("Authorization", "Bearer $token")
            .addHeader("Content-Type", "application/json")
            .post(batchJson.toString().toRequestBody("application/json".toMediaType()))
            .build()

        return try {
            val response = okHttpClient.newCall(request).execute()
            if (!response.isSuccessful) {
                // Revert status to PENDING for retry
                for (item in pending) {
                    queueDao.markFailed(item.id, "HTTP Error ${response.code}")
                }
                return Result.retry()
            }

            val respBody = response.body?.string() ?: ""
            val jsonResp = JSONObject(respBody)
            val results = jsonResp.optJSONArray("results") ?: JSONArray()

            val resultMap = mutableMapOf<String, String>()
            for (i in 0 until results.length()) {
                val resObj = results.getJSONObject(i)
                val key = resObj.getString("idempotency_key")
                val status = resObj.getString("status")
                resultMap[key] = status
            }

            for (item in pending) {
                val status = resultMap[item.idempotencyKey]
                if (status == "applied" || status == "already_processed") {
                    queueDao.updateStatus(item.id, MutationStatus.APPLIED)
                } else {
                    queueDao.markFailed(item.id, status ?: "Unknown status")
                }
            }

            // Cleanup successfully processed mutations
            queueDao.clearCompleted()
            Result.success()
        } catch (e: Exception) {
            for (item in pending) {
                queueDao.markFailed(item.id, e.message ?: "Network exception")
            }
            Result.retry()
        }
    }

    companion object {
        const val WORK_NAME = "splitter_offline_sync_work"

        fun scheduleImmediateSync(context: Context) {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build()

            val syncRequest = OneTimeWorkRequestBuilder<SyncQueueWorker>()
                .setConstraints(constraints)
                .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 15, TimeUnit.SECONDS)
                .build()

            WorkManager.getInstance(context).enqueueUniqueWork(
                WORK_NAME,
                ExistingWorkPolicy.REPLACE,
                syncRequest
            )
        }
    }
}
