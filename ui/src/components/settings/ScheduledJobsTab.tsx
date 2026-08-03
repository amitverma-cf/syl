import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { IconTrash } from "@tabler/icons-react";
import { Button, IconButton, Input, Select, Textarea } from "../ui";
import type { CloudModel, ScheduledJob } from "../../types";

interface ScheduledJobsTabProps {
  cloudModels: CloudModel[];
}

function ScheduledJobsTab({ cloudModels }: ScheduledJobsTabProps) {
  const [jobs, setJobs] = useState<ScheduledJob[]>([]);
  const [name, setName] = useState("");
  const [cronExpr, setCronExpr] = useState("0 0 9 * * *");
  const [prompt, setPrompt] = useState("");
  const [model, setModel] = useState("");
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function refresh() {
    invoke<ScheduledJob[]>("list_scheduled_jobs")
      .then(setJobs)
      .catch((err) => setError(String(err)));
  }

  useEffect(refresh, []);

  async function handleAdd(e: React.SyntheticEvent) {
    e.preventDefault();
    if (!name.trim() || !cronExpr.trim() || !prompt.trim()) return;
    setAdding(true);
    setError(null);
    try {
      await invoke("add_scheduled_job", {
        name: name.trim(),
        cronExpr: cronExpr.trim(),
        prompt: prompt.trim(),
        model: model || null,
      });
      setName("");
      setPrompt("");
      refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setAdding(false);
    }
  }

  async function handleRemove(id: string) {
    setError(null);
    try {
      await invoke("remove_scheduled_job", { id });
      refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div>
      <div className="settings-section-title" style={{ marginTop: 0 }}>
        Scheduled jobs
      </div>
      <p style={{ fontSize: 11.5, color: "var(--text-3)", margin: "0 0 10px", lineHeight: 1.5 }}>
        Each job fires an unattended chat turn on a cron schedule — the exact same tool-calling
        loop and flow as a live message, into its own conversation.
      </p>

      {jobs.length === 0 && <p style={{ fontSize: 12, color: "var(--text-3)", margin: "0 0 8px" }}>No scheduled jobs yet.</p>}
      {jobs.map((job) => (
        <div key={job.id} className="model-row">
          <div>
            <div className="name">{job.name}</div>
            <div className="kind">
              {job.cronExpr} · {job.model ?? "local"}
            </div>
            <div className="kind" style={{ marginTop: 2 }}>
              {job.prompt}
            </div>
          </div>
          <IconButton icon={IconTrash} iconSize={14} variant="danger" onClick={() => handleRemove(job.id)} />
        </div>
      ))}

      <div className="settings-section-title">New job</div>
      <form onSubmit={handleAdd}>
        <div className="form-row">
          <Input
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
            placeholder="Name (e.g. Morning summary)"
          />
          <Input
            value={cronExpr}
            onChange={(e) => setCronExpr(e.currentTarget.value)}
            placeholder="Cron expression (6-field)"
            style={{ flex: "0 0 160px", fontFamily: "ui-monospace, monospace" }}
          />
        </div>
        <div className="form-row">
          <Select value={model} onChange={(e) => setModel(e.currentTarget.value)}>
            <option value="">Local (default)</option>
            {cloudModels.map((m) => (
              <option key={m.id} value={m.id}>
                {m.label} ({m.provider})
              </option>
            ))}
          </Select>
        </div>
        <div className="form-row" style={{ alignItems: "flex-start" }}>
          <Textarea
            value={prompt}
            onChange={(e) => setPrompt(e.currentTarget.value)}
            placeholder="Prompt to send when this job fires"
            rows={2}
            style={{ resize: "vertical" }}
          />
        </div>
        <Button type="submit" disabled={adding} style={{ display: "inline-block" }}>
          {adding ? "Scheduling…" : "Schedule"}
        </Button>
      </form>
      {error && <p className="settings-error">{error}</p>}
    </div>
  );
}

export default ScheduledJobsTab;
