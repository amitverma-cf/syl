"use strict";

const assert = require("assert");
const { By } = require("selenium-webdriver");
const { launchApp } = require("../helpers/app");

describe("app shell", function () {
  let driver, stop;

  before(async function () {
    ({ driver, stop } = await launchApp());
    await driver.findElement(By.css(".onboarding-skip")).click();
    await driver.sleep(100);
  });

  after(async function () {
    if (stop) await stop();
  });

  it("renders the custom title bar, sidebar, and status bar", async function () {
    assert.ok(await driver.findElement(By.css(".app-topbar")).isDisplayed());
    assert.ok(await driver.findElement(By.css(".sidebar")).isDisplayed());
    assert.ok(await driver.findElement(By.css(".statusbar")).isDisplayed());

    const title = await driver.findElement(By.css(".app-title")).getText();
    assert.strictEqual(title, "syl");
  });

  it("shows Chats and Folder tabs in the sidebar, with Chats active by default", async function () {
    const tabs = await driver.findElements(By.css(".sidebar-tab"));
    assert.strictEqual(tabs.length, 2);
    const labels = await Promise.all(tabs.map((t) => t.getText()));
    assert.ok(labels[0].includes("Chats"));
    assert.ok(labels[1].includes("Folder"));

    const activeClass = await tabs[0].getAttribute("class");
    assert.ok(activeClass.includes("active"));
  });

  it("never renders any leftover browser-tab UI", async function () {
    const browserPreview = await driver.findElements(By.css(".browser-preview"));
    const browserTabItems = await driver.findElements(By.css('[data-newtab="browser"]'));
    assert.strictEqual(browserPreview.length, 0);
    assert.strictEqual(browserTabItems.length, 0);
  });

  it("has custom window control buttons instead of relying on OS decorations", async function () {
    const winButtons = await driver.findElements(By.css(".win-btn"));
    assert.strictEqual(winButtons.length, 3);
  });

  it("collapses and re-expands the sidebar", async function () {
    const toggle = await driver.findElement(By.css('[title="Toggle sidebar"]'));
    await toggle.click();
    await driver.sleep(150);
    let sidebar = await driver.findElement(By.css(".sidebar"));
    let classes = await sidebar.getAttribute("class");
    assert.ok(classes.includes("collapsed"));

    await toggle.click();
    await driver.sleep(150);
    sidebar = await driver.findElement(By.css(".sidebar"));
    classes = await sidebar.getAttribute("class");
    assert.ok(!classes.includes("collapsed"));
  });

  it("the app menu's real Quit command actually terminates the app process (must run last: kills the app)", async function () {
    await driver.findElement(By.css('[title="Menu"]')).click();
    const quitItem = await driver.findElement(By.css('[data-testid="app-menu-quit"]'));
    assert.strictEqual((await quitItem.getText()).trim(), "Quit");
    await quitItem.click();

    // the window-close button only hides the window (it stays running for
    // the tray), so seeing the WebDriver session actually go away here is
    // what proves Quit calls the real app.exit(0) path instead.
    await driver.wait(async () => {
      try {
        await driver.findElement(By.css(".app-topbar"));
        return false;
      } catch {
        return true;
      }
    }, 5000);
  });
});
