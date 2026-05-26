// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useCallback } from 'react';
import type { Metrics } from '../types/metrics';

interface UseMetricsHistoryReturn {
  history: Metrics[];
  addMetrics: (metrics: Metrics) => void;
  clearHistory: () => void;
}

export function useMetricsHistory(maxPoints = 100): UseMetricsHistoryReturn {
  const [history, setHistory] = useState<Metrics[]>([]);

  const addMetrics = useCallback(
    (metrics: Metrics) => {
      setHistory((prev) => {
        const updated = [...prev, metrics];
        // Keep only the last maxPoints entries
        return updated.slice(-maxPoints);
      });
    },
    [maxPoints]
  );

  const clearHistory = useCallback(() => {
    setHistory([]);
  }, []);

  return { history, addMetrics, clearHistory };
}
