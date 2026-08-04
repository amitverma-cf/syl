import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Button, Input } from "../ui";

interface AppSettings {
  autostart: boolean;
  telemetryEnabled: boolean;
  maxConcurrentLocalModels: number;
}

function GeneralTab() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AppSettings>("get_settings")
      .then(setSettings)
      .catch((err) => setError(String(err)));
  }, []);

  async function save(next: AppSettings) {
    setSaving(true);
    setError(null);
    try {
      await invoke("update_settings", { settings: next });
      setSettings(next);
      toast.success("Settings saved");
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    } finally {
      setSaving(false);
    }
  }

  if (!settings) {
    return <p style={{ fontSize: 12, color: "var(--text-3)" }}>Loading…</p>;
  }

  return (
    <div>
      <div className="settings-section-title">Startup</div>
      <label className="form-row" style={{ cursor: "pointer" }}>
        <input
          type="checkbox"
          checked={settings.autostart}
          disabled={saving}
          onChange={(e) => save({ ...settings, autostart: e.currentTarget.checked })}
        />
        <span style={{ fontSize: 12.5 }}>Launch syl automatically when you log in</span>
      </label>

      <div className="settings-section-title">Privacy</div>
      <label className="form-row" style={{ cursor: "pointer" }}>
        <input
          type="checkbox"
          checked={settings.telemetryEnabled}
          disabled={saving}
          onChange={(e) => save({ ...settings, telemetryEnabled: e.currentTarget.checked })}
        />
        <span style={{ fontSize: 12.5 }}>Share anonymous usage telemetry</span>
      </label>

      <div className="settings-section-title">Resource limits</div>
      <div className="form-row">
        <span style={{ width: 220, flexShrink: 0, fontSize: 12, color: "rgba(255,255,255,.7)" }}>
          Max concurrently loaded local models
        </span>
        <Input
          type="number"
          min={1}
          max={16}
          value={settings.maxConcurrentLocalModels}
          disabled={saving}
          style={{ width: 80 }}
          onChange={(e) => {
            const value = Number(e.currentTarget.value);
            if (Number.isFinite(value) && value >= 1) {
              setSettings({ ...settings, maxConcurrentLocalModels: Math.floor(value) });
            }
          }}
        />
        <Button
          disabled={saving}
          onClick={() => save(settings)}
        >
          Save
        </Button>
      </div>

      {error && <p className="settings-error">{error}</p>}
    </div>
  );
}

export default GeneralTab;
