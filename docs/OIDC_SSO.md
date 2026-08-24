# Enterprise SSO (OIDC) for Veyron

Configure OpenID Connect so people sign in with their real identity (Keycloak, Auth0, Okta, Azure AD, or any OIDC-compliant IdP) instead of a shared API key. API keys keep working for automation/CI alongside OIDC — the two aren't mutually exclusive, and the local `admin` / bootstrap-password login stays available as a break-glass fallback.

Verified end-to-end against a real Keycloak realm (2026-08-24): SSO login, IdP-group → Veyron-role mapping, and the local-admin break-glass fallback all working together. The walkthrough below and the "Gotchas" section reflect that live run, not just the code.

## How it works

- The dashboard (`dashboard.html`) does the PKCE authorization-code flow itself: `startOidcLogin()` builds the authorization URL from `GET /api/v1/auth/oidc/config` and redirects the browser to your IdP.
- Your IdP redirects back to the dashboard's own URL (`{origin}/dashboard` by default) with `?code=&state=`. The frontend never talks to the IdP's token endpoint directly — it POSTs `{code, redirect_uri, code_verifier}` to `POST /api/v1/auth/oidc/token`, and the **API server** exchanges the code server-side (so the IdP's `token_url` never needs to be reachable from the browser, and a confidential client's secret never reaches the browser either).
- Once the frontend has an `access_token`, it sends `Authorization: Bearer <access_token>` on every API call. The API validates it via JWKS (RS256 signature + issuer + expiry) and maps the token's group claim to a Veyron role.
- There is **no separate backend OAuth `redirect_uri` route** — the redirect target registered with your IdP is the dashboard's own URL, not an API path.

## API server environment

