import React, { useState } from "react";

async function invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  if ((window as any).__TAURI__) {
    return (window as any).__TAURI__.invoke(cmd, args);
  }
  const resp = await fetch(`/api/v1/job/submit`, { method: "POST", body: JSON.stringify(args) });
  return resp.json();
}

interface JobResult {
  status: string;
  job_id?: string;
  state?: string;
  message?: string;
}

export default function SubmitterDashboard() {
  const [manifest, setManifest] = useState('{\n  "name": "my-job",\n  "wasm_cid": "",\n  "workload_class": "BatchCpu"\n}');
  const [results, setResults] = useState<JobResult[]>([]);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async () => {
    setSubmitting(true);
    try {
      const result = await invoke("submit_job", { manifest_json: manifest }) as JobResult;
      setResults((prev) => [result, ...prev]);
    } catch (e) {
      setResults((prev) => [{ status: "error", message: String(e) }, ...prev]);
    }
    setSubmitting(false);
  };

  return (
    <div>
      <h1 style={{ marginBottom: "16px" }}>Submit Job</h1>
      <div style={{ marginBottom: "16px" }}>
        <label style={{ display: "block", marginBottom: "8px", color: "#8b949e" }}>Job Manifest (JSON)</label>
        <textarea
          value={manifest}
          onChange={(e) => setManifest(e.target.value)}
          rows={8}
          style={{
            width: "100%", maxWidth: "600px", background: "#0d1117", color: "#e1e4e8",
            border: "1px solid #30363d", borderRadius: "6px", padding: "12px",
            fontFamily: "monospace", fontSize: "14px"
          }}
        />
      </div>
      <button
        onClick={handleSubmit}
        disabled={submitting}
        style={{
          background: "#238636", color: "#fff", border: "none", borderRadius: "6px",
          padding: "8px 24px", fontSize: "14px", cursor: "pointer"
        }}
      >
        {submitting ? "Submitting..." : "Submit Job"}
      </button>

      {results.length > 0 && (
        <div style={{ marginTop: "24px" }}>
          <h2 style={{ marginBottom: "12px" }}>Results</h2>
          {results.map((r, i) => (
            <div key={i} style={{
              background: "#161b22", border: "1px solid #30363d", borderRadius: "6px",
              padding: "12px", marginBottom: "8px"
            }}>
              <span style={{ color: r.status === "ok" ? "#3fb950" : "#f85149" }}>{r.status}</span>
              {r.job_id && <span style={{ marginLeft: "12px" }}>ID: {r.job_id}</span>}
              {r.state && <span style={{ marginLeft: "12px" }}>State: {r.state}</span>}
              {r.message && <span style={{ marginLeft: "12px" }}>{r.message}</span>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
