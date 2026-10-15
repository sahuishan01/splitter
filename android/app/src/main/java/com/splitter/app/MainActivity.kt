package com.splitter.app

import android.annotation.SuppressLint
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.Uri
import android.net.http.SslError
import android.os.Bundle
import android.view.View
import android.webkit.JavascriptInterface
import android.webkit.SslErrorHandler
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import android.widget.ProgressBar
import android.widget.TextView
import androidx.activity.OnBackPressedCallback
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.splitter.sync.SyncQueueWorker

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var progressBar: ProgressBar
    private lateinit var offlineBanner: TextView
    private var networkCallback: ConnectivityManager.NetworkCallback? = null
    private var fileUploadCallback: ValueCallback<Array<Uri>>? = null
    private var backPressedTime: Long = 0

    private val fileChooserLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        val uris = if (result.resultCode == Activity.RESULT_OK) {
            val intent = result.data
            WebChromeClient.FileChooserParams.parseResult(result.resultCode, intent)
        } else {
            null
        }
        fileUploadCallback?.onReceiveValue(uris)
        fileUploadCallback = null
    }

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
            setLayerType(View.LAYER_TYPE_NONE, null)
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(0xFF0F141C.toInt())
            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                cacheMode = WebSettings.LOAD_DEFAULT
                useWideViewPort = true
                loadWithOverviewMode = true
                offscreenPreRaster = true
                mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
            }
            isVerticalScrollBarEnabled = false
            isHorizontalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
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

        rootLayout.addView(webView)
        rootLayout.addView(progressBar)
        rootLayout.addView(offlineBanner)
        setContentView(rootLayout)

        offlineBanner.setOnClickListener {
            showServerConfigDialog()
        }

        setupWebViewClients()
        setupNetworkMonitoring()

        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                if (webView.canGoBack()) {
                    webView.goBack()
                    return
                }
                if (System.currentTimeMillis() - backPressedTime < 2000) {
                    isEnabled = false
                    onBackPressedDispatcher.onBackPressed()
                } else {
                    backPressedTime = System.currentTimeMillis()
                    android.widget.Toast.makeText(this@MainActivity, "Press back again to exit", android.widget.Toast.LENGTH_SHORT).show()
                }
            }
        })

        val targetUrl = getServerBaseUrl()
        webView.loadUrl(targetUrl)

        triggerSyncSafely()
    }

    private fun getServerBaseUrl(): String {
        val prefs = getSharedPreferences("splitter_prefs", Context.MODE_PRIVATE)
        val custom = prefs.getString("server_url", null)?.trim()
        if (!custom.isNullOrEmpty()) {
            return custom.removeSuffix("/")
        }
        return BuildConfig.API_BASE_URL.removeSuffix("/")
    }

    private fun isValidServerUrl(urlString: String): Boolean {
        return try {
            val uri = Uri.parse(urlString)
            val scheme = uri.scheme?.lowercase() ?: return false
            val host = uri.host?.lowercase() ?: return false
            when (scheme) {
                "https" -> true
                "http" -> host == "127.0.0.1" || host == "localhost"
                else -> false
            }
        } catch (e: Exception) {
            false
        }
    }

    private fun showServerConfigDialog() {
        val input = android.widget.EditText(this).apply {
            setText(getServerBaseUrl())
            hint = "https://splitter.algosculptor.com"
            setSingleLine(true)
        }
        androidx.appcompat.app.AlertDialog.Builder(this)
            .setTitle("Configure Server URL")
            .setMessage("Enter the URL of your Splitter backend:")
            .setView(input)
            .setPositiveButton("Connect") { _, _ ->
                var newUrl = input.text.toString().trim()
                if (newUrl.isNotEmpty()) {
                    if (!newUrl.startsWith("http://") && !newUrl.startsWith("https://")) {
                        newUrl = "https://$newUrl"
                    }
                    if (isValidServerUrl(newUrl)) {
                        val prefs = getSharedPreferences("splitter_prefs", Context.MODE_PRIVATE)
                        prefs.edit().putString("server_url", newUrl).apply()
                        webView.loadUrl(newUrl)
                    } else {
                        android.widget.Toast.makeText(this, "Invalid server URL. HTTPS required.", android.widget.Toast.LENGTH_SHORT).show()
                    }
                }
            }
            .setNegativeButton("Reset Default") { _, _ ->
                val prefs = getSharedPreferences("splitter_prefs", Context.MODE_PRIVATE)
                prefs.edit().remove("server_url").apply()
                webView.loadUrl(BuildConfig.API_BASE_URL)
            }
            .setNeutralButton("Cancel", null)
            .show()
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
                }
            }

            override fun onShowFileChooser(
                webView: WebView?,
                filePathCallback: ValueCallback<Array<Uri>>?,
                fileChooserParams: FileChooserParams?
            ): Boolean {
                fileUploadCallback?.onReceiveValue(null)
                fileUploadCallback = filePathCallback

                return try {
                    val intent = fileChooserParams?.createIntent() ?: Intent(Intent.ACTION_GET_CONTENT).apply {
                        type = "*/*"
                        addCategory(Intent.CATEGORY_OPENABLE)
                    }
                    fileChooserLauncher.launch(intent)
                    true
                } catch (e: Exception) {
                    fileUploadCallback?.onReceiveValue(null)
                    fileUploadCallback = null
                    false
                }
            }
        }

        webView.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(
                view: WebView?,
                request: WebResourceRequest?
            ): Boolean {
                val uri = request?.url ?: return false
                return handleUri(uri)
            }

            @Deprecated("Deprecated in Java")
            override fun shouldOverrideUrlLoading(view: WebView?, url: String?): Boolean {
                val uri = url?.let { Uri.parse(it) } ?: return false
                return handleUri(uri)
            }

            private fun handleUri(uri: Uri): Boolean {
                val scheme = uri.scheme?.lowercase() ?: return false
                val host = uri.host?.lowercase() ?: ""

                // Handle external WhatsApp links (api.whatsapp.com, wa.me, chat.whatsapp.com)
                if (host.contains("whatsapp.com") || host == "wa.me") {
                    return try {
                        val intent = Intent(Intent.ACTION_VIEW, uri).apply {
                            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                        }
                        startActivity(intent)
                        true
                    } catch (e: Exception) {
                        e.printStackTrace()
                        true
                    }
                }

                // If scheme is standard http/https for internal app domain or host, load inside WebView
                if (scheme == "http" || scheme == "https") {
                    val configuredServerHost = try { Uri.parse(getServerBaseUrl()).host?.lowercase() ?: "" } catch (e: Exception) { "" }
                    if (host.isEmpty() || host == configuredServerHost || host.endsWith("algosculptor.com") || host == "127.0.0.1" || host == "localhost") {
                        return false
                    }
                    // For other external http/https domains, open in external browser
                    return try {
                        val intent = Intent(Intent.ACTION_VIEW, uri).apply {
                            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                        }
                        startActivity(intent)
                        true
                    } catch (e: Exception) {
                        e.printStackTrace()
                        false
                    }
                }

                // Handle custom schemes: whatsapp://, intent://, mailto:, tel:, etc.
                return try {
                    if (scheme == "intent") {
                        val intent = Intent.parseUri(uri.toString(), Intent.URI_INTENT_SCHEME)
                        if (intent != null) {
                            val packageManager = packageManager
                            val info = packageManager.resolveActivity(intent, 0)
                            if (info != null) {
                                startActivity(intent)
                                return true
                            }
                            val fallbackUrl = intent.getStringExtra("browser_fallback_url")
                            if (!fallbackUrl.isNullOrEmpty()) {
                                webView.loadUrl(fallbackUrl)
                                return true
                            }
                        }
                    } else {
                        val intent = Intent(Intent.ACTION_VIEW, uri).apply {
                            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                        }
                        startActivity(intent)
                        return true
                    }
                    true
                } catch (e: Exception) {
                    e.printStackTrace()
                    true // Always consume non-http/https schemes so WebView does not trigger net::ERR_UNKNOWN_URL_SCHEME!
                }
            }

            override fun onReceivedSslError(
                view: WebView?,
                handler: SslErrorHandler?,
                error: SslError?
            ) {
                // Fail-Closed: Strictly reject invalid certificates to prevent MITM attacks
                handler?.cancel()
            }

            override fun onReceivedError(
                view: WebView?,
                request: WebResourceRequest?,
                error: WebResourceError?
            ) {
                val scheme = request?.url?.scheme?.lowercase() ?: ""
                if (scheme != "http" && scheme != "https") {
                    return
                }
                if (request?.isForMainFrame == true) {
                    offlineBanner.visibility = View.VISIBLE
                }
            }

            override fun onPageFinished(view: WebView?, url: String?) {
                view?.invalidate()
                view?.requestLayout()
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

    override fun onResume() {
        super.onResume()
        webView.onResume()
    }

    override fun onPause() {
        super.onPause()
        webView.onPause()
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
            return
        }
        if (System.currentTimeMillis() - backPressedTime < 2000) {
            super.onBackPressed()
        } else {
            backPressedTime = System.currentTimeMillis()
            android.widget.Toast.makeText(this, "Press back again to exit", android.widget.Toast.LENGTH_SHORT).show()
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

        @JavascriptInterface
        fun shareWhatsApp(text: String) {
            runOnUiThread {
                try {
                    val sendIntent = Intent().apply {
                        action = Intent.ACTION_SEND
                        putExtra(Intent.EXTRA_TEXT, text)
                        type = "text/plain"
                        `package` = "com.whatsapp"
                    }
                    context.startActivity(sendIntent)
                } catch (e: Exception) {
                    try {
                        val bizIntent = Intent().apply {
                            action = Intent.ACTION_SEND
                            putExtra(Intent.EXTRA_TEXT, text)
                            type = "text/plain"
                            `package` = "com.whatsapp.w4b"
                        }
                        context.startActivity(bizIntent)
                    } catch (e2: Exception) {
                        try {
                            val uri = Uri.parse("whatsapp://send?text=" + Uri.encode(text))
                            val intent = Intent(Intent.ACTION_VIEW, uri).apply {
                                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                            }
                            context.startActivity(intent)
                        } catch (e3: Exception) {
                            try {
                                val uri = Uri.parse("https://wa.me/?text=" + Uri.encode(text))
                                val intent = Intent(Intent.ACTION_VIEW, uri).apply {
                                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                                }
                                context.startActivity(intent)
                            } catch (e4: Exception) {
                                shareText("Share via Splitter", text)
                            }
                        }
                    }
                }
            }
        }

        @JavascriptInterface
        fun setServerUrl(url: String) {
            val clean = url.trim().removeSuffix("/")
            if (clean.isEmpty() || !isValidServerUrl(clean)) {
                return
            }
            val prefs = context.getSharedPreferences("splitter_prefs", Context.MODE_PRIVATE)
            prefs.edit().putString("server_url", clean).apply()
            runOnUiThread {
                val currentUrl = webView.url ?: ""
                if (!currentUrl.startsWith(clean)) {
                    webView.loadUrl(clean)
                }
            }
        }

        @JavascriptInterface
        fun getServerUrl(): String {
            return getServerBaseUrl()
        }
    }
}
