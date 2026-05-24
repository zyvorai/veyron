/** Session-scoped API key auth for dashboard-next (never persisted to localStorage). */

export const API_KEY_STORAGE = 'vmrogue_api_key';
export const AUTH_SESSION_KEY = 'vmrogue_session_authed';
export const DISPLAY_USER_KEY = 'vmrogue_display_user';

export function getApiKey(): string | null {
  if (typeof sessionStorage === 'undefined') return null;
  return sessionStorage.getItem(API_KEY_STORAGE);
}

export function isAuthenticated(): boolean {
  return sessionStorage.getItem(AUTH_SESSION_KEY) === 'true' && !!getApiKey();
}

export function setAuthSession(apiKey: string, username: string): void {
  sessionStorage.setItem(API_KEY_STORAGE, apiKey);
  sessionStorage.setItem(AUTH_SESSION_KEY, 'true');
  sessionStorage.setItem(DISPLAY_USER_KEY, username);
}

export function clearAuthSession(): void {
  sessionStorage.removeItem(API_KEY_STORAGE);
  sessionStorage.removeItem(AUTH_SESSION_KEY);
  sessionStorage.removeItem(DISPLAY_USER_KEY);
}

export function getDisplayUser(): string {
  return sessionStorage.getItem(DISPLAY_USER_KEY) ?? '';
}
