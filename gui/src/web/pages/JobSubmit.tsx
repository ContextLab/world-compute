import React, { useState } from "react";

const API_BASE = "/api/v1";

interface SubmitResult {
  status: string;
  job_id?: string;
  state?: string;
  message?: string;
}

export default function JobSubmit() {
  const [manifest, setManifest] = useState('{\n  "name": "my-job",\n  "wasm_cid": "",\n  "workload_class": "BatchCpu"\n}');
  const [result, setResult] = useState<SubmitResult | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async () => {
    setSubmitting(true);
    try {
      const resp = await fetch(`${API_BASE}/job/submit`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: manifest,
      });
      const data = await resp.json();
      setResult(data);
    } catch (e) {
      setResult({ status: "error", message: String(e) });
    }
    setSubmitting(false);
  };

  return (
    <div style={{ padding: "24px" }}>
      <h1 style={{ marginBottom: "16px" }}>Submit Job</h1>
      <div style={{ marginBottom: "16px" }}>
        <label style={{ display: "block", color: "#8b949e", marginBottom: "8px" }}>
          Job Manifest (JSON)
        </label>
        <textarea
          value={manifest}
          onChange={(e) => setManifest(e.target.value)}
          rows={8}
          style={{
            width: "100%", maxWidth: "600px", background: "#0d1117", color: "#e1e4e8",
            border: "1px solid #30363d", borderRadius: "6px", padding: "12px",
            fontFamily: "monospace", fontSize: "14px",
          }}
        />
      </div>
      <button
        onClick={handleSubmit}
        disabled={submitting}
        style={{
          background: "#238636", color: "#fff", border: "none", borderRadius: "6px",
          padding: "8px 24px", fontSize: "14px", cursor: "pointer",
        }}
      >
        {submitting ? "Submitting..." : "Submit"}
      </button>

      {result && (
        <div style={{
          marginTop: "16px", background: "#161b22", border: "1px solid #30363d",
          borderRadius: "6px", padding: "12px",
        }}>
          <div style={{ color: result.status === "ok" ? "#3fb950" : "#f85149" }}>
            {result.status}
          </div>
          {result.job_id && <div>Job ID: {result.job_id}</div>}
          {result.state && <div>State: {result.state}</div>}
          {result.message && <div>{result.message}</div>}
        </div>
      )}
    </div>
  );
}
