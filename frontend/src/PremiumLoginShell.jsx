/**
 * Centered sign-in composition — Netra / apple.com contract (../netra/web/src/components/Login.tsx).
 */
import './login.css';
import zyvorMark from './assets/zyvor-mark.svg';

export function PremiumLoginShell({
  productName = 'Veyron',
  eyebrow = 'Veyron · Zyvor',
  heroTitle = 'Real VMs. One console.',
  heroLede,
  showHost = true,
  children,
}) {
  const host = typeof window !== 'undefined' ? window.location.host || window.location.hostname : '';
  return (
    <div className="login-shell" data-theme="light" data-testid="premium-login-shell">
      <div className="login-info">
        <img src={zyvorMark} className="login-logo" alt={productName} />
        <p className="eyebrow">{eyebrow}</p>
        <h1>{heroTitle}</h1>
        {heroLede ? <p>{heroLede}</p> : null}
        {showHost && host ? (
          <p className="login-host">
            Connecting to <code>{host}</code>
          </p>
        ) : null}
      </div>
      {children}
    </div>
  );
}

export function LoginError({ message }) {
  if (!message) return null;
  return (
    <p className="login-error" role="alert" aria-live="assertive">
      {message}
    </p>
  );
}
