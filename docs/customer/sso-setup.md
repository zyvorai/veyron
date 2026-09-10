# Setting Up SSO (Keycloak / Auth0 / Okta)

Veyron supports signing in with your organization's identity provider (IdP) over OpenID Connect (OIDC), so people log in with their real account instead of a shared API key. A local admin login stays available as a break-glass fallback even after SSO is turned on.

## Before you start

You'll need admin access to your IdP to create a realm/application and a client, plus the ability to set environment variables (or a Kubernetes Secret) on the Veyron API deployment.

## 1. Create a client in your IdP

Using Keycloak as the example (the steps are conceptually the same for Auth0/Okta/Azure AD):

1. Create a realm for Veyron, e.g. `veyron`.
2. Create a client, e.g. `veyron`:
   - **Client authentication: On** (a *confidential* client) — the recommended choice. Client authentication **Off** (public) also works with no secret needed, PKCE alone secures it.
   - **Standard flow**: enabled.
   - **Valid redirect URIs**: your OIDC callback consumer's URL — the console's login screen does
     not yet drive this flow itself (see the note in step 3 below); if you're building a custom
     client against `POST /api/v1/auth/oidc/token`, use that client's own redirect URL.
   - **Web origins**: the same origin, e.g. `https://veyron.yourcompany.com`.
3. If you created a confidential client, copy its **client secret** from the Credentials tab.
4. Add group-membership info to the token: Keycloak includes a group-membership mapper on the client's own "dedicated" scope automatically — confirm it's there (Client → Client scopes → `<client>-dedicated` → mappers) with **Token Claim Name: `groups`**.
5. Create groups for your Veyron roles, e.g. `veyron-admins` and `veyron-editors`, and add people to them.

> If your IdP requires HTTPS on every endpoint (Keycloak's default) but you're running it without TLS termination in front of it, the auth flow will fail with an "HTTPS required" error until you either add TLS or (lab-only) turn off that requirement for the realm.

## 2. Configure Veyron

Set these on the Veyron API (environment variables, a Kubernetes Secret, or Helm chart `oidc.*` values):

| Setting | Value |
|---|---|
| Issuer | Your realm's issuer URL, e.g. `https://idp.yourcompany.com/realms/veyron` |
| Client ID | The client id you created, e.g. `veyron` |
| Client secret | From the client's Credentials tab — **only if you made a confidential client** |
| JWKS URL | `<issuer>/protocol/openid-connect/certs` (Keycloak) |
| Admin groups | Your IdP group name(s) that should get Veyron **admin** access, e.g. `veyron-admins` |
| Write groups | Your IdP group name(s) that should get Veyron **write** (non-admin) access, e.g. `veyron-editors` |

Deploying with the `deploy-remote.sh` helper script keeps secrets out of your shell history and off the command line:

```bash
export VEYRON_OIDC_ISSUER=https://idp.yourcompany.com/realms/veyron
export VEYRON_OIDC_CLIENT_ID=veyron
export VEYRON_OIDC_CLIENT_SECRET=<paste>
export VEYRON_OIDC_JWKS_URL=https://idp.yourcompany.com/realms/veyron/protocol/openid-connect/certs
export VEYRON_OIDC_GROUP_ADMIN=veyron-admins
export VEYRON_OIDC_GROUP_WRITE=veyron-editors

./scripts/deploy-remote.sh <host> <user> --with-oidc
```

Or with Helm directly:

```bash
helm upgrade --install veyron charts/veyron -n veyron \
  --set oidc.issuer=https://idp.yourcompany.com/realms/veyron \
  --set oidc.clientId=veyron \
  --set oidc.clientSecret=<paste> \
  --set oidc.jwksUrl=https://idp.yourcompany.com/realms/veyron/protocol/openid-connect/certs \
  --set oidc.groupAdmin=veyron-admins \
  --set oidc.groupWrite=veyron-editors
```

## 3. Try it

The React console's login screen currently takes username/password only — it does not yet have
a "Sign in with SSO" button. What you've configured above is real, live backend support for the
OIDC authorization-code + PKCE flow, consumable today by:

1. `GET /api/v1/auth/oidc/config` — confirm `"enabled": true` and the fields your client needs.
2. Your own OIDC client (or a future console build) driving the redirect to your IdP, then
   `POST /api/v1/auth/oidc/token` with the returned `code` + `code_verifier` to exchange for a
   Veyron bearer token — validated the same way as a username/password session from then on,
   with your access level set from your IdP group membership.
3. The console's own username/password login (`admin` + your configured password, or any other
   local account) keeps working unchanged — that's the only sign-in path the console UI exposes
   right now.

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| `GET /api/v1/auth/oidc/config` returns `"enabled": false` | The API doesn't have `VEYRON_OIDC_CLIENT_ID` + issuer/authorization URL configured yet, or the API hasn't picked up the config (restart the API pod after changing a Secret). |
| `POST /api/v1/auth/oidc/token` returns `unauthorized_client` / `invalid_client` | Most commonly a missing `client_secret` on a confidential IdP client — double-check the Client secret setting; a confidential client must authenticate itself at the token endpoint, PKCE alone isn't enough. |
| IdP shows "HTTPS required" or a similar TLS error | Your IdP is enforcing TLS on its endpoints but isn't reachable over HTTPS from where Veyron is running — put TLS in front of it, or (non-production/lab only) relax that requirement. |
| IdP rejects the whole login with an "invalid scope" error | A custom claim name (like `groups`) was added to the requested OAuth scopes — remove it. Group info doesn't need to be requested as a scope; the group mapper on the client's own scope already includes it in every token. |
| Everyone lands as read-only regardless of their IdP group | Group names must match your **Admin groups** / **Write groups** settings exactly (case-insensitive) — check the exact group name in your IdP against what you configured. |

## Related pages

- [Admin Basics](admin-basics.md)
- [Getting Started](getting-started.md)
- [Settings](pages/system/settings.md)
