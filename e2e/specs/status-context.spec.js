"use strict";

const assert = require("assert");
const { By, until } = require("selenium-webdriver");
const { launchApp } = require("../helpers/app");

describe("status bar context usage", function () {
  let driver, stop;

  before(async function () {
    ({ driver, stop } = await launchApp());
    await driver.findElement(By.css(".onboarding-skip")).click();
    await driver.sleep(100);
  });

  after(async function () {
    if (stop) await stop();
  });

  it("shows 'No active chat' with nothing open, and an honest empty dropdown", async function () {
    const item = await driver.findElement(By.css('[data-testid="statusbar-context"]'));
    assert.ok((await item.getText()).includes("No active chat"));

    await item.click();
    const dropdown = await driver.wait(
      until.elementLocated(By.css('[data-testid="statusbar-context-dropdown"]')),
      3000,
    );
    assert.ok((await dropdown.getText()).includes("No token usage tracked yet"));
  });

  it("tracks real (if trivially zero) token usage for a new conversation, defaulting to the catalog's first cloud model", async function () {
    // Token counting doesn't require a configured API key — just knowing
    // which model's tokenizer to use — so ChatPanel defaults to the first
    // entry in the cloud catalog even with zero providers configured, and
    // genuinely tokenizes the (currently empty) message list against it.
    await driver.findElement(By.css('[title="New chat"]')).click();
    await driver.wait(until.elementLocated(By.css(".sidebar-list .conv")), 5000);
    await driver.wait(async () => {
      const item = await driver.findElement(By.css('[data-testid="statusbar-context"]'));
      return (await item.getText()).includes("% context");
    }, 5000);

    const item = await driver.findElement(By.css('[data-testid="statusbar-context"]'));
    assert.ok((await item.getText()).includes("0% context"), await item.getText());

    await item.click();
    const dropdown = await driver.findElement(By.css('[data-testid="statusbar-context-dropdown"]'));
    const text = await dropdown.getText();
    assert.ok(/\d[\d,]* \/ \d[\d,]*/.test(text), `expected a real "used / total" token count, got: ${text}`);
  });
});
