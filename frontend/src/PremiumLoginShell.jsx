/**
 * Apple Store chapter login — 1:1 with h2kvm PremiumLoginShell.tsx (JSX, no Tailwind).
 */
import './zyvor-premium-login.css';

export function PremiumLoginShell({
  logo,
  productName,
  productWordmark,
  productSubtitle,
  heroTitle = 'Private cloud control.',
  heroSubheadline,
  heroCta,
  accent = 'sky',
  chapterNote = 'Veyron · sign in to continue',
  pills,
  panelTitle,
  panelSubtitle,
  panelHint,
  footer,
  formClassName = '',
  showSignInChapter = true,
  children,
}) {
  const tagline =
    heroSubheadline ?? productSubtitle ?? 'Manage KubeVirt VMs from one console. Scroll to sign in.';
  const formHeading =
    panelSubtitle ?? (panelTitle && panelTitle !== 'Sign in' ? panelTitle : 'Sign in');
  const wordmark = (productWordmark ?? productName).trim() || 'veyron';

  return (
    <div className="login-page login-store-page" data-testid="premium-login-shell">
      <main className="login-store-scroll" aria-label="Sign in">
        <section className="login-chapter login-chapter-hero" data-tone={accent} aria-label={productName}>
          <div className="login-chapter-inner">
            {logo ? <div className="login-logo">{logo}</div> : null}
            <p className="login-wordmark" aria-label={productName}>
              {wordmark}
            </p>
            <h1 className="login-hero-title">{heroTitle}</h1>
            {tagline ? <p className="login-tagline">{tagline}</p> : null}
            {pills?.length ? (
              <div className="login-pill-row">
                {pills.map((pill) => (
                  <span key={pill.label} data-tone={pill.tone ?? accent} className="login-pill">
                    <span className="login-pill-dot" aria-hidden />
                    {pill.icon}
                    {pill.label}
                  </span>
                ))}
              </div>
            ) : null}
            {heroCta ? <div className="login-cta">{heroCta}</div> : null}
            {chapterNote ? <p className="login-chapter-note">{chapterNote}</p> : null}
          </div>
        </section>

        {showSignInChapter && children ? (
          <section id="login-sign-in" className="login-chapter login-chapter-sign-in" aria-label="Credentials">
            <div className="login-chapter-inner login-sign-in-inner">
              <p className="login-form-heading">{formHeading}</p>
              <div className={`login-card ${formClassName}`.trim()}>{children}</div>
              {panelHint ? <p className="login-hint">{panelHint}</p> : null}
            </div>
          </section>
        ) : null}
      </main>

      {footer}
    </div>
  );
}

export function LoginError({ message }) {
  if (!message) return null;
  return (
    <div className="login-error login-shake" role="alert" aria-live="assertive">
      <AlertIcon />
      <div>
        <p className="login-error-title">Unable to sign in</p>
        <p className="login-error-msg">{message}</p>
      </div>
    </div>
  );
}

export function LoginField({ label, id, children }) {
  return (
    <div className="login-field-block">
      <label htmlFor={id} className="login-field-label">
        {label}
      </label>
      <div className="login-field-control">{children}</div>
    </div>
  );
}

export function LoginSubmit({ loading, disabled, children, className = '' }) {
  return (
    <button type="submit" disabled={disabled || loading} className={`login-btn-primary ${className}`.trim()}>
      {children}
    </button>
  );
}

export function LoginRemember({
  checked,
  onChange,
  label = 'Remember me on this device',
  hint,
}) {
  return (
    <div className="login-remember-block">
      <label className="login-remember">
        <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
        <span>{label}</span>
      </label>
      {hint ? <p className="login-remember-hint">{hint}</p> : null}
    </div>
  );
}

function AlertIcon() {
  return (
    <svg className="login-error-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <circle cx="12" cy="12" r="10" />
      <path d="M12 8v4" />
      <path d="M12 16h.01" />
    </svg>
  );
}
