// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState, type CSSProperties } from "react";
import { fetchVmDrift, type VmDriftRecord } from "../../lib/api";

type Props = {
  namespace: string;
  vmName: string;
  initialDrift?: boolean | null;
  initialMessage?: string | null;
};

export function VmDriftPanel({ namespace, vmName, initialDrift, initialMessage }: Props) {
  const [detail, setDetail] = useState<VmDriftRecord | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    void fetchVmDrift(namespace, vmName)
      .then((row) => {
        setDetail(row);
        setError(null);
      })
      .catch((e) => {
        setDetail(null);
        setError(e instanceof Error ? e.message : "Drift status unavailable");
      })
      .finally(() => setLoading(false));
  }, [namespace, vmName]);

  const drift = detail?.drift_detected ?? initialDrift ?? null;
  const message = detail?.drift_message ?? initialMessage ?? null;

  if (loading && drift === null && !error) {
    return <p style={muted}>Checking operator drift status…</p>;
  }

  if (error && drift === null) {
    return (
      <div style={wrap}>
        <h4 style={title}>Operator drift</h4>
        <p style={muted}>Not operator-managed (no VMRogueVM CR) or drift API unavailable.</p>
      </div>
    );
  }

  return (
    <div style={wrap}>
      <h4 style={title}>Operator drift</h4>
      <div style={row}>
        <span style={drift ? badgeWarn : badgeOk}>{drift ? "Drift detected" : "In sync"}</span>
        {detail?.template ? (
          <span style={meta}>
            template <code style={code}>{detail.template}</code>
            {detail.profile ? (
              <>
                {" "}
                · profile <code style={code}>{detail.profile}</code>
              </>
            ) : null}
          </span>
        ) : null}
      </div>
      {message ? <p style={messageStyle}>{message}</p> : null}
      {detail?.resolved_spec_hash ? (
        <p style={meta}>
          Spec hash: <code style={code}>{detail.resolved_spec_hash.slice(0, 16)}…</code>
        </p>
      ) : null}
    </div>
  );
}

export default VmDriftPanel;

const wrap: CSSProperties = {
  marginTop: 12,
  padding: "12px 14px",
  borderRadius: 8,
  border: "1px solid #e5e7eb",
  background: "#fafafa",
};
const title: CSSProperties = { margin: "0 0 8px", fontSize: 14, fontWeight: 700, color: "#374151" };
const row: CSSProperties = { display: "flex", flexWrap: "wrap", gap: 10, alignItems: "center" };
const badgeOk: CSSProperties = {
  display: "inline-block",
  padding: "2px 10px",
  borderRadius: 999,
  fontSize: 12,
  fontWeight: 600,
  background: "#dcfce7",
  color: "#166534",
};
const badgeWarn: CSSProperties = {
  ...badgeOk,
  background: "#fef3c7",
  color: "#92400e",
};
const messageStyle: CSSProperties = { margin: "8px 0 0", fontSize: 13, color: "#374151", lineHeight: 1.45 };
const meta: CSSProperties = { fontSize: 12, color: "#6b7280" };
const muted: CSSProperties = { margin: 0, fontSize: 13, color: "#9ca3af" };
const code: CSSProperties = { fontFamily: "ui-monospace, monospace", fontSize: 11 };
