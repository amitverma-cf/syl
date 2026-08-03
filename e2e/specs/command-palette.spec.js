"use strict";

const assert = require("assert");
const { Key, By, until } = require("selenium-webdriver");
const { launchApp } = require("../helpers/app");

describe("command palette", function () {
  let driver, stop;

  before(async function () {
    ({ driver, stop } = await launchApp());
    await driver.findElement(By.css(".onboarding-skip")).click();
    await driver.sleep(100);
  });

  after(async function () {
    if (stop) await stop();
  });

  it("opens with Ctrl+K and closes with Escape", async function () {
    await driver.findElement(By.css("body")).sendKeys(Key.CONTROL, "k");
    await driver.wait(until.elementLocated(By.css(".cmdk-overlay")), 5000);

    await driver.findElement(By.css("[cmdk-input]")).sendKeys(Key.ESCAPE);
    await driver.wait(async () => (await driver.findElements(By.css(".cmdk-overlay"))).length === 0, 5000);
  });

  it("opens from the search icon and filters commands as you type", async function () {
    await driver.findElement(By.css('[title="Search (⌘K)"]')).click();
    await driver.wait(until.elementLocated(By.css("[cmdk-input]")), 5000);

    const input = await driver.findElement(By.css("[cmdk-input]"));
    await input.sendKeys("flow editor");
    await driver.sleep(150);

    const items = await driver.findElements(By.css("[cmdk-item]"));
    assert.strictEqual(items.length, 1);
    assert.ok((await items[0].getText()).toLowerCase().includes("flow editor"));
  });

  it("executing 'Open flow editor' actually opens the flow editor tab", async function () {
    const items = await driver.findElements(By.css("[cmdk-item]"));
    await items[0].click();
    await driver.sleep(200);

    assert.strictEqual((await driver.findElements(By.css(".cmdk-overlay"))).length, 0);
    const activeTab = await driver.findElement(By.css(".chat-tab.active"));
    assert.ok((await activeTab.getText()).toLowerCase().includes("flow"));
  });

  it("'New chat' command creates a real conversation", async function () {
    await driver.findElement(By.css('[title="Search (⌘K)"]')).click();
    await driver.wait(until.elementLocated(By.css("[cmdk-input]")), 5000);
    await driver.findElement(By.css("[cmdk-input]")).sendKeys("New chat");
    await driver.sleep(150);

    const items = await driver.findElements(By.css("[cmdk-item]"));
    const match = [];
    for (const item of items) {
      if ((await item.getText()).includes("New chat")) match.push(item);
    }
    assert.strictEqual(match.length, 1);
    await match[0].click();

    await driver.wait(until.elementLocated(By.css(".sidebar-list .conv")), 5000);
    const convs = await driver.findElements(By.css(".sidebar-list .conv"));
    assert.strictEqual(convs.length, 1);
  });
});
