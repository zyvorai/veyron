import React, { useEffect, useMemo, useState } from 'react';
import {
  Loader2,
  Eye,
  EyeOff,
  User,
  Lock,
  ArrowRight,
  Server,
  Terminal,
  Activity,
  CheckCircle,
  Info,
} from 'lucide-react';
import { ZyvorAboutModal } from './ZyvorAboutModal';
import { ZyvorFooter } from './ZyvorBrand';
import {
  PremiumLoginShell,
  LoginError,
  LoginField,
  LoginSubmit,
  LoginRemember,
  type PremiumLoginFeature,
} from './PremiumLoginShell';

interface LoginProps {
  onLogin: (username: string, password: string) => Promise<void>;
}

const USERNAME_STORAGE_KEY = 'vmrogue_dashboard_username';
const PASSWORD_STORAGE_KEY = 'vmrogue_dashboard_password';
const REMEMBER_STORAGE_KEY = 'vmrogue_dashboard_remember';

const features: PremiumLoginFeature[] = [
  {
    icon: <Server className="w-5 h-5 text-orange-100" />,
    gradient: 'from-orange-500/95 to-red-800/95',
    glow: 'shadow-orange-500/25',
    title: 'KubeVirt inventory',
    description: 'Browse namespaces, VMs, and cluster state from a focused operations console.',
    highlight: true,
  },
  {
    icon: <Terminal className="w-5 h-5 text-orange-100" />,
    gradient: 'from-orange-500/95 to-rose-700/95',
    glow: 'shadow-orange-500/25',
    title: 'Remote access ready',
    description: 'Jump into console, SSH, RDP, and workflow actions when the fleet needs attention.',
  },
  {
    icon: <Activity className="w-5 h-5 text-amber-100" />,
    gradient: 'from-amber-500/95 to-orange-800/95',
    glow: 'shadow-amber-500/25',
    title: 'Operator workflows',
    description: 'Track jobs, signals, and provider health without losing context.',
  },
];

export const Login: React.FC<LoginProps> = ({ onLogin }) => {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [rememberMe, setRememberMe] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [aboutOpen, setAboutOpen] = useState(false);

  useEffect(() => {
    document.title = 'Sign in · VMRogue';
    const savedUsername = localStorage.getItem(USERNAME_STORAGE_KEY);
    const remembered = localStorage.getItem(REMEMBER_STORAGE_KEY) === 'true';

    // Older builds stored passwords in localStorage; remove them when the page loads.
    localStorage.removeItem(PASSWORD_STORAGE_KEY);

    if (savedUsername && remembered) {
      setUsername(savedUsername);
      setRememberMe(true);
    }

    return () => {
      document.title = 'VMRogue';
    };
  }, []);

  const canSubmit = useMemo(
    () => username.trim().length > 0 && password.length > 0 && !isLoading,
    [username, password, isLoading],
  );

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

  return (
    <PremiumLoginShell
      accent="orange"
      logo={
        <div className="w-14 h-14 rounded-2xl flex items-center justify-center bg-gradient-to-br from-orange-500 via-orange-600 to-red-800 shadow-lg shadow-orange-600/30 border border-white/20">
          <span className="text-xl font-extrabold text-white tracking-tight">VR</span>
        </div>
      }
      productName="VMRogue"
      productSubtitle="Fleet operations"
      heroHeadline={
        <>
          KubeVirt lifecycle,
          <br />
          <span className="login-text-gradient">remote access, workflows</span>
        </>
      }
      heroSubheadline="Secure access to KubeVirt VM lifecycle, remote access, and workflow telemetry for platform teams."
      pills={[
        { label: 'Operator sign in', glow: true },
        { label: 'Console · SSH · RDP' },
        { label: 'Job telemetry' },
      ]}
      features={features}
      mobileSubtitle="Operator sign in"
      panelSubtitle="Use your VMRogue credentials to continue to the dashboard"
      panelHint="Passwords are not stored in this browser. Use logout from the dashboard header when switching operators."
      footer={<ZyvorFooter />}
    >
      <form onSubmit={handleSubmit}>
        {error ? <LoginError message={error} /> : null}

        <div className="space-y-5">
          <LoginField label="Username" id="username">
            <User className="login-field-icon" />
            <input
              id="username"
              type="text"
              value={username}
              onChange={(e) => {
                setUsername(e.target.value);
                if (error) setError(null);
              }}
              className="login-input"
              autoComplete="username"
              autoFocus
              placeholder="Operator username"
              required
              disabled={isLoading}
            />
          </LoginField>

          <LoginField label="Password" id="password">
            <Lock className="login-field-icon" />
            <input
              id="password"
              type={showPassword ? 'text' : 'password'}
              value={password}
              onChange={(e) => {
                setPassword(e.target.value);
                if (error) setError(null);
              }}
              className="login-input pr-11"
              autoComplete="current-password"
              placeholder="API key or password"
              required
              disabled={isLoading}
            />
            <button
              type="button"
              onClick={() => setShowPassword((current) => !current)}
              disabled={isLoading}
              className="absolute right-3.5 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300 transition-colors disabled:cursor-not-allowed"
              aria-label={showPassword ? 'Hide password' : 'Show password'}
            >
              {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </LoginField>
        </div>

        <LoginRemember
          checked={rememberMe}
          onChange={setRememberMe}
          label="Remember username"
        />

        <LoginSubmit loading={isLoading} disabled={!canSubmit} className="mt-7">
          {isLoading ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin relative z-10" />
              <span className="relative z-10">Signing in…</span>
            </>
          ) : (
            <>
              <span className="relative z-10">Sign in</span>
              <ArrowRight className="h-4 w-4 relative z-10 group-hover:translate-x-0.5 transition-transform" />
            </>
          )}
        </LoginSubmit>

        <div className="mt-6 pt-5 border-t border-slate-700/30 flex items-center justify-center gap-2 text-xs text-slate-500">
          <CheckCircle className="h-3.5 w-3.5 text-emerald-500/70" />
          <span>Secured with VMRogue API key authentication</span>
        </div>
        <div className="mt-3 flex items-center justify-center">
          <button
            type="button"
            onClick={() => setAboutOpen(true)}
            className="inline-flex items-center gap-1.5 text-xs text-slate-500 hover:text-slate-300 transition-colors"
          >
            <Info className="h-3.5 w-3.5" />
            About VMRogue
          </button>
        </div>
      </form>
      {aboutOpen ? (
        <ZyvorAboutModal
          product="VMRogue"
          productTagline="KubeVirt fleet operations console for Kubernetes clusters."
          onClose={() => setAboutOpen(false)}
          extraLinks={[
            {
              label: 'Full dashboard',
              href: '/dashboard',
              description: 'Classic SPA with VNC, snapshots, and platform pages',
            },
          ]}
        />
      ) : null}
    </PremiumLoginShell>
  );
};
