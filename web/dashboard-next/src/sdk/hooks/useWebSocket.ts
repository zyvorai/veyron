import { useEffect, useState, useCallback, useRef } from 'react';
import type { Metrics } from '../types/metrics';

interface UseWebSocketOptions {
  url: string;
  reconnectInterval?: number;
  reconnectAttempts?: number;
}

interface UseWebSocketReturn {
  data: Metrics | null;
  connected: boolean;
  error: Error | null;
  reconnecting: boolean;
  sendMessage: (message: unknown) => void;
}

export function useWebSocket({
  url,
  reconnectInterval = 3000,
  reconnectAttempts = 10,
}: UseWebSocketOptions): UseWebSocketReturn {
  const [data, setData] = useState<Metrics | null>(null);
  const [connected, setConnected] = useState(false);
  const [reconnecting, setReconnecting] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectCountRef = useRef(0);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const connect = useCallback(() => {
    try {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        console.log('WebSocket connected');
        setConnected(true);
        setReconnecting(false);
        setError(null);
        reconnectCountRef.current = 0;
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);

          // Handle wrapped message format from server
          if (message.type === 'metrics' && message.data && message.data.raw) {
            setData(message.data.raw as Metrics);
          } else if (message.type) {
            // Other message types (status, job_update, etc.)
            console.log('Received WebSocket message:', message.type);
          } else {
            // Direct metrics format (fallback)
            setData(message as Metrics);
          }
        } catch (err) {
          console.error('Failed to parse WebSocket message:', err);
          setError(err as Error);
        }
      };

      ws.onerror = (event) => {
        console.error('WebSocket error:', event);
        setError(new Error('WebSocket connection error'));
      };

      ws.onclose = (event) => {
        console.log('WebSocket closed:', event.code, event.reason);
        setConnected(false);

        // Attempt reconnection if not intentionally closed
        if (event.code !== 1000 && reconnectCountRef.current < reconnectAttempts) {
          setReconnecting(true);
          reconnectCountRef.current++;

          console.log(
            `Reconnecting... (attempt ${reconnectCountRef.current}/${reconnectAttempts})`
          );

          reconnectTimeoutRef.current = setTimeout(() => {
            connect();
          }, reconnectInterval);
        } else if (reconnectCountRef.current >= reconnectAttempts) {
          setError(new Error('Maximum reconnection attempts reached'));
          setReconnecting(false);
        }
      };
    } catch (err) {
      console.error('Failed to create WebSocket:', err);
      setError(err as Error);
    }
  }, [url, reconnectInterval, reconnectAttempts]);

  useEffect(() => {
    connect();

    return () => {
      // Cleanup on unmount
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close(1000, 'Component unmounting');
      }
    };
  }, [connect]);

  const sendMessage = useCallback((message: unknown) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    } else {
      console.warn('WebSocket is not connected. Cannot send message.');
    }
  }, []);

  return { data, connected, error, reconnecting, sendMessage };
}
