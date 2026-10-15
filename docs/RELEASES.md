# Splitter Mobile Release & Self-Hosting Guide

This document describes the complete release paths for Splitter on **Android** and **iOS**, as well as the dynamic server configuration and admin-only onboarding mechanisms.

---

## 1. Dynamic Server URL Architecture

Splitter is server-agnostic. Anyone can host the backend instance and point the Web client, Android app, or iOS app to their server.

### Connecting to a Custom Server
* **In Web / PWA**:
  - Open Splitter.
  - On the Sign In page, click **⚙️ Server URL** at the bottom of the card, or click **🌐 Server** in the top navigation bar when logged in.
  - Enter the full server URL (e.g. `https://expenses.example.com` or `http://192.168.1.100:8088`).
  - Click **🔍 Test Connection** to verify server reachability and policy.
  - Click **Save & Connect** to persist the configuration.
* **In Android App**:
  - The Android app reads the configured server from `SharedPreferences`, defaulting to `https://splitter.algosculptor.com`.
  - Changing the server URL in the web UI automatically invokes the native Android bridge (`AndroidNative.setServerUrl(url)`), updating native background workers (`SyncQueueWorker`).
  - If the server is unreachable or offline, tap the yellow offline/connection error banner to open the native **Configure Server URL** prompt.
* **In iOS App**:
  - The native SwiftUI WKWebView app reads the server URL from `UserDefaults.standard` (`key: "server_url"`).
  - Changing the server in the web UI triggers `window.webkit.messageHandlers.setServerUrl.postMessage(url)`.
  - If a connection fails, tap **Change Server** on the native error banner to launch the native Server Settings sheet.

---

## 2. Admin-Only Onboarding & Forced Password Reset

To run Splitter in private or enterprise environments where public signups must be restricted:

### Disabling Public Registration
1. Log in as an administrator.
2. Navigate to the **Admin Dashboard** (`/admin`).
3. Toggle the **Public Registration Policy** switch to **Disabled (Admin Only)**.
   - Alternatively, start the server with environment variable `DISABLE_SIGNUP=true`.
4. Once disabled, the public **Create Account** tab is automatically hidden from the login screen and `/api/auth/register` rejects unauthenticated signups with `403 Forbidden`.

### Provisioning New Users
1. In the Admin Dashboard, click **➕ Add New User**.
2. Enter the user's Display Name and Email Address.
3. (Optional) Provide a temporary password or leave blank to auto-generate a secure random password (e.g. `Splitter#a1b2c3`).
4. Click **Create User**.
5. Copy the generated temporary credentials and provide them to the user.

### First-Login Password Prompt
* Newly provisioned users are flagged with `must_change_password: true`.
* Upon initial sign-in, Splitter presents an unavoidable, non-dismissible modal: **🔑 Set Permanent Password**.
* The user cannot navigate or execute financial transactions until they submit a new permanent password (minimum 6 characters).
* Once updated, `must_change_password` is cleared and standard access is granted.

---

## 3. Android Release Path

### Deterministic Persistent Release Signing
Per repository invariants, Android releases must strictly be signed with the persistent release keystore stored on the development host at `/home/opc/splitter-signing/splitter-upload.keystore` (permissions `0700` for directory, `0600` for keystore and password files).

### Local Release Build Commands
```bash
cd /home/opc/projects/splitter/android

# Set up local signing properties
cat > keystore.properties <<EOF
storeFile=/home/opc/splitter-signing/splitter-upload.keystore
storePassword=$(cat /home/opc/splitter-signing/store.pass)
keyAlias=splitter-upload
keyPassword=$(cat /home/opc/splitter-signing/key.pass)
EOF

# Build signed APK (direct sideload) and AAB (Google Play Store)
./gradlew assembleRelease bundleRelease --no-daemon

# Artifact locations:
# Signed APK: app/build/outputs/apk/release/app-release.apk
# Signed AAB: app/build/outputs/bundle/release/app-release.aab
```

### GitHub Actions CI/CD Secrets
Configure the following repository secrets in GitHub:
* `UPLOAD_KEYSTORE_B64`: Base64 string of `/home/opc/splitter-signing/splitter-upload.keystore`
  ```bash
  base64 -w 0 /home/opc/splitter-signing/splitter-upload.keystore
  ```
* `KEYSTORE_PASSWORD`: Keystore master password
* `KEY_PASSWORD`: Key alias password
* `KEY_ALIAS`: Keystore alias (`splitter-upload`)

Pushes to tags `v*` trigger `.github/workflows/release-mobile.yml`, publishing the signed APK and AAB to GitHub Releases.

---

## 4. iOS Release Path

The native iOS client is located in `ios/` as a SwiftUI Xcode project (`Splitter.xcodeproj`).

### Project Specifications
* **Bundle Identifier**: `com.splitter.app`
* **Target Deployment**: iOS 16.0+
* **Frameworks**: SwiftUI, WebKit
* **Supported Devices**: iPhone & iPad

### Local Build & Archive (macOS)
```bash
cd /path/to/splitter/ios

# 1. Archive the project
xcodebuild archive \
  -project Splitter.xcodeproj \
  -scheme Splitter \
  -archivePath build/Splitter.xcarchive \
  -destination 'generic/platform=iOS'

# 2. Export signed IPA for App Store / TestFlight distribution
xcodebuild -exportArchive \
  -archivePath build/Splitter.xcarchive \
  -exportPath dist/ \
  -exportOptionsPlist ExportOptions.plist
```

### Automated GitHub Actions CI Workflow
* The workflow `.github/workflows/release-mobile.yml` includes an automated `build-ios` job running on `macos-latest`.
* It compiles `Splitter.xcarchive` and packages it as `splitter-v<version>-ios.xcarchive.zip` in the release assets, ready for App Store Connect or TestFlight deployment.
