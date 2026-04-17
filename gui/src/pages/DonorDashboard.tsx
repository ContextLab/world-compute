import React, { useEffect, useState } from "react";

// Tauri invoke API — falls back to fetch for web builds
async function invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  if ((window as any).__TAURI__) {
    return (window as any).__TAURI__.invoke(cmd, args);
  }
  // Fallback: call REST gateway
  const resp = await fetch(`/api/v1/donor/status`);
  return resp.json();
}

interface DonorStatus {
  status: string;
  state: string;
  credit_balance_ncu: number;
  trust_score: number;
  uptime_secs: number;
  active_leases: number;
  peer_id: string | null;
}

export default function DonorDashboard() {
  const [data, setData] = useState<DonorStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke("get_donor_status")
      .then((d) => setData(d as DonorStatus))
      .catch((e) => setError(String(e)));
  }, []);

  if (error) return <div style={{ color: "#f85149" }}>Error: {error}</div>;
  if (!data) return <div>Loading donor status...</div>;

  return (
    <div>
      <h1 style={{ marginBottom: "16px" }}>Donor Dashboard</h1>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: "16px" }}>
        <Card label="Credit Balance" value={`${data.credit_balance_ncu.toFixed(6)} NCU`} />
        <Card label="Trust Score" value={data.trust_score.toFixed(4)} />
        <Card label="State" value={data.state} />
        <Card label="Active Leases" value={String(data.active_leases)} />
        <Card label="Uptime" value={`${data.uptime_secs}s`} />
        <Card label="Peer ID" value={data.peer_id ?? "not enrolled"} />
      </div>
    </div>
  );
}

function Card({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ background: "#161b22", border: "1px solid #30363d", borderRadius: "8px", padding: "16px" }}>
      <div style={{ fontSize: "12px", color: "#8b949e", marginBottom: "4px" }}>{label}</div>
      <div style={{ fontSize: "20px", fontWeight: 600 }}>{value}</div>
    </div>
  );
}
