<!-- Copyright 2026 Zyvor AI Labs · https://zyvor.dev -->
<!-- SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0 -->
# Single sign-on (OIDC)

Veyron accepts OpenID Connect access tokens from Keycloak, Okta, Auth0, Azure AD or any
OIDC-compliant IdP, and maps IdP groups to Veyron roles. API keys keep working for automation, and
the local `admin` account remains as break-glass access.

## How it works

1. A client runs the authorization-code flow with PKCE against your IdP, using the settings from
   `GET /api/v1/auth/oidc/config` (public).
2. The client posts `{code, redirect_uri, code_verifier}` to `POST /api/v1/auth/oidc/token`. The
   API exchanges the code on the server side, so a confidential client's secret never reaches the
   browser.
3. The client sends `Authorization: Bearer <access_token>` on every call. Veyron checks the RS256
   signature (JWKS), issuer and expiry, then maps the group claim to a role. The `aud` claim is checked
   only when `VEYRON_OIDC_AUDIENCE` is set; otherwise any token your IdP issues for this issuer is accepted,
   whichever client it was minted for. Set it (for Keycloak, add an audience mapper for your client) or restrict
   who can obtain tokens (or use a dedicated realm). `VEYRON_API_KEY` must also be set:
   without it the API answers `503` before it looks at the bearer token.

## Settings

| Variable | Notes |
|---|---|
| `VEYRON_OIDC_ISSUER` | Expected `iss`; Keycloak-style endpoints are derived from it. SSO is switched on by `CLIENT_ID` plus either this or `AUTHORIZATION_URL`, but without an issuer the `iss` claim is **not checked**, so always set it |
| `VEYRON_OIDC_CLIENT_ID` | Required |
| `VEYRON_OIDC_AUDIENCE` | Optional. When set, the token's `aud` (string or array) must contain it. Unset keeps the old behaviour: not checked |
| `VEYRON_OIDC_CLIENT_SECRET` | Required for confidential clients (Keycloak's default). Omit for public clients |
| `VEYRON_OIDC_AUTHORIZATION_URL`, `VEYRON_OIDC_TOKEN_URL` | For IdPs that don't use Keycloak's URL layout |
| `VEYRON_OIDC_JWKS_URL` | Recommended: enables local signature checks. Without it, tokens are only accepted if `VEYRON_OIDC_USERINFO_URL` is set and the IdP's userinfo endpoint approves them |
| `VEYRON_OIDC_USERINFO_URL` | Fallback validation by calling the IdP's userinfo endpoint |
| `VEYRON_OIDC_REDIRECT_URI` | Where the IdP sends users back; register the full URL with the IdP |
| `VEYRON_OIDC_ROLE_CLAIM` | Claim holding groups (default `groups`) |
| `VEYRON_OIDC_GROUP_ADMIN` | Groups mapped to `admin` (default `veyron-admins,cluster-admins`) |
| `VEYRON_OIDC_GROUP_WRITE` | Groups mapped to `write` (default `veyron-write,veyron-editors`) |
| `VEYRON_OIDC_ROLE_MAP` | Optional JSON namespace -> group -> role map with deny by default, see [below](#per-namespace-roles-veyron_oidc_role_map) |

Role mapping: exact group matches come first, case-insensitive. Next, any group containing `admin`
or `write` maps to that role. Everyone else is `readonly`. This is the default; to deny callers
that match nothing, or to grant roles per namespace, use the role map below.

## Per-namespace roles (`VEYRON_OIDC_ROLE_MAP`)

A JSON object of namespace -> group -> role. `*` applies to every request:

```json
{
  "*":      { "platform-admins": "admin", "auditors": "readonly" },
  "team-a": { "team-a-devs": "write", "team-a-viewers": "readonly" }
}
```

- Roles are `admin`, `write`, `readonly`; group names match exactly, case-insensitive. A caller gets the highest role
  from `*` plus the request's namespace.
- **Deny by default.** Once the variable is set, the substring and `readonly` fallbacks no longer apply: a caller whose
  groups match nothing gets no role (`401`). `VEYRON_OIDC_GROUP_ADMIN` / `_WRITE` exact matches still count as `*`
  grants, so existing admin groups keep working. Unset, behaviour is exactly as described above.
- **Invalid JSON, an unknown role or a bad namespace name denies every OIDC caller** (and logs a warning) instead of
  being half-applied. API keys and local accounts are not affected.
- A namespace-specific grant applies only to routes that name the namespace in the path
  (`/api/v1/vms/{ns}/{name}/...`). Cluster-wide routes, and routes where the namespace is a query parameter or body
  field, are decided by `*` grants only, because a handler may ignore or override those. Route-level role rules
  (for example `admin`-only routes) still apply on top.
- It needs signed tokens: set `VEYRON_OIDC_JWKS_URL`. The userinfo fallback cannot carry the map, so with the map set it
  is not used (a token that fails JWKS validation is denied).
- Not covered: namespace grants are not a full tenant boundary. For hard tenant isolation also use
  `VEYRON_TENANT_SCOPES`.

Deploy the settings as a Secret instead of passing them on the command line:

```bash
set -a && source contrib/veyron-oidc-keycloak.env.example && set +a
VEYRON_OIDC_CLIENT_SECRET=<secret> ./scripts/deploy-remote.sh <host> <user> --with-oidc
```

With Helm, use `oidc.*` values or `oidc.existingSecret`.

`--with-oidc` and the Helm `oidc.*` values do not carry `VEYRON_OIDC_AUDIENCE` or `VEYRON_OIDC_USERINFO_URL`. Add them
to the Secret yourself and restart the API (`--with-oidc` deletes and recreates `veyron-oidc`, so repeat this after
each such deploy):

```bash
kubectl -n veyron-system patch secret veyron-oidc --type merge \
  -p '{"stringData":{"VEYRON_OIDC_AUDIENCE":"veyron"}}'
kubectl -n veyron-system rollout restart deploy/veyron-api
```

With Helm, create your own Secret holding all the `VEYRON_OIDC_*` keys and name it in `oidc.existingSecret`.

## Keycloak in five steps

1. Create a realm, for example `veyron`.
2. Create a client `veyron`: client authentication **on**, standard flow **on**. Set the valid
   redirect URI to your exact callback URL and the web origin to `https://<host>:30151`.
3. Copy the secret from the client's **Credentials** tab.
4. On the client's dedicated scope, add a **Group Membership** mapper with token claim `groups` and
   **Full group path** off.
5. Create the groups `veyron-admins` and `veyron-editors` and add users to them.

## Common problems

| Symptom | Cause |
|---|---|
| `unauthorized_client` from the token exchange | Confidential client with no `VEYRON_OIDC_CLIENT_SECRET` set |
| `invalid_scope` at the IdP | A custom scope such as `groups` was requested; `openid profile email` is enough |
| `HTTPS required` from Keycloak | The realm requires SSL and Keycloak has no TLS; put TLS in front of it |
| Token accepted but role is `readonly` | Group names don't match; set `VEYRON_OIDC_GROUP_ADMIN` / `_WRITE` |
| `401` for a valid token with `VEYRON_OIDC_ROLE_MAP` set | No group matches `*` (or the path's namespace), the map is invalid (see the API log), or `VEYRON_OIDC_JWKS_URL` is not set |

**Console note:** the console currently signs in with local accounts or API keys. OIDC bearer
tokens work on the API today, and an SSO button in the console is on the roadmap.
