/** Session-scoped API key or OIDC bearer auth for dashboard-next. */

export const API_KEY_STORAGE = 'vmrogue_api_key';
export const BEARER_TOKEN_STORAGE = 'vmrogue_token';
export const AUTH_SESSION_KEY = 'vmrogue_session_authed';
export const DISPLAY_USER_KEY = 'vmrogue_display_user';

export function getApiKey(): string | null {
  if (typeof sessionStorage === 'undefined') return null;
  return sessionStorage.getItem(API_KEY_STORAGE);
}

export function getBearerToken(): string | null {
  if (typeof sessionStorage === 'undefined') return null;
  return sessionStorage.getItem(BEARER_TOKEN_STORAGE);
}

export function isAuthenticated(): boolean {
  return (
    sessionStorage.getItem(AUTH_SESSION_KEY) === 'true' &&
    (!!getApiKey() || !!getBearerToken())
  );
}

export function setAuthSession(apiKey: string, username: string): void {
  sessionStorage.setItem(API_KEY_STORAGE, apiKey);
  sessionStorage.removeItem(BEARER_TOKEN_STORAGE);
  sessionStorage.setItem(AUTH_SESSION_KEY, 'true');
  sessionStorage.setItem(DISPLAY_USER_KEY, username);
}

export function setOidcSession(accessToken: string, username: string): void {
  sessionStorage.setItem(BEARER_TOKEN_STORAGE, accessToken);
  sessionStorage.removeItem(API_KEY_STORAGE);
  sessionStorage.setItem(AUTH_SESSION_KEY, 'true');
  sessionStorage.setItem(DISPLAY_USER_KEY, username);
}

export function clearAuthSession(): void {
  sessionStorage.removeItem(API_KEY_STORAGE);
  sessionStorage.removeItem(BEARER_TOKEN_STORAGE);
  sessionStorage.removeItem(AUTH_SESSION_KEY);
  sessionStorage.removeItem(DISPLAY_USER_KEY);
}

export function getDisplayUser(): string {
  return sessionStorage.getItem(DISPLAY_USER_KEY) ?? '';
}
