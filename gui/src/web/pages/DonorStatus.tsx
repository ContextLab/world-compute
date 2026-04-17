import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

const API_BASE = "/api/v1";

interface DonorStatusData {
  state: string;
  credit_balance: number;
  trust_score: number;
  uptime_secs: number;
}

function DonorStatus() {
  const [data, setData] = useState<DonorStatusData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetch(`${API_BASE}/donor/status`)
      .then((r) => r.json())
      .then((d) => setData(d))
      .catch((e) => setError(String(e)));
  }, []);

  if (error) return <div style={{ color: "#f85149", padding: "24px" }}>Error: {error}</div>;
  if (!data) return <div style={{ padding: "24px" }}>Loading donor status...</div>;

  return (
    <div style={{ padding: "24px" }}>
      <h1 style={{ marginBottom: "16px" }}>Donor Status</h1>
      <table style={{ borderCollapse: "collapse" }}>
        <tbody>
          <Row label="State" value={data.state} />
          <Row label="Credit Balance" value={`${data.credit_balance} NCU`} />
          <Row label="Trust Score" value={String(data.trust_score)} />
          <Row label="Uptime" value={`${data.uptime_secs}s`} />
        </tbody>
      </table>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  const cellStyle = { padding: "8px 16px", borderBottom: "1px solid #30363d" };
  return (
    <tr>
      <td style={{ ...cellStyle, color: "#8b949e" }}>{label}</td>
      <td style={{ ...cellStyle, fontWeight: 600 }}>{value}</td>
    </tr>
  );
}

const container = document.getElementById("root");
if (container) {
  createRoot(container).render(<DonorStatus />);
}

export default DonorStatus;
