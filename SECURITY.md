# Security Policy

## Supported Versions

We release patches for security vulnerabilities for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take the security of vmrogue seriously. If you have discovered a security vulnerability, please follow these steps:

### 🔒 Private Disclosure

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via one of the following methods:

1. **GitHub Security Advisories** (Preferred)
   - Go to the [Security tab](https://github.com/ssahani/VMRogue/security/advisories)
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

When using vmrogue:

1. **Keep Updated**: Always use the latest version
2. **Least Privilege**: Run with minimum required permissions
3. **Review Configs**: Validate VM configurations before applying
4. **Audit Logs**: Monitor vmrogue operations in production
5. **Secure Credentials**: Never commit credentials or secrets to git
6. **Network Security**: Use appropriate network policies in Kubernetes

### 🚨 Known Security Considerations

#### Kubernetes Access
- vmrogue requires access to Kubernetes API
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

Thank you for helping keep vmrogue and our users safe!
