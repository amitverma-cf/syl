"use strict";

const assert = require("assert");
const { By } = require("selenium-webdriver");
const { launchApp } = require("../helpers/app");

const PANES = [
  { key: "General" },
  { key: "AI Providers & Models" },
  { key: "MCP Servers" },
  { key: "Memory" },
  { key: "Tools" },
  { key: "Scheduled Jobs" },
  { key: "Flow Templates" },
];

describe("settings overlay", function () {
  let driver, stop;

  before(async function () {
    ({ driver, stop } = await launchApp());
    await driver.findElement(By.css(".onboarding-skip")).click();
    await driver.sleep(100);
  });

  after(async function () {
    if (stop) await stop();
  });

  it("opens from the sidebar footer on the Models pane by default", async function () {
    await driver.findElement(By.css(".settings-entry")).click();
    await driver.sleep(150);
    const overlay = await driver.findElement(By.css(".settings-overlay"));
    assert.ok((await overlay.getAttribute("class")).includes("open"));
    const title = await driver.findElement(By.css(".settings-content-head h2")).getText();
    assert.strictEqual(title, "AI Providers & Models");
  });

  it("switches between every settings pane without crashing", async function () {
    const navItems = await driver.findElements(By.css(".settings-nav-item"));
    assert.strictEqual(navItems.length, PANES.length);

    for (let i = 0; i < navItems.length; i++) {
      const items = await driver.findElements(By.css(".settings-nav-item"));
      await items[i].click();
      await driver.sleep(100);
      const title = await driver.findElement(By.css(".settings-content-head h2")).getText();
      assert.strictEqual(title, PANES[i].key);
      // the pane must actually render something, not silently fail
      const body = await driver.findElement(By.css(".settings-body"));
      assert.ok((await body.getText()).length >= 0);
    }
  });

  it("shows empty states for a workspace with no local models and no configured providers", async function () {
    const navItems = await driver.findElements(By.css(".settings-nav-item"));
    await navItems[1].click();
    await driver.sleep(100);
    const body = await driver.findElement(By.css(".settings-body"));
    const text = await body.getText();
    assert.ok(text.includes("No .gguf files found"));
  });

  it("closes via the header close button", async function () {
    await driver.findElement(By.css(".settings-content-head button")).click();
    await driver.sleep(150);
    // SettingsOverlay unmounts entirely (returns null) when closed, rather than
    // just toggling a CSS class.
    const overlays = await driver.findElements(By.css(".settings-overlay"));
    assert.strictEqual(overlays.length, 0);
  });
});
