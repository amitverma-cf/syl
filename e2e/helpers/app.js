"use strict";

const { spawn } = require("child_process");
const http = require("http");
const path = require("path");
const os = require("os");
const fs = require("fs");
const net = require("net");
const { Builder, By, until } = require("selenium-webdriver");

const REPO_ROOT = path.resolve(__dirname, "..", "..");
const DEBUG_BINARY = path.join(REPO_ROOT, "target", "debug", "syl.exe");
const UI_DIST_DIR = path.join(REPO_ROOT, "ui", "dist");
// baked into the debug binary at compile time from tauri.conf.json's build.devUrl —
// a debug build always tries to load exactly this URL, so the static server below
// has to bind this exact port, not a dynamically chosen one.
const DEV_URL_PORT = 1420;

const MIME_TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".svg": "image/svg+xml",
  ".json": "application/json",
  ".ico": "image/x-icon",
  ".png": "image/png",
  ".woff2": "font/woff2",
};

function resolveAppBinary() {
  if (!fs.existsSync(DEBUG_BINARY)) {
    throw new Error(`no debug app binary found at ${DEBUG_BINARY} — run "cargo build -p syl" first`);
  }
  return DEBUG_BINARY;
}

/**
 * A debug Tauri build (unlike a release build) honors tauri.conf.json's
 * build.devUrl and tries to load exactly http://localhost:1420 at runtime,
 * exactly like `tauri dev` does. In this sandboxed environment, WebView2
 * session *creation* for the release build fails outright with "DevToolsActivePort
 * file doesn't exist", but the debug build's session/automation attachment works
 * fine — so instead of a live Vite dev server, this serves the already-built
 * production ui/dist bundle (the exact same assets a release build would embed)
 * on that fixed port, giving a self-contained, no-HMR, production-content run
 * against a binary that's proven to actually attach WebView2 automation here.
 */
function startStaticServer(rootDir, port) {
  const server = http.createServer((req, res) => {
    let urlPath = decodeURIComponent((req.url || "/").split("?")[0]);
    if (urlPath === "/") urlPath = "/index.html";
    let filePath = path.join(rootDir, urlPath);
    if (!filePath.startsWith(rootDir)) {
      res.writeHead(403);
      res.end();
      return;
    }
    if (!fs.existsSync(filePath) || fs.statSync(filePath).isDirectory()) {
      filePath = path.join(rootDir, "index.html");
    }
    const ext = path.extname(filePath);
    fs.readFile(filePath, (err, data) => {
      if (err) {
        res.writeHead(404);
        res.end("not found");
        return;
      }
      res.writeHead(200, { "Content-Type": MIME_TYPES[ext] || "application/octet-stream" });
      res.end(data);
    });
  });
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, "127.0.0.1", () => resolve(server));
  });
}

function findFreePort() {
  return new Promise((resolve, reject) => {
    const srv = net.createServer();
    srv.unref();
    srv.on("error", reject);
    srv.listen(0, () => {
      const { port } = srv.address();
      srv.close(() => resolve(port));
    });
  });
}

async function waitForPort(port, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const ok = await new Promise((resolve) => {
      const socket = net.createConnection({ port, host: "127.0.0.1" });
      socket.once("connect", () => {
        socket.end();
        resolve(true);
      });
      socket.once("error", () => resolve(false));
    });
    if (ok) return;
    if (Date.now() > deadline) throw new Error(`nothing listening on port ${port} in time`);
    await new Promise((r) => setTimeout(r, 150));
  }
}

function withTimeout(promise, ms, message) {
  let timer;
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(message)), ms);
  });
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer));
}

/**
 * On a truly empty workspace, the real app's bootstrap step (ensure_workspace_seeded)
 * eagerly downloads every catalog engine/model over the network — several GB — before
 * the window is even created. That's intentional for a fresh dev machine, but it means
 * an E2E run would hang for a very long time (or fail entirely without network access)
 * before there's anything to automate. So tests pre-seed an *empty* registry, which is
 * exactly the state ensure_workspace_seeded() already treats as "already seeded" and
 * skips — giving a fast, fully offline, zero-local-models workspace to test against.
 */
function preSeedEmptyRegistry(workspaceDir) {
  const registryDir = path.join(workspaceDir, "registry");
  fs.mkdirSync(registryDir, { recursive: true });
  fs.writeFileSync(path.join(registryDir, "engines.json"), "[]");
  fs.writeFileSync(path.join(registryDir, "models.json"), "[]");
}

/**
 * Launches a real, freshly-built copy of the syl.exe Tauri app via tauri-driver
 * (which itself drives the OS-native WebView2 control the app renders into),
 * pointed at an isolated temp workspace dir so tests never touch real user data
 * and never depend on state left by a previous test run.
 */
async function launchApp() {
  if (!fs.existsSync(UI_DIST_DIR)) {
    throw new Error(`no built frontend found at ${UI_DIST_DIR} — run "pnpm --dir ui build" first`);
  }

  const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), "syl-e2e-"));
  preSeedEmptyRegistry(workspaceDir);

  const appBinary = resolveAppBinary();
  const staticServer = await startStaticServer(UI_DIST_DIR, DEV_URL_PORT);

  // tauri-driver's own intermediary port AND the underlying msedgedriver port it
  // spawns both need to be unique per run — otherwise a slow-to-exit previous
  // instance (or a parallel run) causes msedgedriver to fail to bind, which
  // makes tauri-driver's proxy degrade into an endless "connection closed
  // before message completed" error loop instead of a clean failure.
  const driverPort = await findFreePort();
  const nativePort = await findFreePort();

  const driverProcess = spawn(
    "tauri-driver",
    ["--port", String(driverPort), "--native-port", String(nativePort)],
    {
      env: { ...process.env, SYL_WORKSPACE_DIR: workspaceDir },
      stdio: "ignore",
    },
  );

  let driver = null;

  async function cleanup() {
    if (driver) {
      // Ask the app to actually exit (quit_app -> app.exit(0)) instead of
      // just force-killing the process. The window's own close button only
      // hides it (tray-app behavior), so without this the process would
      // otherwise only ever be terminated, never given a chance to run its
      // own exit path.
      await driver
        .executeScript(() => window.__TAURI_INTERNALS__.invoke("quit_app"))
        .catch(() => {});
      await new Promise((r) => setTimeout(r, 300));
      await driver.quit().catch(() => {});
    }
    driverProcess.kill();
    await new Promise((resolve) => staticServer.close(resolve));
    fs.rmSync(workspaceDir, { recursive: true, force: true });
  }

  try {
    driverProcess.on("error", (err) => {
      throw new Error(`failed to start tauri-driver: ${err.message}. Is it installed ("cargo install tauri-driver")?`);
    });

    await waitForPort(driverPort, 15000);

    driver = await withTimeout(
      new Builder()
        .withCapabilities({
          browserName: "wry",
          "tauri:options": { application: appBinary },
        })
        .usingServer(`http://127.0.0.1:${driverPort}`)
        .build(),
      30000,
      "timed out creating a WebDriver session (tauri-driver/msedgedriver may have failed to launch the app)",
    );

    await driver.wait(until.elementLocated(By.css(".app-topbar")), 20000);
  } catch (err) {
    await cleanup();
    throw err;
  }

  return { driver, workspaceDir, stop: cleanup };
}

module.exports = { launchApp, By, until };
