package com.splitter.app

import android.annotation.SuppressLint
import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.os.Bundle
import android.view.View
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
import androidx.swiperefreshlayout.widget.SwipeRefreshLayout
import com.splitter.sync.SyncQueueWorker

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var progressBar: ProgressBar
    private lateinit var swipeRefresh: SwipeRefreshLayout
    private lateinit var offlineBanner: TextView

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val rootLayout = FrameLayout(this)
        rootLayout.setBackgroundColor(0xFF0D1117.toInt())

        webView = WebView(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                databaseEnabled = true
                cacheMode = WebSettings.LOAD_DEFAULT
                useWideViewPort = true
                loadWithOverviewMode = true
                mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
            }
        }

        progressBar = ProgressBar(this, null, android.R.attr.progressBarStyleHorizontal).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                8
            )
            max = 100
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
                SyncQueueWorker.scheduleImmediateSync(this@MainActivity)
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
        val connectivityManager = getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val request = NetworkRequest.Builder()
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .build()

        connectivityManager.registerNetworkCallback(request, object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                runOnUiThread {
                    offlineBanner.visibility = View.GONE
                    SyncQueueWorker.scheduleImmediateSync(this@MainActivity)
                }
            }

            override fun onLost(network: Network) {
                runOnUiThread {
                    offlineBanner.visibility = View.VISIBLE
                }
            }
        })
    }

    @Deprecated("Deprecated in Java")
    override fun onBackPressed() {
        if (webView.canGoBack()) {
            webView.goBack()
        } else {
            super.onBackPressed()
        }
    }
}
