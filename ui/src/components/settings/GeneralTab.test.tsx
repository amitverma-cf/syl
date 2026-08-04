import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }));

import GeneralTab from "./GeneralTab";

const defaultSettings = {
  autostart: false,
  telemetryEnabled: false,
  maxConcurrentLocalModels: 3,
};

function mockInvoke(settings = defaultSettings) {
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === "get_settings") return Promise.resolve(settings);
    if (cmd === "update_settings") return Promise.resolve();
    return Promise.reject(new Error(`unexpected command ${cmd}`));
  });
}

describe("GeneralTab", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    mockInvoke();
  });

  it("loads and displays the persisted settings", async () => {
    render(<GeneralTab />);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("get_settings"));
    const autostartCheckbox = await screen.findByLabelText(
      "Launch syl automatically when you log in",
    );
    expect(autostartCheckbox).not.toBeChecked();
  });

  it("toggling autostart saves the updated settings immediately", async () => {
    render(<GeneralTab />);
    const autostartCheckbox = await screen.findByLabelText(
      "Launch syl automatically when you log in",
    );

    fireEvent.click(autostartCheckbox);

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("update_settings", {
        settings: { ...defaultSettings, autostart: true },
      }),
    );
  });

  it("toggling telemetry saves the updated settings immediately", async () => {
    render(<GeneralTab />);
    const telemetryCheckbox = await screen.findByLabelText("Share anonymous usage telemetry");

    fireEvent.click(telemetryCheckbox);

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("update_settings", {
        settings: { ...defaultSettings, telemetryEnabled: true },
      }),
    );
  });

  it("saving the concurrency limit sends the whole updated settings object", async () => {
    render(<GeneralTab />);
    const input = await screen.findByDisplayValue("3");

    fireEvent.change(input, { target: { value: "5" } });
    fireEvent.click(screen.getByText("Save"));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("update_settings", {
        settings: { ...defaultSettings, maxConcurrentLocalModels: 5 },
      }),
    );
  });
});
