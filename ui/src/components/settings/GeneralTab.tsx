import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Button, Checkbox, Input, Select } from "../ui";
import { useShellStore } from "../../store/shellStore";

interface AppSettings {
  autostart: boolean;
  telemetryEnabled: boolean;
  maxConcurrentLocalModels: number;
  defaultFlowName: string;
  maxToolIterations: number;
  contextBudgetChars: number;
  localEngineContextSize: number;
  localEngineMaxTokens: number;
  imageEngineSteps: number;
  imageEngineWidth: number;
  imageEngineHeight: number;
}

function GeneralTab() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { theme, setTheme } = useShellStore();

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

  function numberField(
    label: string,
    key: keyof AppSettings,
    opts: { min?: number; max?: number } = {},
  ) {
    return (
      <div className="form-row" key={key}>
        <span style={{ width: 220, flexShrink: 0, fontSize: 12, color: "rgba(255,255,255,.7)" }}>
          {label}
        </span>
        <Input
          type="number"
          min={opts.min}
          max={opts.max}
          value={settings![key] as number}
          disabled={saving}
          style={{ width: 90 }}
          onChange={(e) => {
            const value = Number(e.currentTarget.value);
            if (Number.isFinite(value)) {
              setSettings({ ...settings, [key]: Math.floor(value) } as AppSettings);
            }
          }}
        />
      </div>
    );
  }

  return (
    <div>
      <div className="settings-section-title">Appearance</div>
      <div className="form-row">
        <span style={{ width: 220, flexShrink: 0, fontSize: 12, color: "rgba(255,255,255,.7)" }}>
          Theme
        </span>
        <Select
          value={theme}
          onChange={(e) => setTheme(e.currentTarget.value as "dark" | "light" | "system")}
          style={{ width: 120 }}
        >
          <option value="system">System</option>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </Select>
      </div>

      <div className="settings-section-title">Startup</div>
      <label className="form-row" style={{ cursor: "pointer" }}>
        <Checkbox
          checked={settings.autostart}
          disabled={saving}
          onChange={(e) => save({ ...settings, autostart: e.currentTarget.checked })}
        />
        <span style={{ fontSize: 12.5 }}>Launch syl automatically when you log in</span>
      </label>

      <div className="settings-section-title">Privacy</div>
      <label className="form-row" style={{ cursor: "pointer" }}>
        <Checkbox
          checked={settings.telemetryEnabled}
          disabled={saving}
          onChange={(e) => save({ ...settings, telemetryEnabled: e.currentTarget.checked })}
        />
        <span style={{ fontSize: 12.5 }}>Share anonymous usage telemetry</span>
      </label>

      <div className="settings-section-title">Resource limits</div>
      {numberField("Max concurrently loaded local models", "maxConcurrentLocalModels", {
        min: 1,
        max: 16,
      })}

      <div className="settings-section-title">Conversation defaults</div>
      <div className="form-row">
        <span style={{ width: 220, flexShrink: 0, fontSize: 12, color: "rgba(255,255,255,.7)" }}>
          Default flow for new conversations
        </span>
        <Input
          type="text"
          value={settings.defaultFlowName}
          disabled={saving}
          style={{ width: 160 }}
          onChange={(e) => setSettings({ ...settings, defaultFlowName: e.currentTarget.value })}
        />
      </div>
      {numberField("Max tool-call iterations per turn", "maxToolIterations", { min: 1, max: 50 })}
      {numberField("Conversation history budget (characters)", "contextBudgetChars", {
        min: 1000,
        max: 100000,
      })}

      <div className="settings-section-title">Local chat engine</div>
      {numberField("Context window (tokens)", "localEngineContextSize", { min: 256, max: 131072 })}
      {numberField("Max tokens per reply", "localEngineMaxTokens", { min: 16, max: 8192 })}

      <div className="settings-section-title">Local image generation</div>
      {numberField("Sampling steps", "imageEngineSteps", { min: 1, max: 150 })}
      {numberField("Image width (px)", "imageEngineWidth", { min: 64, max: 2048 })}
      {numberField("Image height (px)", "imageEngineHeight", { min: 64, max: 2048 })}

      <div className="form-row">
        <Button disabled={saving} onClick={() => save(settings)}>
          Save
        </Button>
      </div>

      {error && <p className="settings-error">{error}</p>}
    </div>
  );
}

export default GeneralTab;
