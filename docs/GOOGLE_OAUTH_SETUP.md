# Setting up Google OAuth2 for Gemini in HireLens

HireLens uses OAuth2 PKCE to authenticate with the Google Generative Language API. No API key is stored on disk — only a short-lived access token (and an encrypted refresh token in the OS keyring).

This guide takes about 10 minutes.

---

## Prerequisites

- A Google account
- HireLens GUI running (`cargo run -- gui`)

---

## Step 1 — Create a Google Cloud project

1. Open [console.cloud.google.com](https://console.cloud.google.com/).
2. Click the project selector at the top → **New Project**.
3. Name it `HireLens` (or anything you like) → **Create**.
4. Make sure this project is selected in the top bar before continuing.

---

## Step 2 — Enable the Generative Language API

1. In the left sidebar: **APIs & Services → Library**.
2. Search for **Generative Language API**.
3. Click it → **Enable**.

> This is the API that powers Gemini. Without it, tokens will be issued but every request will return 403.

---

## Step 3 — Configure the OAuth2 consent screen

1. Go to **APIs & Services → OAuth consent screen**.
2. Choose **External** (lets you use your own Google account) → **Create**.
3. Fill in the required fields:
   - **App name**: `HireLens`
   - **User support email**: your email
   - **Developer contact email**: your email
4. Click **Save and Continue** through the Scopes and Test Users steps (no changes needed).
5. On the Summary page click **Back to Dashboard**.

> The app stays in "Testing" mode. That's fine for personal use — you can add your own Google account as a test user if prompted.

---

## Step 4 — Create OAuth2 credentials

1. Go to **APIs & Services → Credentials**.
2. Click **Create Credentials → OAuth client ID**.
3. **Application type**: **Desktop app**.
4. **Name**: `HireLens desktop` (or anything).
5. Click **Create**.

A dialog shows your **Client ID** and **Client Secret**. Copy both — you'll need them in the next step.

- Client ID looks like: `123456789-abcdefghijklmnop.apps.googleusercontent.com`
- Client Secret looks like: `GOCSPX-xxxxxxxxxxxxxxxxxxxxxxxx`

> These are not secret in the same sense as an API key — OAuth2 PKCE is designed to be safe even with a public client secret for desktop apps. Still, don't share them publicly.

---

## Step 5 — Enter credentials in HireLens

1. Open HireLens GUI: `cargo run -- gui`
2. Click **⚙️** (top right) to open Settings.
3. Expand **🌟 Google Gemini (OAuth2)**.
4. Expand **▸ Identifiants Google Cloud**.
5. Paste your **Client ID** and **Client Secret** into the respective fields.
   - They are saved immediately to `~/.config/hirelens/settings.json`.

---

## Step 6 — Authenticate

1. Still in Settings, click **🔑 Connexion Google**.
2. Your default browser opens the Google sign-in page.
3. Sign in with the Google account that has access to the project.
4. If prompted "HireLens wants to access your Google Account", click **Continue**.
5. HireLens shows **✅ Connecté à Google Gemini !** in the settings panel.

The access token is stored in the OS keyring (Windows Credential Manager / macOS Keychain / libsecret on Linux). It is never written to disk in plain text.

---

## Step 7 — Use Gemini

1. Close Settings (← Retour).
2. In the provider dropdown, select **🌟 Google Gemini**.
3. Paste your CV and job offer, click **🔍 Analyser** or **✨ Optimiser le CV**.

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| "Configurez Gemini dans ⚙️ Paramètres" (greyed out) | Client ID field is empty | Complete Steps 4–5 |
| "Token Gemini expiré" in Settings | Refresh token expired (rare) | Click 🔑 Connexion Google again |
| 403 on first API call | Generative Language API not enabled | Complete Step 2 |
| "access_denied" in browser | Account not added as test user | Go to OAuth consent screen → Test users → Add your email |
| Browser doesn't open | `open` crate can't find a browser | Open the URL printed in the terminal manually |

---

## How the flow works (for developers)

```
HireLens                  Google OAuth2
   │                           │
   ├─ generate PKCE challenge ─┤
   ├─ open browser ────────────► accounts.google.com/o/oauth2/v2/auth
   │                           │  scope: generativelanguage
   │                           │  redirect: http://127.0.0.1:<port>/callback
   │  ◄─ GET /callback?code=X ─┤
   ├─ POST token exchange ──────► oauth2.googleapis.com/token
   │  ◄─ { access_token, refresh_token } ─┤
   ├─ store in OS keyring ─────┤
```

- The redirect URI is `http://127.0.0.1:<random-port>/callback` — HireLens starts a temporary TCP listener on a random port and registers it dynamically. No fixed port to configure.
- The PKCE `state` parameter is verified on callback to prevent CSRF (see `src/auth/google.rs`).
- The access token expires in ~1 hour. HireLens refreshes it automatically using the stored refresh token.
- Source: `src/auth/google.rs`, `src/auth/oauth_server.rs`, `src/auth/token_store.rs`.
