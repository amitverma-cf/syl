"use strict";

const assert = require("assert");
const { By } = require("selenium-webdriver");
const { launchApp } = require("../helpers/app");

async function isPresent(driver, cssSelector) {
  return (await driver.findElements(By.css(cssSelector))).length > 0;
}

describe("first-run onboarding", function () {
  let driver, stop;

  before(async function () {
    ({ driver, stop } = await launchApp());
  });

  after(async function () {
    if (stop) await stop();
  });

  it("shows automatically on a workspace with zero conversations", async function () {
    // OnboardingOverlay unmounts entirely (returns null) when closed, rather than
    // just toggling a CSS class — so presence in the DOM at all is the real signal.
    assert.ok(await isPresent(driver, ".onboarding-overlay"), "onboarding overlay should be mounted on a fresh workspace");
  });

  it("'Download a local model' routes into Settings > Models and unmounts onboarding", async function () {
    const downloadBtn = await driver.findElement(By.css("#ob-download-model, .onboarding-btn.primary"));
    await downloadBtn.click();
    await driver.sleep(150);

    assert.ok(!(await isPresent(driver, ".onboarding-overlay")), "onboarding overlay should be gone from the DOM");

    const settings = await driver.findElement(By.css(".settings-overlay"));
    assert.ok((await settings.getAttribute("class")).includes("open"));
    const title = await driver.findElement(By.css(".settings-content-head h2")).getText();
    assert.ok(title.toLowerCase().includes("model"));
  });

  it("does not reappear after being dismissed once settings are closed", async function () {
    const closeBtn = await driver.findElement(By.css(".settings-content-head .header-icon-btn"));
    await closeBtn.click();
    await driver.sleep(150);
    assert.ok(!(await isPresent(driver, ".onboarding-overlay")));
  });
});
