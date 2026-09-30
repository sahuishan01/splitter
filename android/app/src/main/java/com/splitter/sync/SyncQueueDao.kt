package com.splitter.sync

import androidx.room.*
import kotlinx.coroutines.flow.Flow

@Dao
interface SyncQueueDao {
    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun enqueue(mutation: QueuedMutationEntity): Long

    @Query("SELECT * FROM queued_mutations WHERE status = 'PENDING' ORDER BY createdAt ASC")
    suspend fun getPendingMutations(): List<QueuedMutationEntity>

    @Query("SELECT COUNT(*) FROM queued_mutations WHERE status = 'PENDING'")
    fun observePendingCount(): Flow<Int>

    @Query("UPDATE queued_mutations SET status = :status WHERE id = :id")
    suspend fun updateStatus(id: Long, status: MutationStatus)

    @Query("UPDATE queued_mutations SET status = 'FAILED', lastError = :error, retryCount = retryCount + 1 WHERE id = :id")
    suspend fun markFailed(id: Long, error: String)

    @Query("DELETE FROM queued_mutations WHERE status = 'APPLIED'")
    suspend fun clearCompleted()

    @Delete
    suspend fun delete(mutation: QueuedMutationEntity)
}
