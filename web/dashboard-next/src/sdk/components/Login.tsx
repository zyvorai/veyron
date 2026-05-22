import React, { useEffect, useMemo, useState } from 'react';

interface LoginProps {
  onLogin: (username: string, password: string) => Promise<void>;
}

const USERNAME_STORAGE_KEY = 'vmrogue_dashboard_username';
const PASSWORD_STORAGE_KEY = 'vmrogue_dashboard_password';
const REMEMBER_STORAGE_KEY = 'vmrogue_dashboard_remember';

const features = [
  {
    title: 'KubeVirt inventory',
    description: 'Browse namespaces, VMs, and cluster state from a focused operations console.',
  },
  {
    title: 'Remote access ready',
    description: 'Jump into console, SSH, RDP, and workflow actions when the fleet needs attention.',
  },
  {
    title: 'Operator workflows',
    description: 'Track jobs, signals, and provider health without losing context.',
  },
] as const;

export const Login: React.FC<LoginProps> = ({ onLogin }) => {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [rememberMe, setRememberMe] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const savedUsername = localStorage.getItem(USERNAME_STORAGE_KEY);
    const remembered = localStorage.getItem(REMEMBER_STORAGE_KEY) === 'true';

    // Older builds stored passwords in localStorage; remove them when the page loads.
    localStorage.removeItem(PASSWORD_STORAGE_KEY);

    if (savedUsername && remembered) {
      setUsername(savedUsername);
      setRememberMe(true);
    }
  }, []);

  const canSubmit = useMemo(() => username.trim().length > 0 && password.length > 0 && !isLoading, [
    username,
    password,
    isLoading,
  ]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;

    setIsLoading(true);
    setError(null);

    try {
      await onLogin(username.trim(), password);

      if (rememberMe) {
        localStorage.setItem(USERNAME_STORAGE_KEY, username.trim());
        localStorage.setItem(REMEMBER_STORAGE_KEY, 'true');
      } else {
        localStorage.removeItem(USERNAME_STORAGE_KEY);
        localStorage.removeItem(REMEMBER_STORAGE_KEY);
      }
      localStorage.removeItem(PASSWORD_STORAGE_KEY);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setIsLoading(false);
    }
  };

  const inputStyle: React.CSSProperties = {
    width: '100%',
    padding: '12px 14px',
    border: '1px solid rgba(34, 35, 36, 0.14)',
    borderRadius: '10px',
    fontSize: '15px',
    backgroundColor: '#fff',
    color: '#111827',
    boxShadow: '0 1px 2px rgba(15, 23, 42, 0.04)',
    transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
  };

  const labelStyle: React.CSSProperties = {
    display: 'block',
    fontSize: '13px',
    fontWeight: '600',
    color: '#222324',
    marginBottom: '8px',
  };

  return (
    <div style={{
      minHeight: '100vh',
      backgroundColor: '#f0f2f7',
      backgroundImage: 'radial-gradient(circle at 12% 18%, rgba(240, 88, 58, 0.14), transparent 32%), radial-gradient(circle at 88% 8%, rgba(34, 35, 36, 0.08), transparent 28%), linear-gradient(135deg, #f7f8fb 0%, #eef1f7 50%, #f9fafb 100%)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      padding: '32px',
    }}>
      <div className="vmrogue-login-shell" style={{
        width: '100%',
        maxWidth: '1080px',
        minHeight: '620px',
        display: 'grid',
        gridTemplateColumns: 'minmax(0, 1.05fr) minmax(360px, 0.8fr)',
        backgroundColor: '#fff',
        borderRadius: '28px',
        border: '1px solid rgba(34, 35, 36, 0.08)',
        boxShadow: '0 32px 80px rgba(34, 35, 36, 0.14)',
        overflow: 'hidden',
      }}>
        <section
          className="vmrogue-login-hero"
          style={{
            position: 'relative',
            display: 'flex',
            flexDirection: 'column',
            justifyContent: 'space-between',
            minHeight: '620px',
            padding: '48px',
            color: '#fff',
            backgroundColor: '#222324',
            backgroundImage: 'radial-gradient(circle at 16% 18%, rgba(240, 88, 58, 0.5), transparent 30%), radial-gradient(circle at 86% 72%, rgba(255, 255, 255, 0.12), transparent 28%), linear-gradient(145deg, #222324 0%, #111827 100%)',
            overflow: 'hidden',
          }}
        >
          <div
            className="vmrogue-login-orb"
            style={{
              position: 'absolute',
              right: '-90px',
              top: '-90px',
              width: '260px',
              height: '260px',
              borderRadius: '999px',
              border: '1px solid rgba(255, 255, 255, 0.12)',
            }}
            aria-hidden
          />
          <div
            style={{
              position: 'absolute',
              inset: 0,
              backgroundImage: 'linear-gradient(rgba(255,255,255,0.045) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.045) 1px, transparent 1px)',
              backgroundSize: '44px 44px',
              maskImage: 'linear-gradient(120deg, black 0%, transparent 78%)',
            }}
            aria-hidden
          />

          <div style={{ position: 'relative', zIndex: 1 }}>
            <div style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: '10px',
              padding: '8px 12px',
              borderRadius: '999px',
              backgroundColor: 'rgba(255, 255, 255, 0.1)',
              border: '1px solid rgba(255, 255, 255, 0.16)',
              fontSize: '12px',
              fontWeight: 700,
              letterSpacing: '0.08em',
              textTransform: 'uppercase',
            }}>
              <span style={{
                width: '8px',
                height: '8px',
                borderRadius: '999px',
                backgroundColor: '#f0583a',
                boxShadow: '0 0 0 6px rgba(240, 88, 58, 0.14)',
              }} />
              Fleet operations
            </div>

            <h1 style={{
              margin: '32px 0 16px 0',
              fontSize: 'clamp(40px, 6vw, 72px)',
              fontWeight: '800',
              lineHeight: 0.95,
              letterSpacing: '-0.06em',
            }}>
              VMRogue
            </h1>
            <p style={{
              margin: 0,
              maxWidth: '520px',
              color: 'rgba(255, 255, 255, 0.74)',
              fontSize: '17px',
              lineHeight: 1.7,
            }}>
              Secure access to KubeVirt VM lifecycle, remote access, and workflow telemetry for platform teams.
            </p>
          </div>

          <div style={{
            position: 'relative',
            zIndex: 1,
            display: 'grid',
            gap: '12px',
          }}>
            {features.map((feature, index) => (
              <div
                key={feature.title}
                className={`vmrogue-login-feature vmrogue-login-delay-${index + 1}`}
                style={{
                  display: 'grid',
                  gridTemplateColumns: '36px 1fr',
                  gap: '12px',
                  alignItems: 'start',
                  padding: '16px',
                  borderRadius: '18px',
                  backgroundColor: 'rgba(255, 255, 255, 0.08)',
                  border: '1px solid rgba(255, 255, 255, 0.12)',
                  backdropFilter: 'blur(12px)',
                }}
              >
                <span style={{
                  width: '36px',
                  height: '36px',
                  borderRadius: '12px',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  backgroundColor: index === 0 ? '#f0583a' : 'rgba(255, 255, 255, 0.12)',
                  color: '#fff',
                  fontWeight: 800,
                }}>
                  {index + 1}
                </span>
                <span>
                  <strong style={{ display: 'block', fontSize: '14px', marginBottom: '3px' }}>
                    {feature.title}
                  </strong>
                  <span style={{ display: 'block', color: 'rgba(255, 255, 255, 0.64)', fontSize: '13px', lineHeight: 1.5 }}>
                    {feature.description}
                  </span>
                </span>
              </div>
            ))}
          </div>
        </section>

        <section style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: '42px',
          backgroundColor: 'rgba(255, 255, 255, 0.86)',
        }}>
          <div style={{ width: '100%', maxWidth: '400px' }}>
            <div style={{ marginBottom: '30px' }}>
              <span style={{
                display: 'inline-flex',
                alignItems: 'center',
                padding: '5px 10px',
                borderRadius: '999px',
                backgroundColor: 'rgba(240, 88, 58, 0.1)',
                color: '#d94b32',
                fontSize: '12px',
                fontWeight: 700,
              }}>
                Operator sign in
              </span>
              <h2 style={{
                fontSize: '30px',
                fontWeight: '800',
                color: '#111827',
                letterSpacing: '-0.04em',
                margin: '16px 0 8px 0',
              }}>
                Welcome back
              </h2>
              <p style={{
                fontSize: '14px',
                color: '#6b7280',
                margin: 0,
                lineHeight: 1.6,
              }}>
                Use your VMRogue credentials to continue to the dashboard.
              </p>
            </div>

            {error && (
              <div
                role="alert"
                style={{
                  padding: '12px 14px',
                  backgroundColor: '#fef2f2',
                  color: '#991b1b',
                  border: '1px solid #fecaca',
                  borderRadius: '12px',
                  marginBottom: '18px',
                  fontSize: '13px',
                  fontWeight: '600',
                  lineHeight: 1.45,
                }}
              >
                {error}
              </div>
            )}

            <form onSubmit={handleSubmit}>
              <div style={{ marginBottom: '16px' }}>
                <label htmlFor="username" style={labelStyle}>
                  Username
                </label>
                <input
                  id="username"
                  type="text"
                  value={username}
                  onChange={(e) => {
                    setUsername(e.target.value);
                    if (error) setError(null);
                  }}
                  autoComplete="username"
                  required
                  disabled={isLoading}
                  style={inputStyle}
                  onFocus={(e) => {
                    e.currentTarget.style.borderColor = '#f0583a';
                    e.currentTarget.style.boxShadow = '0 0 0 4px rgba(240, 88, 58, 0.12)';
                    e.currentTarget.style.outline = 'none';
                  }}
                  onBlur={(e) => {
                    e.currentTarget.style.borderColor = 'rgba(34, 35, 36, 0.14)';
                    e.currentTarget.style.boxShadow = '0 1px 2px rgba(15, 23, 42, 0.04)';
                  }}
                />
              </div>

              <div style={{ marginBottom: '16px' }}>
                <label htmlFor="password" style={labelStyle}>
                  Password
                </label>
                <div style={{ position: 'relative' }}>
                  <input
                    id="password"
                    type={showPassword ? 'text' : 'password'}
                    value={password}
                    onChange={(e) => {
                      setPassword(e.target.value);
                      if (error) setError(null);
                    }}
                    autoComplete="current-password"
                    required
                    disabled={isLoading}
                    style={{ ...inputStyle, paddingRight: '74px' }}
                    onFocus={(e) => {
                      e.currentTarget.style.borderColor = '#f0583a';
                      e.currentTarget.style.boxShadow = '0 0 0 4px rgba(240, 88, 58, 0.12)';
                      e.currentTarget.style.outline = 'none';
                    }}
                    onBlur={(e) => {
                      e.currentTarget.style.borderColor = 'rgba(34, 35, 36, 0.14)';
                      e.currentTarget.style.boxShadow = '0 1px 2px rgba(15, 23, 42, 0.04)';
                    }}
                  />
                  <button
                    type="button"
                    onClick={() => setShowPassword((current) => !current)}
                    disabled={isLoading}
                    style={{
                      position: 'absolute',
                      top: '50%',
                      right: '10px',
                      transform: 'translateY(-50%)',
                      border: 'none',
                      background: 'transparent',
                      color: '#6b7280',
                      fontSize: '12px',
                      fontWeight: 700,
                      cursor: isLoading ? 'not-allowed' : 'pointer',
                      padding: '6px 8px',
                    }}
                  >
                    {showPassword ? 'Hide' : 'Show'}
                  </button>
                </div>
              </div>

              <div style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                gap: '16px',
                marginBottom: '22px',
              }}>
                <label style={{
                  display: 'flex',
                  alignItems: 'center',
                  cursor: isLoading ? 'not-allowed' : 'pointer',
                  userSelect: 'none',
                }}>
                  <input
                    type="checkbox"
                    checked={rememberMe}
                    onChange={(e) => setRememberMe(e.target.checked)}
                    disabled={isLoading}
                    style={{
                      width: '16px',
                      height: '16px',
                      marginRight: '9px',
                      cursor: isLoading ? 'not-allowed' : 'pointer',
                      accentColor: '#f0583a',
                    }}
                  />
                  <span style={{
                    fontSize: '13px',
                    color: '#4b5563',
                    fontWeight: '600',
                  }}>
                    Remember username
                  </span>
                </label>
              </div>

              <button
                type="submit"
                disabled={!canSubmit}
                className="vmrogue-login-submit"
                style={{
                  width: '100%',
                  padding: '13px 16px',
                  backgroundColor: canSubmit ? '#f0583a' : '#9ca3af',
                  color: '#fff',
                  border: 'none',
                  borderRadius: '12px',
                  fontSize: '15px',
                  fontWeight: '800',
                  cursor: canSubmit ? 'pointer' : 'not-allowed',
                  boxShadow: canSubmit ? '0 14px 28px rgba(240, 88, 58, 0.28)' : 'none',
                  transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
                }}
              >
                {isLoading ? 'Signing in...' : 'Sign in'}
              </button>
            </form>

            <div style={{
              marginTop: '22px',
              padding: '14px',
              borderRadius: '14px',
              backgroundColor: '#f9fafb',
              border: '1px solid #eef0f4',
            }}>
              <p style={{
                margin: 0,
                color: '#6b7280',
                fontSize: '12px',
                lineHeight: 1.55,
              }}>
                Passwords are not stored in this browser. Use logout from the dashboard header when switching operators.
              </p>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
};
