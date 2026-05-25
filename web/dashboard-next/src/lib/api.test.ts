import { describe, expect, it } from 'vitest';
import {
  extractApiError,
  parseApiResponse,
  unwrapApiData,
} from './api';

describe('extractApiError', () => {
  it('reads nested error.message', () => {
    expect(
      extractApiError({ error: { message: 'Forbidden' } }, 'Bad Request'),
    ).toBe('Forbidden');
  });

  it('reads string error field', () => {
    expect(extractApiError({ error: 'INVALID_KEY' }, 'Unauthorized')).toBe('INVALID_KEY');
  });

  it('reads top-level message', () => {
    expect(extractApiError({ message: 'Not found' }, 'Not Found')).toBe('Not found');
  });

  it('falls back to response text', () => {
    expect(extractApiError(null, 'Internal Server Error', 'upstream timeout')).toBe('upstream timeout');
  });
});

describe('unwrapApiData', () => {
  it('unwraps ApiResponse envelope', () => {
    expect(unwrapApiData<{ name: string }>({ success: true, data: { name: 'vm-1' } })).toEqual({
      name: 'vm-1',
    });
  });

  it('returns bare arrays unchanged', () => {
    expect(unwrapApiData<string[]>(['a', 'b'])).toEqual(['a', 'b']);
  });
});

describe('parseApiResponse', () => {
  it('throws on HTTP 401 with JSON body', async () => {
    const res = new Response(JSON.stringify({ error: { message: 'Invalid API key' } }), {
      status: 401,
      statusText: 'Unauthorized',
    });
    await expect(parseApiResponse(res)).rejects.toThrow('Invalid API key');
  });

  it('throws on HTTP 500 with plain text', async () => {
    const res = new Response('database unavailable', {
      status: 500,
      statusText: 'Internal Server Error',
    });
    await expect(parseApiResponse(res)).rejects.toThrow('database unavailable');
  });

  it('throws on HTTP 200 success:false', async () => {
    const res = new Response(
      JSON.stringify({ success: false, error: { message: 'Schedule invalid' } }),
      { status: 200, statusText: 'OK' },
    );
    await expect(parseApiResponse(res)).rejects.toThrow('Schedule invalid');
  });

  it('unwraps success envelope data', async () => {
    const res = new Response(JSON.stringify({ success: true, data: [{ name: 'ns1' }] }), {
      status: 200,
      statusText: 'OK',
    });
    await expect(parseApiResponse<Array<{ name: string }>>(res)).resolves.toEqual([{ name: 'ns1' }]);
  });

  it('returns bare JSON objects', async () => {
    const res = new Response(JSON.stringify({ total_cost: 12.5, currency: 'USD' }), {
      status: 200,
      statusText: 'OK',
    });
    await expect(parseApiResponse<{ total_cost: number; currency: string }>(res)).resolves.toEqual({
      total_cost: 12.5,
      currency: 'USD',
    });
  });
});
