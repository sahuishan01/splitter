import SwiftUI

struct ContentView: View {
    @State private var currentUrl: URL = {
        let stored = UserDefaults.standard.string(forKey: "server_url")?.trimmingCharacters(in: .whitespacesAndNewlines)
        if let stored = stored, let url = URL(string: stored) {
            return url
        }
        return URL(string: "https://splitter.algosculptor.com")!
    }()

    @State private var isLoading = false
    @State private var loadFailed = false
    @State private var showServerSheet = false
    @State private var serverInputText = ""
    @State private var testResultText = ""
    @State private var isTesting = false

    var body: some View {
        ZStack(alignment: .top) {
            Color(red: 13/255.0, green: 17/255.0, blue: 23/255.0)
                .ignoresSafeArea()

            VStack(spacing: 0) {
                // Top Loading Indicator
                if isLoading {
                    ProgressView()
                        .progressViewStyle(LinearProgressViewStyle(tint: Color(red: 88/255.0, green: 166/255.0, blue: 255/255.0)))
                        .frame(height: 2)
                }

                // Offline / Error banner
                if loadFailed {
                    HStack {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .foregroundColor(.black)
                        Text("Unable to connect to server.")
                            .font(.system(size: 13, weight: .medium))
                            .foregroundColor(.black)
                        Spacer()
                        Button(action: {
                            serverInputText = currentUrl.absoluteString
                            showServerSheet = true
                        }) {
                            Text("Change Server")
                                .font(.system(size: 12, weight: .bold))
                                .foregroundColor(.black)
                                .padding(.horizontal, 8)
                                .padding(.vertical, 4)
                                .background(Color.white.opacity(0.8))
                                .cornerRadius(4)
                        }
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                    .background(Color(red: 210/255.0, green: 153/255.0, blue: 34/255.0))
                }

                WebView(
                    currentUrl: $currentUrl,
                    isLoading: $isLoading,
                    loadFailed: $loadFailed,
                    onShowServerConfig: {
                        serverInputText = currentUrl.absoluteString
                        showServerSheet = true
                    }
                )
            }
        }
        .sheet(isPresented: $showServerSheet) {
            serverConfigSheet
        }
    }

    private var serverConfigSheet: some View {
        NavigationView {
            Form {
                Section(header: Text("Server Address")) {
                    TextField("https://splitter.algosculptor.com", text: $serverInputText)
                        .keyboardType(.URL)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                }

                if !testResultText.isEmpty {
                    Section {
                        Text(testResultText)
                            .font(.system(size: 13))
                            .foregroundColor(testResultText.starts(with: "✅") ? .green : .red)
                    }
                }

                Section {
                    Button(action: testConnection) {
                        HStack {
                            if isTesting {
                                ProgressView()
                                    .padding(.trailing, 4)
                            }
                            Text(isTesting ? "Testing..." : "Test Connection")
                        }
                    }
                    .disabled(isTesting || serverInputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)

                    Button("Reset to Default Server") {
                        let defaultUrl = URL(string: "https://splitter.algosculptor.com")!
                        UserDefaults.standard.removeObject(forKey: "server_url")
                        currentUrl = defaultUrl
                        showServerSheet = false
                    }
                    .foregroundColor(.red)
                }
            }
            .navigationTitle("Server Settings")
            .navigationBarItems(
                leading: Button("Cancel") {
                    showServerSheet = false
                },
                trailing: Button("Save & Connect") {
                    saveAndConnect()
                }
                .font(.headline)
                .disabled(serverInputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            )
        }
    }

    private func saveAndConnect() {
        var clean = serverInputText.trimmingCharacters(in: .whitespacesAndNewlines)
        if !clean.starts(with: "http://") && !clean.starts(with: "https://") {
            clean = "https://" + clean
        }
        if let url = URL(string: clean) {
            UserDefaults.standard.set(url.absoluteString, forKey: "server_url")
            currentUrl = url
            showServerSheet = false
        }
    }

    private func testConnection() {
        var clean = serverInputText.trimmingCharacters(in: .whitespacesAndNewlines)
        if !clean.starts(with: "http://") && !clean.starts(with: "https://") {
            clean = "https://" + clean
        }
        guard let url = URL(string: clean) else {
            testResultText = "❌ Invalid URL format"
            return
        }

        isTesting = true
        testResultText = "Testing connection..."

        let configUrl = url.appendingPathComponent("api/auth/config")
        var request = URLRequest(url: configUrl)
        request.timeoutInterval = 6

        URLSession.shared.dataTask(with: request) { data, response, error in
            DispatchQueue.main.async {
                isTesting = false
                if let error = error {
                    testResultText = "❌ Failed: \(error.localizedDescription)"
                    return
                }
                guard let http = response as? HTTPURLResponse, (200...299).contains(http.statusCode) else {
                    testResultText = "❌ Server error (HTTP \((response as? HTTPURLResponse)?.statusCode ?? 0))"
                    return
                }
                testResultText = "✅ Connection successful! Splitter server reached."
            }
        }.resume()
    }
}
