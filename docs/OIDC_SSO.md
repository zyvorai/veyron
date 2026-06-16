# Enterprise SSO (OIDC) for Veyron

Configure OpenID Connect for human login while keeping API keys for automation. The classic dashboard at `/dashboard` uses API keys in browser localStorage today; OIDC bearer validation is available on the API for clients that obtain tokens from your IdP.

## API server environment

| Variable | Required | Description |
|----------|----------|-------------|
| `VMROGUE_OIDC_ISSUER` | For JWKS | Expected `iss` claim (e.g. `https://keycloak.example.com/realms/veyron`) |
| `VMROGUE_OIDC_JWKS_URL` | For JWKS | JWKS document URL |
| `VMROGUE_OIDC_ROLE_CLAIM` | No | JWT claim for role mapping (default: `groups`) |
| `VMROGUE_OIDC_USERINFO_URL` | Fallback | Userinfo endpoint when JWKS is not set |
| `VMROGUE_JWT_SECRET` + `VMROGUE_JWT_ISSUER` | Optional | HMAC JWT for service accounts |

Role mapping (first matching group wins):

- `veyron-admin` or claim value `admin` → **admin**
- `veyron-write` or `write` → **write**
- otherwise → **readonly**

## Public OIDC discovery

| Variable | Description |
|----------|-------------|
| `VMROGUE_OIDC_CLIENT_ID` | OAuth client id (exposed via discovery) |
| `VMROGUE_OIDC_AUTHORIZATION_URL` | Authorization endpoint |
| `VMROGUE_OIDC_TOKEN_URL` | Token endpoint |
| `VMROGUE_OIDC_REDIRECT_URI` | Callback URL (default: `{origin}/dashboard`) |

Clients read `GET /api/v1/auth/oidc/config` for issuer, client id, and endpoints. The classic dashboard uses PKCE (`POST /api/v1/auth/oidc/token` exchanges the authorization code server-side) and sends `Authorization: Bearer <access_token>` on API calls.

Route-level RBAC after authentication is enforced in `src/api/auth_context.rs` (e.g. mutating VM routes require **write**; cluster activate and DR routes require **admin**). VNC and serial WebSocket upgrades can use a one-time ticket from `POST /api/v1/ws/ticket` instead of embedding long-lived API keys in query strings.

## Keycloak quick start

1. Create realm `veyron`, client `veyron-dashboard`, access type **public**, PKCE enabled.
2. Valid redirect URI: `https://<api-host>/dashboard/*`
3. Add group mappers → `veyron-admin`, `veyron-write`.
4. Set env vars on the API Deployment (see table above).

## SAML / Okta / Azure AD

Use the vendor’s **OIDC** application (or Keycloak as broker). Veyron does not embed a SAML SP; MFA is enforced at the IdP.
