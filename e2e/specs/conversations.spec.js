"use strict";

const assert = require("assert");
const { By, until } = require("selenium-webdriver");
const { launchApp } = require("../helpers/app");

describe("conversations", function () {
  let driver, stop;

  before(async function () {
    ({ driver, stop } = await launchApp());
    // dismiss the onboarding overlay that blocks interaction on a fresh workspace
    await driver.findElement(By.css(".onboarding-skip")).click();
    await driver.sleep(100);
  });

  after(async function () {
    if (stop) await stop();
  });

  it("starts with zero conversations and the empty-state CTA visible", async function () {
    const convs = await driver.findElements(By.css(".sidebar-list .conv"));
    assert.strictEqual(convs.length, 0);
    const empty = await driver.findElement(By.css(".main-content .empty-state"));
    assert.ok((await empty.getText()).toLowerCase().includes("select a conversation"));
  });

  it("creates a real conversation via the + button and opens it as a tab", async function () {
    await driver.findElement(By.css('[title="New chat"]')).click();
    await driver.wait(until.elementLocated(By.css(".sidebar-list .conv")), 5000);

    const convs = await driver.findElements(By.css(".sidebar-list .conv"));
    assert.strictEqual(convs.length, 1);
    assert.ok((await convs[0].getText()).includes("New chat"));

    const tabs = await driver.findElements(By.css(".chat-tab"));
    assert.strictEqual(tabs.length, 1);
    assert.ok(await driver.findElement(By.css(".transcript")).isDisplayed());
  });

  it("persists the conversation across a fresh list_conversations query (re-selecting it)", async function () {
    // Switch away by opening the flow editor, then click the conversation again —
    // this forces the sidebar list (fed by list_conversations) and ChatPanel
    // (fed by list_messages) to be freshly re-queried from the real sqlite store.
    await driver.findElement(By.css('[title="Open flow editor"]')).click();
    await driver.sleep(150);
    await driver.findElement(By.css(".sidebar-list .conv")).click();
    await driver.sleep(150);
    assert.ok(await driver.findElement(By.css(".transcript")).isDisplayed());
  });

  it("shows an in-app confirmation instead of deleting immediately, and Cancel keeps the conversation", async function () {
    await driver.findElement(By.css('[data-testid="conv-delete-btn"]')).click();
    await driver.wait(until.elementLocated(By.css('[data-testid="conv-delete-confirm"]')), 3000);

    await driver.findElement(By.css('[data-testid="conv-delete-no"]')).click();
    await driver.sleep(150);

    const convs = await driver.findElements(By.css(".sidebar-list .conv"));
    assert.strictEqual(convs.length, 1, "conversation should still exist after cancelling delete");
  });

  it("deletes the conversation for real after confirming, and removes its tab", async function () {
    await driver.findElement(By.css('[data-testid="conv-delete-btn"]')).click();
    await driver.wait(until.elementLocated(By.css('[data-testid="conv-delete-yes"]')), 3000);
    await driver.findElement(By.css('[data-testid="conv-delete-yes"]')).click();

    await driver.wait(async () => {
      const convs = await driver.findElements(By.css(".sidebar-list .conv"));
      return convs.length === 0;
    }, 5000);

    // a flow-editor tab was also opened earlier in this file (to force a fresh
    // list_conversations query) and is unrelated to this conversation, so check
    // specifically that no tab for the deleted chat remains rather than
    // asserting zero tabs overall.
    const tabs = await driver.findElements(By.css(".chat-tab"));
    const labels = await Promise.all(tabs.map((t) => t.getText()));
    assert.ok(!labels.some((l) => l.includes("New chat")), `expected no leftover tab for the deleted chat, got: ${labels}`);
  });
});
