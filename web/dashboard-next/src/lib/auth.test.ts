import { afterEach, describe, expect, it } from 'vitest';
import {
  API_KEY_STORAGE,
  AUTH_SESSION_KEY,
  DISPLAY_USER_KEY,
  clearAuthSession,
  getApiKey,
  getDisplayUser,
  isAuthenticated,
  setAuthSession,
} from './auth';

describe('auth session', () => {
  afterEach(() => {
    sessionStorage.clear();
  });

  it('starts unauthenticated', () => {
    expect(isAuthenticated()).toBe(false);
    expect(getApiKey()).toBeNull();
    expect(getDisplayUser()).toBe('');
  });

  it('stores API key in sessionStorage only', () => {
    setAuthSession('secret-key', 'operator');
    expect(sessionStorage.getItem(API_KEY_STORAGE)).toBe('secret-key');
    expect(sessionStorage.getItem(AUTH_SESSION_KEY)).toBe('true');
    expect(getDisplayUser()).toBe('operator');
    expect(isAuthenticated()).toBe(true);
  });

  it('clears session on logout', () => {
    setAuthSession('secret-key', 'operator');
    clearAuthSession();
    expect(sessionStorage.getItem(API_KEY_STORAGE)).toBeNull();
    expect(sessionStorage.getItem(AUTH_SESSION_KEY)).toBeNull();
    expect(sessionStorage.getItem(DISPLAY_USER_KEY)).toBeNull();
    expect(isAuthenticated()).toBe(false);
  });
});
