package com.splitter.app

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.http.SslError
import android.os.Bundle
import android.view.View
import android.webkit.JavascriptInterface
import android.webkit.SslErrorHandler
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import android.widget.ProgressBar
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.swiperefreshlayout.widget.SwipeRefreshLayout
import com.splitter.sync.SyncQueueWorker

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var progressBar: ProgressBar
    private lateinit var swipeRefresh: SwipeRefreshLayout
    private lateinit var offlineBanner: TextView
    private var networkCallback: ConnectivityManager.NetworkCallback? = null

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val rootLayout = FrameLayout(this).apply {
            fitsSystemWindows = false
            setBackgroundColor(0xFF0D1117.toInt())
        }

        // Apply clean status bar and navigation bar insets exactly once without duplicate gap
        ViewCompat.setOnApplyWindowInsetsListener(rootLayout) { view, windowInsets ->
            val statusBars = windowInsets.getInsets(WindowInsetsCompat.Type.statusBars())
            val navBars = windowInsets.getInsets(WindowInsetsCompat.Type.navigationBars())
            view.setPadding(0, statusBars.top, 0, navBars.bottom)
            WindowInsetsCompat.CONSUMED
        }

        val insetsController = WindowInsetsControllerCompat(window, window.decorView)
        insetsController.isAppearanceLightStatusBars = false
        insetsController.isAppearanceLightNavigationBars = false

        webView = WebView(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                cacheMode = WebSettings.LOAD_DEFAULT
                useWideViewPort = true
                loadWithOverviewMode = true
                mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
            }
            addJavascriptInterface(WebAppInterface(this@MainActivity), "AndroidNative")
        }

        val progressHeightPx = (3 * resources.displayMetrics.density).toInt().coerceAtLeast(6)
        progressBar = ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                progressHeightPx
            )
            max = 100
            progressDrawable?.setTint(0xFF58A6FF.toInt())
            visibility = View.GONE
        }

        offlineBanner = TextView(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            )
            text = "⚠️ Offline mode active — Actions are queued locally"
            setBackgroundColor(0xFFD29922.toInt())
            setTextColor(0xFF000000.toInt())
            setPadding(24, 16, 24, 16)
            textAlignment = View.TEXT_ALIGNMENT_CENTER
            visibility = View.GONE
        }

        swipeRefresh = SwipeRefreshLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            addView(webView)
            setOnRefreshListener {
                triggerSyncSafely()
                webView.reload()
            }
        }

        rootLayout.addView(swipeRefresh)
        rootLayout.addView(progressBar)
        rootLayout.addView(offlineBanner)
        setContentView(rootLayout)

        setupWebViewClients()
        setupNetworkMonitoring()

        val targetUrl = BuildConfig.API_BASE_URL
        webView.loadUrl(targetUrl)

        triggerSyncSafely()
    }

    private fun triggerSyncSafely() {
        try {
            SyncQueueWorker.scheduleImmediateSync(this)
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    private fun setupWebViewClients() {
        webView.webChromeClient = object : WebChromeClient() {
            override fun onProgressChanged(view: WebView?, newProgress: Int) {
                progressBar.progress = newProgress
                if (newProgress < 100) {
                    progressBar.visibility = View.VISIBLE
                } else {
                    progressBar.visibility = View.GONE
                    swipeRefresh.isRefreshing = false
                }
            }
        }

        webView.webViewClient = object : WebViewClient() {
            override fun onReceivedSslError(
                view: WebView?,
                handler: SslErrorHandler?,
                error: SslError?
            ) {
                val host = error?.url?.let { android.net.Uri.parse(it).host } ?: ""
                if (host.endsWith("algosculptor.com")) {
                    handler?.proceed()
                } else {
                    super.onReceivedSslError(view, handler, error)
                }
            }

            override fun onReceivedError(
                view: WebView?,
                request: WebResourceRequest?,
                error: WebResourceError?
            ) {
                if (request?.isForMainFrame == true) {
                    offlineBanner.visibility = View.VISIBLE
                }
            }

            override fun onPageFinished(view: WebView?, url: String?) {
                swipeRefresh.isRefreshing = false
                view?.evaluateJavascript(
                    "(function() { return localStorage.getItem('splitter_token') || ''; })();"
                ) { tokenResult ->
                    val cleanToken = tokenResult?.trim('"', ' ', '\'')
                    if (!cleanToken.isNullOrEmpty() && cleanToken != "null") {
                        val prefs = getSharedPreferences("splitter_prefs", Context.MODE_PRIVATE)
                        prefs.edit().putString("jwt_token", cleanToken).apply()
                    }
                }
            }
        }
    }

    private fun setupNetworkMonitoring() {
        val connectivityManager = getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager ?: return
        val request = NetworkRequest.Builder()
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .build()

        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                runOnUiThread {
                    offlineBanner.visibility = View.GONE
                    triggerSyncSafely()
                }
            }

            override fun onLost(network: Network) {
                runOnUiThread {
                    offlineBanner.visibility = View.VISIBLE
                }
            }
        }

        try {
            connectivityManager.registerNetworkCallback(request, callback)
            networkCallback = callback
        } catch (e: Exception) {
            e.printStackTrace()
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        networkCallback?.let {
            val cm = getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
            try {
                cm?.unregisterNetworkCallback(it)
            } catch (e: Exception) {
                e.printStackTrace()
            }
        }
        networkCallback = null
    }

    @Deprecated("Deprecated in Java")
    override fun onBackPressed() {
        if (webView.canGoBack()) {
            webView.goBack()
        } else {
            super.onBackPressed()
        }
    }

    inner class WebAppInterface(private val context: Context) {
        @JavascriptInterface
        fun shareText(title: String, text: String) {
            runOnUiThread {
                try {
                    val sendIntent = Intent().apply {
                        action = Intent.ACTION_SEND
                        putExtra(Intent.EXTRA_TITLE, title)
                        putExtra(Intent.EXTRA_SUBJECT, title)
                        putExtra(Intent.EXTRA_TEXT, text)
                        type = "text/plain"
                    }
                    val shareIntent = Intent.createChooser(sendIntent, title)
                    context.startActivity(shareIntent)
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }
        }
    }
}
