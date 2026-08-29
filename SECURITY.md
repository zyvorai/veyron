# Security Policy

**Related documentation:** [README.md](README.md) (TLS, API keys), [docs/DEVELOPER_VM_ACCESS.md](docs/DEVELOPER_VM_ACCESS.md) (cluster access patterns).

## Supported Versions

We release patches for security vulnerabilities for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| 0.1.x   | :x:                |

## Reporting a Vulnerability

We take the security of veyron seriously. If you have discovered a security vulnerability, please follow these steps:

### 🔒 Private Disclosure

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via one of the following methods:

1. **GitHub Security Advisories** (Preferred)
   - Go to the [Security tab](https://github.com/zyvorai/veyron/security/advisories)
   - Click "Report a vulnerability"
   - Fill in the details

2. **Email**
   - Send an email to: [your-email@example.com]
   - Include as much information as possible (see below)

### 📝 What to Include

Please include the following information:

- **Type of vulnerability** (e.g., buffer overflow, SQL injection, XSS, etc.)
- **Full paths of source file(s)** related to the vulnerability
- **Location of the affected source code** (tag/branch/commit or direct URL)
- **Step-by-step instructions to reproduce** the issue
- **Proof-of-concept or exploit code** (if possible)
- **Impact of the issue**, including how an attacker might exploit it

### ⏱️ Response Timeline

- **Initial Response**: Within 48 hours
- **Assessment**: Within 1 week
- **Fix Timeline**: Depends on severity
  - Critical: Within 7 days
  - High: Within 14 days
  - Medium: Within 30 days
  - Low: Next regular release

### 🛡️ Security Update Process

1. **Acknowledgment**: We'll acknowledge receipt of your vulnerability report
2. **Investigation**: We'll investigate and assess the severity
3. **Fix Development**: We'll develop a fix (with your help if desired)
4. **Notification**: We'll notify you when the fix is ready
5. **Release**: We'll release a security update
6. **Disclosure**: We'll publish a security advisory (crediting you if you wish)

### 🏆 Credit

We believe in giving credit where credit is due. If you report a valid security issue:

- We'll acknowledge your contribution in the security advisory
- We'll include your name (or handle) in the CHANGELOG
- You can choose to remain anonymous if you prefer

### 🔐 Security Best Practices

When using veyron:

1. **Keep Updated**: Always use the latest version
2. **Least Privilege**: Run with minimum required permissions
3. **Review Configs**: Validate VM configurations before applying
4. **Audit Logs**: Monitor veyron operations in production
5. **Secure Credentials**: Never commit credentials or secrets to git
6. **Network Security**: Use appropriate network policies in Kubernetes

### 🛡️ Security Hardening

Veyron implements the following security measures:

#### Authentication & Authorization
- **JWT Bearer token auth** with HMAC-SHA256 signature verification and OIDC issuer checking
- **Multi-key RBAC** via `VEYRON_API_KEYS` environment variable with three roles: `admin` (full access), `write` (create/modify operations), `readonly` (read-only access)
- API key authentication via `X-API-Key` header, `Authorization: Bearer`, or `?token=` query param
- Constant-time API key comparison prevents timing attacks
- **Auth bypass removed**: A previous referer-based dashboard bypass has been removed. The dashboard now sends the API key via the `X-API-Key` header like any other client, ensuring uniform authentication for all API requests.

#### Audit & Secrets
- **Persistent audit trail** saved to disk for all operations, enabling forensic review
- **Local secrets encryption** with key expansion and integrity tag for at-rest protection
- Secret values are zeroized on drop, rotate, and revoke using the `zeroize` crate
- `Secret` type does not implement `Clone` to prevent accidental copies that bypass zeroization
- Secret values are excluded from serialization (`#[serde(skip_serializing)]`)
- Custom `Debug` implementation redacts secret values

#### SSRF Prevention
- Webhook URLs are validated against private/internal IP ranges (RFC 1918, RFC 6598 CGNAT, link-local, loopback)
- DNS resolution is performed upfront and all returned addresses are validated
- Curl is pinned to the resolved IP via `--resolve` to prevent DNS rebinding attacks
- Only `http://` and `https://` schemes are allowed

#### Data Persistence
- Persistent data is never written to world-readable `/tmp`
- All file writes use atomic write-to-temp + fsync + rename to prevent corruption
- Temp files use randomized names to prevent race conditions
- Files are created with mode `0600` on Unix

#### API Security
- CORS disabled by default; requires explicit origin configuration
- TLS certificate validation is always enforced
- PAM usernames limited to 32 characters
- API error responses do not leak internal details
- Security headers on all responses (CSP, X-Frame-Options DENY, HSTS, no-sniff, no-referrer)
- Rate limiting (configurable per minute, dashboard endpoints exempt)
- Kubernetes name validation (RFC 1123) on all VM/snapshot names
- VNC WebSocket proxy uses direct K8s API WebSocket with client certificate authentication (mTLS)
- **VeyronPolicy CRD enforcement**: VM creation evaluates VeyronPolicy CRDs before proceeding. Policies with `Deny` enforcement block creation outright; policies with `Warn` enforcement log warnings but allow the operation to continue.
- **Structured error types**: The API returns proper HTTP status codes (e.g., 401 for auth failures, 409 for conflicts) instead of returning 200 with silent failures where feasible. VM lifecycle routes use Kubernetes directly; analytics routes may return heuristic projections labeled in JSON where applicable.
- **Input validation on batch operations**: Batch VM operations validate Kubernetes names (RFC 1123) before processing. Budget creation ensures the `veyron-system` namespace exists before writing resources.
- **CRDPolicyRule structured value field**: Policy conditions use a structured `value` field for thresholds instead of parsing values from message strings, which was fragile and potentially exploitable via crafted input.

#### Dashboard Security
- API key stored in localStorage (opt-in "Remember me") or sessionStorage
- 401 responses auto-clear stored key and re-prompt
- CSP restricts scripts to `'self'` only (no external CDN dependencies)
- noVNC bundled locally (no runtime CDN fetches)
- Dashboard page served without auth; all API calls require valid key
- Core endpoints return cluster-backed data; auxiliary endpoints document estimate/disclaimer fields where values are not sourced from billing or external SaaS.

#### Operator Security
- **CEL policy expressions**: The Veyron Operator supports CEL (Common Expression Language) policy expressions for custom compliance rules, evaluated safely in a sandboxed environment.
- **Operator Prometheus metrics**: 7 custom metrics exposed for monitoring operator health and policy enforcement activity.

#### Metrics Security
- **Real Kubernetes metrics**: Replaced random number generation (`rand::thread_rng`) with real data from the Kubernetes Metrics Server. When the Metrics Server is unavailable, the API returns allocation-only metrics (zeros) instead of fabricated random numbers, preventing misleading resource reporting.

### 🚨 Known Security Considerations

> **Note:** Several previous security considerations have been addressed:
> the referer-based auth bypass has been removed, random metric generation has been
> replaced with real Metrics Server data, all experimental modules have been promoted
> (no feature gates), JWT Bearer auth and multi-key RBAC have been added, and local
> secrets encryption is now available. See the "Security Hardening" section above for details.

#### Kubernetes Access
- veyron requires access to Kubernetes API
- Use appropriate RBAC policies to limit access
- Review and restrict service account permissions

#### Configuration Files
- Configuration files may contain sensitive data
- Use `.gitignore` to exclude sensitive configs
- Never commit cloud-init user-data with passwords

#### Container Images
- Container disk images are pulled from registries
- Verify image sources and use trusted registries
- Consider using private registries for production

#### Cloud-Init
- Cloud-init user-data can execute arbitrary code in VMs
- Review cloud-init scripts before deployment
- Avoid hardcoded credentials in cloud-init

### 📚 Security Resources

- [Kubernetes Security Best Practices](https://kubernetes.io/docs/concepts/security/)
- [KubeVirt Security](https://kubevirt.io/user-guide/security/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)

### 🤝 Security Hall of Fame

We'd like to thank the following people for responsibly disclosing security issues:

<!-- Will be updated as security reports are received and fixed -->

*No security issues reported yet.*

---

Thank you for helping keep veyron and our users safe!
