package com.splitter.app

import android.app.Application
import com.splitter.sync.SplitterDatabase

class SplitterApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        try {
            SplitterDatabase.getInstance(this)
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }
}
