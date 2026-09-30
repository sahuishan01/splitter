package com.splitter.app

import android.app.Application
import com.splitter.sync.SplitterDatabase
import com.splitter.sync.SyncQueueWorker

class SplitterApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        SplitterDatabase.getInstance(this)
        SyncQueueWorker.scheduleImmediateSync(this)
    }
}
