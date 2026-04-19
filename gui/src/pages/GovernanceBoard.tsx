import React, { useEffect, useState } from "react";

async function invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  if ((window as any).__TAURI__) {
    return (window as any).__TAURI__.invoke(cmd, args);
  }
  const resp = await fetch(`/api/v1/governance/proposals`);
  return resp.json();
}

interface Proposal {
  id: string;
  kind: string;
  title: string;
  status: string;
  votes_for: number;
  votes_against: number;
}

interface ProposalData {
  status: string;
  proposals: Proposal[];
  proposal_kinds: string[];
}

export default function GovernanceBoard() {
  const [data, setData] = useState<ProposalData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [voting, setVoting] = useState<string | null>(null);

  useEffect(() => {
    invoke("get_proposals")
      .then((d) => setData(d as ProposalData))
      .catch((e) => setError(String(e)));
  }, []);

  const handleVote = async (proposalId: string, approve: boolean) => {
    setVoting(proposalId);
    try {
      await invoke("cast_vote", { proposal_id: proposalId, approve });
    } catch (e) {
      setError(String(e));
    }
    setVoting(null);
  };

  if (error) return <div style={{ color: "#f85149" }}>Error: {error}</div>;
  if (!data) return <div>Loading proposals...</div>;

  return (
    <div>
      <h1 style={{ marginBottom: "16px" }}>Governance Board</h1>

      {data.proposals.length === 0 ? (
        <div style={{ color: "#8b949e", padding: "24px", textAlign: "center" }}>
          No active proposals. Proposal kinds: {data.proposal_kinds.join(", ")}
        </div>
      ) : (
        data.proposals.map((p) => (
          <div key={p.id} style={{
            background: "#161b22", border: "1px solid #30363d", borderRadius: "8px",
            padding: "16px", marginBottom: "12px"
          }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: "16px" }}>{p.title}</div>
                <div style={{ color: "#8b949e", fontSize: "12px" }}>{p.kind} — {p.status}</div>
              </div>
              <div style={{ display: "flex", gap: "8px" }}>
                <button
                  onClick={() => handleVote(p.id, true)}
                  disabled={voting === p.id}
                  style={{ background: "#238636", color: "#fff", border: "none", borderRadius: "6px", padding: "6px 16px", cursor: "pointer" }}
                >
                  Approve ({p.votes_for})
                </button>
                <button
                  onClick={() => handleVote(p.id, false)}
                  disabled={voting === p.id}
                  style={{ background: "#da3633", color: "#fff", border: "none", borderRadius: "6px", padding: "6px 16px", cursor: "pointer" }}
                >
                  Reject ({p.votes_against})
                </button>
              </div>
            </div>
          </div>
        ))
      )}
    </div>
  );
}
