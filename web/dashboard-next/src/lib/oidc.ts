// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/** OIDC authorization code + PKCE helpers for dashboard-next. */

export type OidcPublicConfig = {
  enabled: boolean;
  issuer: string;
  client_id: string;
  authorization_url: string;
  token_url: string;
  redirect_uri: string;
  scope: string;
};

const PKCE_VERIFIER_KEY = 'vmrogue_oidc_pkce_verifier';
const PKCE_STATE_KEY = 'vmrogue_oidc_state';

function randomString(len: number): string {
  const bytes = new Uint8Array(len);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (b) => (b % 36).toString(36)).join('');
}

async function sha256Base64Url(input: string): Promise<string> {
  const data = new TextEncoder().encode(input);
  const digest = await crypto.subtle.digest('SHA-256', data);
  const bytes = new Uint8Array(digest);
  let binary = '';
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

export async function fetchOidcConfig(): Promise<OidcPublicConfig | null> {
  const res = await fetch('/api/v1/auth/oidc/config');
  if (!res.ok) return null;
  const body = (await res.json()) as OidcPublicConfig;
  return body.enabled ? body : null;
}

export async function startOidcLogin(config: OidcPublicConfig): Promise<void> {
  const verifier = randomString(64);
  const challenge = await sha256Base64Url(verifier);
  const state = randomString(24);
  sessionStorage.setItem(PKCE_VERIFIER_KEY, verifier);
  sessionStorage.setItem(PKCE_STATE_KEY, state);

  const redirect =
    config.redirect_uri.startsWith('http') ?
      config.redirect_uri
    : `${window.location.origin}${config.redirect_uri.startsWith('/') ? '' : '/'}${config.redirect_uri}`;

  const params = new URLSearchParams({
    response_type: 'code',
    client_id: config.client_id,
    redirect_uri: redirect,
    scope: config.scope || 'openid profile email',
    state,
    code_challenge: challenge,
    code_challenge_method: 'S256',
  });
  window.location.href = `${config.authorization_url}?${params.toString()}`;
}

export async function completeOidcCallback(
  config: OidcPublicConfig,
): Promise<{ accessToken: string; username: string } | null> {
  const params = new URLSearchParams(window.location.search);
  const code = params.get('code');
  const state = params.get('state');
  const savedState = sessionStorage.getItem(PKCE_STATE_KEY);
  const verifier = sessionStorage.getItem(PKCE_VERIFIER_KEY);
  if (!code || !state || !verifier || state !== savedState) return null;

  const redirect =
    config.redirect_uri.startsWith('http') ?
      config.redirect_uri
    : `${window.location.origin}${config.redirect_uri.startsWith('/') ? '' : '/'}${config.redirect_uri}`;

  const body = new URLSearchParams({
    grant_type: 'authorization_code',
    client_id: config.client_id,
    code,
    redirect_uri: redirect,
    code_verifier: verifier,
  });

  const res = await fetch(config.token_url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: body.toString(),
  });
  if (!res.ok) return null;
  const token = (await res.json()) as {
    access_token?: string;
    id_token?: string;
  };
  const accessToken = token.access_token ?? token.id_token;
  if (!accessToken) return null;

  sessionStorage.removeItem(PKCE_VERIFIER_KEY);
  sessionStorage.removeItem(PKCE_STATE_KEY);
  window.history.replaceState({}, document.title, window.location.pathname);

  let username = 'oidc-user';
  try {
    const payload = accessToken.split('.')[1];
  if (payload) {
      const json = JSON.parse(atob(payload.replace(/-/g, '+').replace(/_/g, '/')));
      username =
        (json.preferred_username as string) ||
        (json.email as string) ||
        (json.sub as string) ||
        username;
    }
  } catch {
    /* ignore decode errors */
  }

  return { accessToken, username };
}
