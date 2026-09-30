package com.splitter.sync

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase

@Database(entities = [QueuedMutationEntity::class], version = 1, exportSchema = false)
abstract class SplitterDatabase : RoomDatabase() {
    abstract fun syncQueueDao(): SyncQueueDao

    companion object {
        @Volatile
        private var INSTANCE: SplitterDatabase? = null

        fun getInstance(context: Context): SplitterDatabase {
            return INSTANCE ?: synchronized(this) {
                val instance = Room.databaseBuilder(
                    context.applicationContext,
                    SplitterDatabase::class.java,
                    "splitter_local.db"
                ).build()
                INSTANCE = instance
                instance
            }
        }
    }
}
