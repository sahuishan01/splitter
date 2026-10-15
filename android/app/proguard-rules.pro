# Preserve WebView JavaScriptInterface methods
-keepclassmembers class * {
    @android.webkit.JavascriptInterface <methods>;
}

# Preserve WebAppInterface inner class in MainActivity
-keepclassmembers class com.splitter.app.MainActivity$WebAppInterface {
    public *;
}

# WorkManager Worker instantiation
-keep class com.splitter.sync.SyncQueueWorker {
    public <init>(android.content.Context, androidx.work.WorkerParameters);
}

# Room Database and Entities
-keep class * extends androidx.room.RoomDatabase
-keep class com.splitter.sync.QueuedMutationEntity { *; }
-keep interface com.splitter.sync.SyncQueueDao { *; }

# OkHttp & Coroutines
-dontwarn okhttp3.**
-dontwarn okio.**
-dontwarn kotlinx.coroutines.**
