"use strict";

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { By, until } = require("selenium-webdriver");
const { launchApp } = require("../helpers/app");
const { setReactValue, jsClick } = require("../helpers/react-input");

function testId(id) {
  return By.css(`[data-testid="${id}"]`);
}

describe("flow editor", function () {
  let driver, stop, workspaceDir;

  before(async function () {
    ({ driver, stop, workspaceDir } = await launchApp());
    await driver.findElement(By.css(".onboarding-skip")).click();
    await driver.sleep(100);
    await driver.findElement(By.css('[title="Open flow editor"]')).click();
    await driver.wait(until.elementLocated(testId("flow-toolbar")), 5000);
  });

  after(async function () {
    if (stop) await stop();
  });

  it("opens as a singleton dedicated tab (not a chat), seeded with a start state", async function () {
    const tabs = await driver.findElements(By.css(".chat-tab"));
    assert.strictEqual(tabs.length, 1);
    const text = await driver.findElement(By.css(".main-content")).getText();
    assert.ok(text.includes("start"));
  });

  it("re-opening the flow editor focuses the same tab instead of creating a second one", async function () {
    await driver.findElement(By.css('[title="Open flow editor"]')).click();
    await driver.sleep(150);
    const tabs = await driver.findElements(By.css(".chat-tab"));
    assert.strictEqual(tabs.length, 1);
  });

  it("adding a state creates a second node on the canvas", async function () {
    await driver.findElement(testId("flow-add-state")).click();
    await driver.sleep(150);
    const text = await driver.findElement(By.css(".main-content")).getText();
    assert.ok(/new-state-1/.test(text));
  });

  it("renaming the selected state (via the side panel) updates the canvas node", async function () {
    const nameField = await driver.findElement(testId("flow-state-name-input"));
    await setReactValue(driver, nameField, "checkpoint");
    await driver.sleep(150);
    const text = await driver.findElement(By.css(".main-content")).getText();
    assert.ok(text.includes("checkpoint"));
  });

  it("saves the flow to a real JSON file on disk via save_flow, matching the schema", async function () {
    const flowNameInput = await driver.findElement(testId("flow-name-input"));
    await setReactValue(driver, flowNameInput, "e2e-test-flow");
    await driver.sleep(100);

    await driver.findElement(testId("flow-save-btn")).click();
    await driver.sleep(300);

    const flowPath = path.join(workspaceDir, "flows", "e2e-test-flow.json");
    assert.ok(fs.existsSync(flowPath), `expected ${flowPath} to exist after saving`);

    const saved = JSON.parse(fs.readFileSync(flowPath, "utf8"));
    assert.strictEqual(saved.name, "e2e-test-flow");
    assert.strictEqual(saved.states.length, 2);
    assert.ok(saved.states.some((s) => s.name === "checkpoint"));
  });

  it("the saved flow round-trips back through get_flow without a schema-validation error", async function () {
    // Regression guard: save_flow re-serializes the validated Rust struct, so any
    // field that isn't skip_serializing_if=Option::is_none would come back out as
    // an explicit `null`, which the JSON schema then rejects on the next load.
    const result = await driver.executeAsyncScript(
      (name, callback) => {
        window.__TAURI_INTERNALS__.invoke("get_flow", { name })
          .then((flow) => callback({ ok: true, flow }))
          .catch((err) => callback({ ok: false, error: String(err) }));
      },
      "e2e-test-flow",
    );
    assert.strictEqual(result.ok, true, result.error);
    assert.strictEqual(result.flow.name, "e2e-test-flow");
  });

  it("rejects saving a schema-invalid flow at the real Rust command, not just in the UI", async function () {
    // The editor UI can't construct an invalid flow (its own affordances keep
    // initial_state/transitions in sync), so this calls the real save_flow
    // Tauri command directly with a deliberately broken definition — proving
    // the Rust-side executor::parse_flow validation is what actually guards
    // the on-disk file, not some client-side check that could be bypassed.
    const brokenJson = JSON.stringify({
      name: "e2e-broken-flow",
      initial_state: "does-not-exist",
      states: [{ name: "only-state", system_prompt: "", tool_allowlist: [], transitions: [] }],
    });

    const result = await driver.executeAsyncScript(
      (name, json, callback) => {
        window.__TAURI_INTERNALS__.invoke("save_flow", { name, json })
          .then(() => callback({ ok: true }))
          .catch((err) => callback({ ok: false, error: String(err) }));
      },
      "e2e-broken-flow",
      brokenJson,
    );

    assert.strictEqual(result.ok, false);
    assert.ok(/does-not-exist|initial_state|UnknownInitialState/i.test(result.error), result.error);

    const flowPath = path.join(workspaceDir, "flows", "e2e-broken-flow.json");
    assert.ok(!fs.existsSync(flowPath), "an invalid flow must never be written to disk");
  });

  it("loads one of the workspace's seeded default flows from disk", async function () {
    await driver.findElement(testId("flow-load-btn")).click();
    await driver.sleep(150);
    // this dropdown closes on mouseleave, and a real WebDriver click moves the
    // cursor across it first — click via the DOM directly instead.
    await jsClick(driver, await driver.findElement(By.css('[data-flow-name="default"]')));
    await driver.sleep(200);

    const text = await driver.findElement(By.css(".main-content")).getText();
    assert.ok(text.includes("chatting"));
  });

  it("deletes a saved flow from disk via the trash icon", async function () {
    await driver.findElement(testId("flow-load-btn")).click();
    await driver.sleep(150);
    await jsClick(driver, await driver.findElement(By.css('[data-flow-name="e2e-test-flow"]')));
    await driver.sleep(150);

    // first click arms the in-app confirmation, second click actually deletes
    await driver.findElement(testId("flow-delete-btn")).click();
    await driver.sleep(150);
    await driver.findElement(testId("flow-delete-btn")).click();
    await driver.sleep(300);

    const flowPath = path.join(workspaceDir, "flows", "e2e-test-flow.json");
    assert.ok(!fs.existsSync(flowPath), "flow file should be removed from disk after delete");
  });

  it("surfaces a real backend error (not a fabricated success) when generating with no model configured", async function () {
    const textarea = await driver.findElement(testId("flow-ai-textarea"));
    await setReactValue(driver, textarea, "a customer support triage flow");
    // a toast from the previous test (flow saved/deleted) may still be
    // animating out and visually overlapping the send button.
    await jsClick(driver, await driver.findElement(testId("flow-ai-send")));

    // no chat model is loaded/configured in a fresh workspace, so this must fail for real
    await driver.wait(async () => {
      const body = await driver.findElement(By.css("body")).getText();
      return body.toLowerCase().includes("no model") || body.toLowerCase().includes("not loaded");
    }, 5000);

    // and it must NOT have silently produced a fake flow draft to paper over the error
    const insertButtons = await driver.findElements(testId("flow-ai-insert"));
    assert.strictEqual(insertButtons.length, 0);
  });
});
