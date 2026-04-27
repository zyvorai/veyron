import React, { useState, useEffect } from 'react';

interface LoginProps {
  onLogin: (username: string, password: string) => Promise<void>;
}

export const Login: React.FC<LoginProps> = ({ onLogin }) => {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [rememberMe, setRememberMe] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load saved credentials on mount
  useEffect(() => {
    const savedUsername = localStorage.getItem("vmrogue_dashboard_username");
    const savedPassword = localStorage.getItem("vmrogue_dashboard_password");
    if (savedUsername && savedPassword) {
      setUsername(savedUsername);
      setPassword(savedPassword);
      setRememberMe(true);
    }
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError(null);

    try {
      await onLogin(username, password);

      // Save credentials if "Remember Me" is checked
      if (rememberMe) {
        localStorage.setItem("vmrogue_dashboard_username", username);
        localStorage.setItem("vmrogue_dashboard_password", password);
        localStorage.setItem("vmrogue_dashboard_remember", "true");
      } else {
        localStorage.removeItem("vmrogue_dashboard_username");
        localStorage.removeItem("vmrogue_dashboard_password");
        localStorage.removeItem("vmrogue_dashboard_remember");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setIsLoading(false);
    }
  };

  const inputStyle: React.CSSProperties = {
    width: '100%',
    padding: '10px 14px',
    border: '1px solid #d1d5db',
    borderRadius: '4px',
    fontSize: '14px',
    backgroundColor: '#fff',
    color: '#000',
    transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
  };

  const labelStyle: React.CSSProperties = {
    display: 'block',
    fontSize: '13px',
    fontWeight: '600',
    color: '#374151',
    marginBottom: '6px',
  };

  return (
    <div style={{
      minHeight: '100vh',
      backgroundColor: '#f0f2f7',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      padding: '20px',
    }}>
      <div style={{
        backgroundColor: '#fff',
        borderRadius: '8px',
        padding: '32px 28px',
        width: '100%',
        maxWidth: '380px',
        border: '1px solid #e5e7eb',
        boxShadow: '0 1px 3px rgba(0,0,0,0.1)',
      }}>
        {/* Logo/Title */}
        <div style={{ textAlign: 'center', marginBottom: '28px' }}>
          <h1 style={{
            fontSize: '24px',
            fontWeight: '700',
            color: '#111827',
            margin: '0 0 4px 0',
          }}>
            VMRogue
          </h1>
          <p style={{
            fontSize: '14px',
            color: '#6b7280',
            margin: 0,
          }}>
            Sign in to your account
          </p>
        </div>

        {/* Error Message */}
        {error && (
          <div style={{
            padding: '10px 12px',
            backgroundColor: '#fee2e2',
            color: '#991b1b',
            borderRadius: '6px',
            marginBottom: '18px',
            fontSize: '13px',
            fontWeight: '500',
          }}>
            {error}
          </div>
        )}

        {/* Login Form */}
        <form onSubmit={handleSubmit}>
          <div style={{ marginBottom: '16px' }}>
            <label htmlFor="username" style={labelStyle}>
              Username
            </label>
            <input
              id="username"
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
              disabled={isLoading}
              style={inputStyle}
              onFocus={(e) => {
                e.currentTarget.style.borderColor = '#f0583a';
                e.currentTarget.style.outline = 'none';
              }}
              onBlur={(e) => {
                e.currentTarget.style.borderColor = '#d1d5db';
              }}
            />
          </div>

          <div style={{ marginBottom: '16px' }}>
            <label htmlFor="password" style={labelStyle}>
              Password
            </label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              disabled={isLoading}
              style={inputStyle}
              onFocus={(e) => {
                e.currentTarget.style.borderColor = '#f0583a';
                e.currentTarget.style.outline = 'none';
              }}
              onBlur={(e) => {
                e.currentTarget.style.borderColor = '#d1d5db';
              }}
            />
          </div>

          {/* Remember Me Checkbox */}
          <div style={{ marginBottom: '20px' }}>
            <label style={{
              display: 'flex',
              alignItems: 'center',
              cursor: 'pointer',
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
                  marginRight: '8px',
                  cursor: isLoading ? 'not-allowed' : 'pointer',
                  accentColor: '#f0583a',
                }}
              />
              <span style={{
                fontSize: '13px',
                color: '#6b7280',
                fontWeight: '500',
              }}>
                Remember me
              </span>
            </label>
          </div>

          <button
            type="submit"
            disabled={isLoading}
            onMouseEnter={(e) => {
              if (!isLoading) {
                e.currentTarget.style.backgroundColor = '#d94b32';
                e.currentTarget.style.transform = 'translateY(-1px)';
              }
            }}
            onMouseLeave={(e) => {
              if (!isLoading) {
                e.currentTarget.style.backgroundColor = '#f0583a';
                e.currentTarget.style.transform = 'translateY(0)';
              }
            }}
            style={{
              width: '100%',
              padding: '11px',
              backgroundColor: isLoading ? '#9ca3af' : '#f0583a',
              color: '#fff',
              border: 'none',
              borderRadius: '6px',
              fontSize: '14px',
              fontWeight: '600',
              cursor: isLoading ? 'not-allowed' : 'pointer',
              transition: 'all 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
            }}
          >
            {isLoading ? 'Signing in...' : 'Sign in'}
          </button>
        </form>

        {/* Footer Links */}
        <div style={{
          marginTop: '20px',
          paddingTop: '16px',
          borderTop: '1px solid #e5e7eb',
          textAlign: 'center',
        }}>
          <a
            href="#"
            style={{
              color: '#6b7280',
              fontSize: '13px',
              textDecoration: 'none',
              transition: 'color 0.25s cubic-bezier(0.215, 0.61, 0.355, 1)',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.color = '#f0583a';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.color = '#6b7280';
            }}
          >
            Forgot password?
          </a>
        </div>
      </div>
    </div>
  );
};
