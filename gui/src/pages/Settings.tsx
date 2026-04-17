import React, { useEffect, useState } from "react";

async function invoke(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  if ((window as any).__TAURI__) {
    return (window as any).__TAURI__.invoke(cmd, args);
  }
  const resp = await fetch(`/api/v1/donor/status`);
  return resp.json();
}

interface SettingsData {
  workload_classes: Record<string, boolean>;
  cpu_cap_percent: number;
  memory_cap_mb: number;
  storage_cap_gb: number;
  network_egress_enabled: boolean;
}

export default function Settings() {
  const [settings, setSettings] = useState<SettingsData | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke("get_settings")
      .then((d: any) => setSettings({
        workload_classes: d.workload_classes,
        cpu_cap_percent: d.cpu_cap_percent,
        memory_cap_mb: d.memory_cap_mb,
        storage_cap_gb: d.storage_cap_gb,
        network_egress_enabled: d.network_egress_enabled,
      }))
      .catch((e) => setError(String(e)));
  }, []);

  const handleToggle = (key: string) => {
    if (!settings) return;
    setSettings({
      ...settings,
      workload_classes: { ...settings.workload_classes, [key]: !settings.workload_classes[key] }
    });
  };

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await invoke("update_settings", { settings_json: JSON.stringify(settings) });
    } catch (e) {
      setError(String(e));
    }
    setSaving(false);
  };

  if (error) return <div style={{ color: "#f85149" }}>Error: {error}</div>;
  if (!settings) return <div>Loading settings...</div>;

  return (
    <div>
      <h1 style={{ marginBottom: "16px" }}>Settings</h1>

      <section style={{ marginBottom: "24px" }}>
        <h2 style={{ fontSize: "16px", marginBottom: "12px" }}>Workload Classes</h2>
        {Object.entries(settings.workload_classes).map(([key, enabled]) => (
          <label key={key} style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "8px", cursor: "pointer" }}>
            <input type="checkbox" checked={enabled} onChange={() => handleToggle(key)} />
            <span>{key.replace(/_/g, " ")}</span>
          </label>
        ))}
      </section>

      <section style={{ marginBottom: "24px" }}>
        <h2 style={{ fontSize: "16px", marginBottom: "12px" }}>Resource Caps</h2>
        <div style={{ marginBottom: "12px" }}>
          <label style={{ display: "block", color: "#8b949e", marginBottom: "4px" }}>
            CPU Cap: {settings.cpu_cap_percent}%
          </label>
          <input
            type="range" min={10} max={100} value={settings.cpu_cap_percent}
            onChange={(e) => setSettings({ ...settings, cpu_cap_percent: Number(e.target.value) })}
            style={{ width: "300px" }}
          />
        </div>
        <div style={{ marginBottom: "12px" }}>
          <label style={{ display: "block", color: "#8b949e", marginBottom: "4px" }}>
            Storage Cap: {settings.storage_cap_gb} GB
          </label>
          <input
            type="range" min={1} max={500} value={settings.storage_cap_gb}
            onChange={(e) => setSettings({ ...settings, storage_cap_gb: Number(e.target.value) })}
            style={{ width: "300px" }}
          />
        </div>
      </section>

      <button
        onClick={handleSave}
        disabled={saving}
        style={{
          background: "#238636", color: "#fff", border: "none", borderRadius: "6px",
          padding: "8px 24px", fontSize: "14px", cursor: "pointer"
        }}
      >
        {saving ? "Saving..." : "Save Settings"}
      </button>
    </div>
  );
}
