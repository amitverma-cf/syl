"use strict";

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { By } = require("selenium-webdriver");
const { launchApp } = require("../helpers/app");

function invokeFs(driver, command, args) {
  return driver.executeAsyncScript(
    (cmd, a, callback) => {
      window.__TAURI_INTERNALS__.invoke(cmd, a)
        .then(() => callback({ ok: true }))
        .catch((err) => callback({ ok: false, error: String(err) }));
    },
    command,
    args,
  );
}

describe("folder tree", function () {
  let driver, stop, workspaceDir;

  before(async function () {
    ({ driver, stop, workspaceDir } = await launchApp());
    await driver.findElement(By.css(".onboarding-skip")).click();
    await driver.sleep(100);
    // second sidebar tab is "Folder"
    const tabs = await driver.findElements(By.css(".sidebar-tab"));
    await tabs[1].click();
    await driver.sleep(100);
  });

  after(async function () {
    if (stop) await stop();
  });

  it("shows an empty state with an 'Open a folder' action (the native picker itself can't be driven by WebDriver)", async function () {
    const empty = await driver.findElement(By.css(".folder-list .empty-state"));
    assert.ok((await empty.getText()).includes("No folder opened"));
    assert.ok(await driver.findElement(By.css(".folder-list .es-action")).isDisplayed());
  });

  // "Open folder" itself opens a native OS file-picker dialog, which is a
  // separate window outside the webview WebDriver automates — there is no
  // way to click through it from here. So the real fs plugin commands the
  // rename/delete/new-file/new-folder buttons call, and the real scope-grant
  // that "Open folder" performs, are instead exercised directly — proving
  // the actual capability and its restriction work, rather than skipping it.

  it("denies fs plugin access to a directory that was never granted scope", async function () {
    const ungrantedDir = path.join(os.tmpdir(), `syl-e2e-ungranted-${Date.now()}`);
    fs.mkdirSync(ungrantedDir, { recursive: true });
    try {
      const result = await invokeFs(driver, "plugin:fs|read_dir", { path: ungrantedDir });
      assert.strictEqual(result.ok, false, "expected read_dir to be denied without a granted scope");
      assert.ok(/forbidden|not allowed|scope/i.test(result.error), result.error);
    } finally {
      fs.rmSync(ungrantedDir, { recursive: true, force: true });
    }
  });

  it("grant_folder_access opens up exactly the granted directory for mkdir/rename/remove", async function () {
    const grantResult = await invokeFs(driver, "grant_folder_access", { path: workspaceDir });
    assert.strictEqual(grantResult.ok, true, grantResult.error);

    const testDir = path.join(workspaceDir, "e2e-folder-test");
    const mkdirResult = await invokeFs(driver, "plugin:fs|mkdir", { path: testDir });
    assert.strictEqual(mkdirResult.ok, true, mkdirResult.error);
    assert.ok(fs.existsSync(testDir) && fs.statSync(testDir).isDirectory());

    const renamedDir = path.join(workspaceDir, "e2e-folder-renamed");
    const renameResult = await invokeFs(driver, "plugin:fs|rename", { oldPath: testDir, newPath: renamedDir });
    assert.strictEqual(renameResult.ok, true, renameResult.error);
    assert.ok(!fs.existsSync(testDir), "old path should be gone after rename");
    assert.ok(fs.existsSync(renamedDir), "new path should exist after rename");

    const removeResult = await invokeFs(driver, "plugin:fs|remove", {
      path: renamedDir,
      options: { recursive: true },
    });
    assert.strictEqual(removeResult.ok, true, removeResult.error);
    assert.ok(!fs.existsSync(renamedDir), "directory should be gone after remove");
  });

  it("granting one directory does not open up an unrelated one", async function () {
    // workspaceDir was granted in the previous test; a sibling temp dir that
    // was never granted (and isn't a subpath of anything granted) must still
    // be denied — proving the grant is scoped to what was actually chosen.
    const stillUngrantedDir = path.join(os.tmpdir(), `syl-e2e-still-ungranted-${Date.now()}`);
    fs.mkdirSync(stillUngrantedDir, { recursive: true });
    try {
      const result = await invokeFs(driver, "plugin:fs|read_dir", { path: stillUngrantedDir });
      assert.strictEqual(result.ok, false, "granting workspaceDir must not leak access to unrelated paths");
    } finally {
      fs.rmSync(stillUngrantedDir, { recursive: true, force: true });
    }
  });
});
