import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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

  async function handleAdd(e: React.FormEvent) {
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
    <div className="flex flex-col gap-2">
      <h2 className="text-sm font-medium">Scheduled jobs</h2>
      <p className="text-xs text-neutral-500">
        Each job fires an unattended chat turn on a cron schedule — the exact same tool-calling
        loop and flow as a live message, into its own conversation.
      </p>
      <form onSubmit={handleAdd} className="flex flex-col gap-2">
        <div className="flex gap-2">
          <input
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
            placeholder="Name (e.g. Morning summary)"
            className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
          />
          <input
            value={cronExpr}
            onChange={(e) => setCronExpr(e.currentTarget.value)}
            placeholder="Cron expression (6-field)"
            className="w-48 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm font-mono"
          />
          <select
            value={model}
            onChange={(e) => setModel(e.currentTarget.value)}
            className="rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
          >
            <option value="">Local (default)</option>
            {cloudModels.map((m) => (
              <option key={m.id} value={m.id}>
                {m.label} ({m.provider})
              </option>
            ))}
          </select>
        </div>
        <textarea
          value={prompt}
          onChange={(e) => setPrompt(e.currentTarget.value)}
          placeholder="Prompt to send when this job fires"
          rows={2}
          className="rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
        />
        <button
          type="submit"
          disabled={adding}
          className="self-start rounded bg-neutral-100 px-3 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
        >
          {adding ? "Scheduling..." : "Schedule"}
        </button>
      </form>

      {jobs.length === 0 && <p className="text-sm text-neutral-500">No scheduled jobs yet.</p>}
      {jobs.map((job) => (
        <div
          key={job.id}
          className="flex items-center justify-between rounded border border-neutral-800 px-3 py-2 text-sm"
        >
          <div>
            <span className="font-mono">{job.name}</span>
            <span className="ml-2 text-xs text-neutral-500">
              {job.cronExpr} · {job.model ?? "local"}
            </span>
            <p className="text-xs text-neutral-500">{job.prompt}</p>
          </div>
          <button
            onClick={() => handleRemove(job.id)}
            className="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400"
          >
            Remove
          </button>
        </div>
      ))}
      {error && <p className="text-sm text-red-400">{error}</p>}
    </div>
  );
}

export default ScheduledJobsTab;
