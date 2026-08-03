"use strict";

const assert = require("assert");
const { By, until } = require("selenium-webdriver");
const { launchApp } = require("../helpers/app");
const { setReactValue } = require("../helpers/react-input");

describe("composer auto-grow", function () {
  let driver, stop;

  before(async function () {
    ({ driver, stop } = await launchApp());
    await driver.findElement(By.css(".onboarding-skip")).click();
    await driver.sleep(100);
    await driver.findElement(By.css('[title="New chat"]')).click();
    await driver.wait(until.elementLocated(By.css(".composer .input")), 5000);
  });

  after(async function () {
    if (stop) await stop();
  });

  it("grows the textarea's height as multi-line text is typed, and shrinks back when cleared", async function () {
    const textarea = await driver.findElement(By.css(".composer .input"));
    const initialHeight = await driver.executeScript((el) => el.getBoundingClientRect().height, textarea);

    const manyLines = Array.from({ length: 8 }, (_, i) => `line ${i}`).join("\n");
    await setReactValue(driver, textarea, manyLines);
    await driver.sleep(100);

    const grownHeight = await driver.executeScript((el) => el.getBoundingClientRect().height, textarea);
    assert.ok(grownHeight > initialHeight, `expected height to grow (was ${initialHeight}, now ${grownHeight})`);

    await setReactValue(driver, textarea, "");
    await driver.sleep(100);
    const shrunkHeight = await driver.executeScript((el) => el.getBoundingClientRect().height, textarea);
    assert.ok(shrunkHeight < grownHeight, `expected height to shrink back down (was ${grownHeight}, now ${shrunkHeight})`);
  });

  it("caps growth at the CSS max-height instead of growing unbounded", async function () {
    const textarea = await driver.findElement(By.css(".composer .input"));
    const maxHeight = await driver.executeScript(
      (el) => parseFloat(getComputedStyle(el).maxHeight),
      textarea,
    );

    const manyLines = Array.from({ length: 40 }, (_, i) => `line ${i}`).join("\n");
    await setReactValue(driver, textarea, manyLines);
    await driver.sleep(100);

    const height = await driver.executeScript((el) => el.getBoundingClientRect().height, textarea);
    assert.ok(height <= maxHeight + 1, `expected height (${height}) to be capped at max-height (${maxHeight})`);
  });
});
