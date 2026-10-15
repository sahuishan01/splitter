import SwiftUI
import WebKit

struct WebView: UIViewRepresentable {
    @Binding var currentUrl: URL
    @Binding var isLoading: Bool
    @Binding var loadFailed: Bool
    var onShowServerConfig: () -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(self)
    }

    func makeUIView(context: Context) -> WKWebView {
        let contentController = WKUserContentController()
        contentController.add(context.coordinator, name: "setServerUrl")
        contentController.add(context.coordinator, name: "shareText")
        contentController.add(context.coordinator, name: "shareWhatsApp")

        let config = WKWebViewConfiguration()
        config.userContentController = contentController
        config.allowsInlineMediaPlayback = true
        config.mediaTypesRequiringUserActionForPlayback = []

        let webView = WKWebView(frame: .zero, configuration: config)
        webView.navigationDelegate = context.coordinator
        webView.uiDelegate = context.coordinator
        webView.isOpaque = false
        webView.backgroundColor = UIColor(red: 13/255.0, green: 17/255.0, blue: 23/255.0, alpha: 1.0)
        webView.scrollView.backgroundColor = webView.backgroundColor
        webView.scrollView.showsHorizontalScrollIndicator = false
        webView.scrollView.alwaysBounceHorizontal = false

        // Pull to refresh
        let refreshControl = UIRefreshControl()
        refreshControl.tintColor = UIColor(red: 88/255.0, green: 166/255.0, blue: 255/255.0, alpha: 1.0)
        refreshControl.addTarget(context.coordinator, action: #selector(Coordinator.handleRefresh(_:)), for: .valueChanged)
        webView.scrollView.refreshControl = refreshControl

        context.coordinator.webView = webView
        webView.load(URLRequest(url: currentUrl))

        return webView
    }

    func updateUIView(_ uiView: WKWebView, context: Context) {
        if uiView.url != currentUrl && !context.coordinator.isHandlingNavigation {
            uiView.load(URLRequest(url: currentUrl))
        }
    }

    class Coordinator: NSObject, WKNavigationDelegate, WKUIDelegate, WKScriptMessageHandler {
        var parent: WebView
        weak var webView: WKWebView?
        var isHandlingNavigation = false

        init(_ parent: WebView) {
            self.parent = parent
        }

        @objc func handleRefresh(_ sender: UIRefreshControl) {
            webView?.reload()
        }

        // WKScriptMessageHandler
        func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
            switch message.name {
            case "setServerUrl":
                if let urlString = message.body as? String, let cleanUrl = URL(string: urlString.trimmingCharacters(in: .whitespacesAndNewlines)) {
                    UserDefaults.standard.set(cleanUrl.absoluteString, forKey: "server_url")
                    DispatchQueue.main.async {
                        self.parent.currentUrl = cleanUrl
                        self.webView?.load(URLRequest(url: cleanUrl))
                    }
                }
            case "shareText":
                if let text = message.body as? String {
                    share(items: [text])
                }
            case "shareWhatsApp":
                if let text = message.body as? String {
                    shareWhatsApp(text: text)
                }
            default:
                break
            }
        }

        private func share(items: [Any]) {
            guard let windowScene = UIApplication.shared.connectedScenes.first as? UIWindowScene,
                  let rootVC = windowScene.windows.first?.rootViewController else { return }
            let activityVC = UIActivityViewController(activityItems: items, applicationActivities: nil)
            rootVC.present(activityVC, animated: true)
        }

        private func shareWhatsApp(text: String) {
            let encoded = text.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? ""
            if let whatsappURL = URL(string: "whatsapp://send?text=\(encoded)"), UIApplication.shared.canOpenURL(whatsappURL) {
                UIApplication.shared.open(whatsappURL)
            } else if let webURL = URL(string: "https://wa.me/?text=\(encoded)") {
                UIApplication.shared.open(webURL)
            } else {
                share(items: [text])
            }
        }

        // Navigation Management
        func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction, decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
            guard let url = navigationAction.request.url else {
                decisionHandler(.allow)
                return
            }

            let scheme = url.scheme?.lowercased() ?? ""
            let host = url.host?.lowercased() ?? ""

            // WhatsApp link handling
            if host.contains("whatsapp.com") || host == "wa.me" || scheme == "whatsapp" {
                if UIApplication.shared.canOpenURL(url) {
                    UIApplication.shared.open(url)
                }
                decisionHandler(.cancel)
                return
            }

            // Check if navigation is within the configured Splitter backend host
            let currentServerHost = parent.currentUrl.host?.lowercased() ?? ""
            if scheme == "http" || scheme == "https" {
                if host.isEmpty || host == currentServerHost || host.hasSuffix("algosculptor.com") || host == "127.0.0.1" || host == "localhost" {
                    decisionHandler(.allow)
                    return
                }
                // External web links open in Safari
                UIApplication.shared.open(url)
                decisionHandler(.cancel)
                return
            }

            // Custom schemes
            if UIApplication.shared.canOpenURL(url) {
                UIApplication.shared.open(url)
                decisionHandler(.cancel)
                return
            }

            decisionHandler(.allow)
        }

        func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) {
            isHandlingNavigation = true
            parent.isLoading = true
            parent.loadFailed = false
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            isHandlingNavigation = false
            parent.isLoading = false
            webView.scrollView.refreshControl?.endRefreshing()
        }

        func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
            isHandlingNavigation = false
            parent.isLoading = false
            parent.loadFailed = true
            webView.scrollView.refreshControl?.endRefreshing()
        }

        func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
            isHandlingNavigation = false
            parent.isLoading = false
            parent.loadFailed = true
            webView.scrollView.refreshControl?.endRefreshing()
        }

        // Fail-Closed SSL: Reject invalid or untrusted certificates to prevent MITM
        func webView(_ webView: WKWebView, didReceive challenge: URLAuthenticationChallenge, completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void) {
            completionHandler(.performDefaultHandling, nil)
        }
    }
}