| Variable | Required | Description |
|----------|----------|--------------|
| `VEYRON_OIDC_ISSUER` | Yes | Expected `iss` claim (e.g. `https://keycloak.example.com/realms/veyron`). Also used to derive `AUTHORIZATION_URL`/`TOKEN_URL` if those aren't set explicitly (assumes a Keycloak-style `/protocol/openid-connect/{auth,token}` layout). |
| `VEYRON_OIDC_CLIENT_ID` | Yes | OAuth client id, exposed via discovery. |
| `VEYRON_OIDC_CLIENT_SECRET` | **Only for confidential clients** | Sent as `client_secret` in the server-side token exchange. **Required whenever the IdP client is confidential** (Keycloak's default — `publicClient: false`); omit entirely for a public/PKCE-only client. Missing this on a confidential client makes every login fail with `unauthorized_client` even though the browser-side redirect looks successful. |
| `VEYRON_OIDC_AUTHORIZATION_URL` / `VEYRON_OIDC_TOKEN_URL` | No (derived from issuer if unset) | Explicit endpoints, for IdPs that don't follow Keycloak's URL layout. |
| `VEYRON_OIDC_JWKS_URL` | Recommended | JWKS document URL — enables real RS256 signature verification of bearer tokens. Without it, the API falls back to a userinfo-endpoint check (`VEYRON_OIDC_USERINFO_URL`) instead of verifying signatures locally. |
| `VEYRON_OIDC_REDIRECT_URI` | No | Path or full URL the frontend redirects back to (default `/dashboard`, resolved against the page's own origin). Register the resolved **full** URL (e.g. `https://your-host:30151/dashboard`) as the IdP client's redirect URI — not a bare relative path. |
| `VEYRON_OIDC_ROLE_CLAIM` | No | JWT claim holding group/role info (default: `groups`). |
| `VEYRON_OIDC_GROUP_ADMIN` | No | Comma-separated, case-insensitive IdP group names mapped to the **admin** role (exact match). Default: `veyron-admins,cluster-admins`. |
| `VEYRON_OIDC_GROUP_WRITE` | No | Same, mapped to the **write** role. Default: `veyron-write,veyron-editors`. |
| `VEYRON_OIDC_USERINFO_URL` | Fallback | Userinfo endpoint, used only when `VEYRON_OIDC_JWKS_URL` isn't set. |
| `VEYRON_JWT_SECRET` + `VEYRON_JWT_ISSUER` | Optional | Separate HMAC JWT support for service accounts — unrelated to OIDC, doesn't require an IdP. |

### Role mapping

1. **Exact group match first** — if the token's `VEYRON_OIDC_ROLE_CLAIM` (default `groups`) contains any of the names in `VEYRON_OIDC_GROUP_ADMIN` → **admin**; any name in `VEYRON_OIDC_GROUP_WRITE` → **write**. Case-insensitive, comma-separated.
2. **Substring fallback** — if no configured group matches, a group/role value containing `admin` (or the literal `veyron-admin`) → **admin**; containing `write` (or `veyron-write`) → **write**.
3. Anything else → **readonly**.

Use the exact-match vars (`VEYRON_OIDC_GROUP_ADMIN`/`GROUP_WRITE`) whenever your IdP's group names don't happen to contain "admin"/"write" as a substring — e.g. `engineering-leads` or `platform-team`.

## Public OIDC discovery

`GET /api/v1/auth/oidc/config` (unauthenticated) returns `{enabled, issuer, client_id, authorization_url, token_url, redirect_uri, scope}` for the frontend to build the login redirect. `enabled` is `true` only when `VEYRON_OIDC_CLIENT_ID` is set **and** either `VEYRON_OIDC_AUTHORIZATION_URL` or `VEYRON_OIDC_ISSUER` is set.

Route-level RBAC after authentication is enforced in `src/api/auth_context.rs` (e.g. mutating VM routes require **write**; cluster activate and DR routes require **admin**). VNC and serial WebSocket upgrades use a one-time ticket from `POST /api/v1/ws/ticket` instead of embedding long-lived credentials in query strings.

## Keycloak walkthrough (verified working end-to-end)

This is the exact setup verified live — adapt the realm/client/host names to your environment.

### 1. Create a dedicated realm and confidential client

In the Keycloak admin console (or via the Admin REST API):

1. Create a realm, e.g. `veyron`.
2. **Realm settings → General → Require SSL**: set to **`None`** *only if* this Keycloak instance is fronted by plain HTTP (no TLS termination) — Keycloak's default `sslRequired: external` rejects every OIDC endpoint, **including `/.well-known/openid-configuration`**, with `{"error":"invalid_request","error_description":"HTTPS required"}` for any request it doesn't consider internal/loopback. A real deployment should have TLS on Keycloak instead of loosening this.
3. Create a client, e.g. `veyron`:
   - **Client authentication**: **On** (confidential) — the standard, recommended choice. A public client is also supported (see below) but a confidential client keeps the secret server-side only.
   - **Standard flow**: enabled.
   - **Valid redirect URIs**: the exact URL your dashboard resolves to, e.g. `https://<your-host>:30151/dashboard` (add a `/*` variant too if you want some slack, but the app always redirects to this one exact path).
   - **Web origins**: `https://<your-host>:30151`.
4. Copy the client secret from the client's **Credentials** tab.

### 2. Map groups to a claim

Keycloak includes a group-membership mapper on the client's **dedicated** client scope by default — this is included in every token issued to the client unconditionally, **regardless of what's in the `scope=` request parameter**. Confirm (or add) a mapper:

- Client → **Client scopes** → `<client-id>-dedicated` → **Add mapper** → **By configuration** → **Group Membership**.
- **Token Claim Name**: `groups` (matches `VEYRON_OIDC_ROLE_CLAIM` default).
- **Full group path**: off (so you get `veyron-admins`, not `/veyron-admins`).
- Add to ID token / access token / userinfo: all on.

Create groups matching your intended role mapping, e.g. `veyron-admins` and `veyron-editors`, and add users to them.

> **Don't add `groups` (or any custom claim name) to the requested scopes** (`openid profile email` is sufficient). Keycloak rejects the *whole* authorization request with `invalid_scope` if you request a scope name that isn't a registered client scope — the dedicated-scope mapper above doesn't need to be requested to be included.

### 3. Deploy Veyron pointed at the realm

Set the env vars (directly, via a `.env` file, or via Kubernetes Secret) and deploy:

```bash
export VEYRON_OIDC_ISSUER=https://<keycloak-host>/realms/veyron
export VEYRON_OIDC_CLIENT_ID=veyron
export VEYRON_OIDC_CLIENT_SECRET=<paste from Keycloak Credentials tab>
export VEYRON_OIDC_JWKS_URL=https://<keycloak-host>/realms/veyron/protocol/openid-connect/certs
export VEYRON_OIDC_REDIRECT_URI=/dashboard
export VEYRON_OIDC_ROLE_CLAIM=groups
export VEYRON_OIDC_GROUP_ADMIN=veyron-admins,cluster-admins
export VEYRON_OIDC_GROUP_WRITE=veyron-editors

# Remote/lab deploy (writes a `veyron-oidc` Secret from the env above — never via --set,
# since a client secret can contain characters --set mishandles):
./scripts/deploy-remote.sh <host> <user> --with-oidc

# Helm (production): set oidc.* values, or point oidc.existingSecret at your own Secret
# whose keys are named VEYRON_OIDC_ISSUER, VEYRON_OIDC_CLIENT_ID, etc.
helm upgrade --install veyron charts/veyron -n veyron \
  --set oidc.issuer=https://<keycloak-host>/realms/veyron \
  --set oidc.clientId=veyron \
  --set oidc.clientSecret=<paste> \
  --set oidc.jwksUrl=https://<keycloak-host>/realms/veyron/protocol/openid-connect/certs \
  --set oidc.groupAdmin=veyron-admins,cluster-admins \
  --set oidc.groupWrite=veyron-editors
```

See [`contrib/veyron-oidc-keycloak.env.example`](../contrib/veyron-oidc-keycloak.env.example) for a filled-in template (secret intentionally excluded — pass it inline at deploy time, never commit it).

### 4. Verify

- `curl https://<your-host>/api/v1/auth/oidc/config` → `{"enabled":true,...}`.
- Open the dashboard, click **Sign in with SSO**, log in at Keycloak. You should land back on the dashboard authenticated, with your role determined by group membership (top-right shows `SSO session · <username>`).
- The local `admin` / bootstrap-password login should still work as a break-glass fallback — confirm the SSO switch didn't disturb it.

### Public (non-confidential) client alternative

If you'd rather not manage a client secret at all, set **Client authentication: Off** in Keycloak (a public client) and simply don't set `VEYRON_OIDC_CLIENT_SECRET` — PKCE alone secures the exchange for a public client, and the token endpoint accepts the request without a secret. Everything else in this guide is unchanged.

## Gotchas (found live, worth knowing before you debug from scratch)

1. **Confidential client → `client_secret` is mandatory at the token endpoint**, PKCE alone is not sufficient. Symptom: the browser redirect to/from Keycloak looks completely successful (login form renders, redirect lands back on the right URL with `200`), but the app silently stays on the login screen. Fix: set `VEYRON_OIDC_CLIENT_SECRET`.
2. **`sslRequired=external` (Keycloak's default) blocks an HTTP-only IdP entirely**, including the discovery/JWKS endpoints — see step 1 of the walkthrough above.
3. **The redirect URI must match exactly what the app resolves to** — `{origin}${VEYRON_OIDC_REDIRECT_URI}`, i.e. `https://your-host:30151/dashboard` by default, **not** a separate backend callback path. There is no `/api/v1/auth/oidc/callback` route in Veyron.
4. **Don't request custom claim names as OAuth scopes** (e.g. `groups`) unless the IdP has that scope explicitly registered — see step 2 above.
5. A cert-warning interstitial from a self-signed TLS cert can block automated browser testing tools that can't click through Chrome's own security interstitial — not a Veyron issue, just something to expect when testing against a lab deployment with the API's auto-generated self-signed cert.

## SAML / other IdPs

Use the vendor's **OIDC** application (or Keycloak as a broker in front of SAML/Azure AD/etc.) — Veyron doesn't embed a SAML SP. MFA is enforced at the IdP, not by Veyron.
