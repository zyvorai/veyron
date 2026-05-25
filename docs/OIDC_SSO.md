# Enterprise SSO (OIDC) for VMRogue

Configure OpenID Connect for human login to `/dashboard-next/` while keeping API keys for automation.

## API server environment

| Variable | Required | Description |
|----------|----------|-------------|
| `VMROGUE_OIDC_ISSUER` | For JWKS | Expected `iss` claim (e.g. `https://keycloak.example.com/realms/vmrogue`) |
| `VMROGUE_OIDC_JWKS_URL` | For JWKS | JWKS document URL |
| `VMROGUE_OIDC_ROLE_CLAIM` | No | JWT claim for role mapping (default: `groups`) |
| `VMROGUE_OIDC_USERINFO_URL` | Fallback | Userinfo endpoint when JWKS is not set |
| `VMROGUE_JWT_SECRET` + `VMROGUE_JWT_ISSUER` | Optional | HMAC JWT for service accounts |

Role mapping (first matching group wins):

- `vmrogue-admin` or claim value `admin` → **admin**
- `vmrogue-write` or `write` → **write**
- otherwise → **readonly**

## Dashboard (PKCE public client)

| Variable | Description |
|----------|-------------|
| `VMROGUE_OIDC_CLIENT_ID` | OAuth client id |
| `VMROGUE_OIDC_AUTHORIZATION_URL` | Authorization endpoint |
| `VMROGUE_OIDC_TOKEN_URL` | Token endpoint |
| `VMROGUE_OIDC_REDIRECT_URI` | Callback URL (default: `{origin}/dashboard-next/`) |

The operator UI reads `GET /api/v1/auth/oidc/config` and shows **Sign in with SSO** when configured.

## Keycloak quick start

1. Create realm `vmrogue`, client `vmrogue-dashboard`, access type **public**, PKCE enabled.
2. Valid redirect URI: `https://<api-host>/dashboard-next/*`
3. Add group mappers → `vmrogue-admin`, `vmrogue-write`.
4. Set env vars on the API Deployment (see table above).

## SAML / Okta / Azure AD

Use the vendor’s **OIDC** application (or Keycloak as broker). VMRogue does not embed a SAML SP; MFA is enforced at the IdP.
