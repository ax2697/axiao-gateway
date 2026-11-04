import http from "node:http";
import { readFile, writeFile, mkdir, copyFile, stat } from "node:fs/promises";
import { existsSync, readFileSync } from "node:fs";
import { join, dirname, extname } from "node:path";
import { homedir } from "node:os";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

const PORT = Number(process.env.PORT || 7870);
const WB_HOME = process.env.WB_SWITCH_HOME || join(homedir(), ".wb-switch");
const STORE_DIR = join(WB_HOME, "axiao");
const STORE_FILE = join(STORE_DIR, "accounts.json");
const SWITCH_ACCOUNTS = join(WB_HOME, "accounts.json");
const LOCALAPPDATA =
  process.env.LOCALAPPDATA || join(homedir(), "AppData", "Local");
const AUTH_FILE =
  process.env.WB_AUTH_FILE ||
  join(
    LOCALAPPDATA,
    "CodeBuddyExtension",
    "Data",
    "Public",
    "auth.workbuddy.workbuddy-ai",
  );
// 签到助手 exe 的路径。不给默认值：没设环境变量时视为未安装，
// 接口会返回「签到助手不存在」，而不是拿开发机的路径去猜。
const SIGNIN_EXE = process.env.WB_SIGNIN_EXE || "";
const GATEWAY_BASE = process.env.WB_GATEWAY_BASE || "http://127.0.0.1:7863/v1";
const GATEWAY_ORIGIN = process.env.WB_GATEWAY_ORIGIN || "http://127.0.0.1:7863";
let GATEWAY_KEY = process.env.WB_GATEWAY_KEY || "";
try {
  GATEWAY_KEY =
    JSON.parse(
      readFileSync(join(__dirname, "..", "config.json"), "utf8"),
    ).api_key || GATEWAY_KEY;
} catch {
  /* config.json 不存在时留空，由面板自行提示输入 */
}

/** 与 REA SetupClient 对齐的客户端规则（REA 内置 workbuddy 规则原样复用） */
const CLIENT_RULES = [
  {
    name: "workbuddy",
    displayName: "WorkBuddy",
    configPath: [".workbuddy", "mcp.json"],
    markerPath: [".workbuddy"],
    skillPath: [".workbuddy", "skills"],
    format: "json",
    authFile:
      "AppData/Local/CodeBuddyExtension/Data/Public/auth.workbuddy.workbuddy-ai",
    apiBase: "https://copilot.tencent.com",
  },
  {
    name: "codebuddy",
    displayName: "CodeBuddy",
    configPath: [".codebuddy", "mcp.json"],
    markerPath: [".codebuddy"],
    skillPath: [".codebuddy", "skills"],
    format: "json",
  },
  {
    name: "trae",
    displayName: "Trae",
    configPath: ["AppData/Roaming/Trae/User/mcp.json"],
    markerPath: ["AppData/Roaming/Trae/User"],
    skillPath: [".trae", "skills"],
    format: "json",
  },
];
const CLIENT_NAMES = new Set(CLIENT_RULES.map((c) => c.name));

const DEFAULT_STORE = {
  schemaVersion: "1.0.0",
  exportedAt: new Date().toISOString(),
  generator: "axiao-shell",
  clientRules: CLIENT_RULES,
  accounts: [],
};

const json = (res, code, obj) => {
  const body = JSON.stringify(obj);
  res.writeHead(code, {
    "content-type": "application/json; charset=utf-8",
    "cache-control": "no-store",
  });
  res.end(body);
};

const text = (res, code, body, type = "text/plain; charset=utf-8") => {
  res.writeHead(code, { "content-type": type });
  res.end(body);
};

async function readJson(path, fallback = null) {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch {
    return fallback;
  }
}

async function ensureStore() {
  await mkdir(STORE_DIR, { recursive: true });
  if (!existsSync(STORE_FILE)) {
    await writeFile(
      STORE_FILE,
      JSON.stringify(DEFAULT_STORE, null, 2),
      "utf8",
    );
  }
}

async function loadStore() {
  await ensureStore();
  const store = await readJson(STORE_FILE, null);
  if (!store || !Array.isArray(store.accounts)) return structuredClone(DEFAULT_STORE);
  // 始终以最新规则表为准，账号保留
  return { ...store, clientRules: CLIENT_RULES };
}

async function saveStore(store) {
  await ensureStore();
  store.exportedAt = new Date().toISOString();
  store.generator = store.generator || "axiao-shell";
  await writeFile(STORE_FILE, JSON.stringify(store, null, 2), "utf8");
  return store;
}

/** 轻量校验：覆盖 Schema 的关键约束，不引入 ajv */
function validateExport(obj) {
  const errors = [];
  if (!obj || typeof obj !== "object") return ["根节点必须是对象"];
  if (obj.schemaVersion !== "1.0.0") errors.push("schemaVersion 必须为 1.0.0");
  if (!Array.isArray(obj.clientRules) || obj.clientRules.length === 0)
    errors.push("clientRules 必须为非空数组");
  else {
    const names = new Set();
    obj.clientRules.forEach((c, i) => {
      if (!c || typeof c.name !== "string")
        errors.push(`clientRules[${i}].name 缺失`);
      else names.add(c.name);
      if (c && c.format === undefined)
        errors.push(`clientRules[${i}].format 缺失`);
    });
    (obj.accounts || []).forEach((a, i) => {
      if (!a || typeof a.id !== "string")
        errors.push(`accounts[${i}].id 缺失`);
      if (!a || typeof a.client !== "string")
        errors.push(`accounts[${i}].client 缺失`);
      else if (!names.has(a.client))
        errors.push(`accounts[${i}].client 引用了不存在的规则 ${a.client}`);
      if (a && typeof a.enabled !== "boolean")
        errors.push(`accounts[${i}].enabled 必须为布尔`);
    });
  }
  if (!Array.isArray(obj.accounts)) errors.push("accounts 必须为数组");
  return errors;
}

/** 脱敏：默认不导出明文 token */
function redact(store, includeTokens) {
  const clone = structuredClone(store);
  clone.exportedAt = new Date().toISOString();
  clone.generator = "axiao-shell";
  if (!includeTokens) {
    clone.accounts = clone.accounts.map((a) => ({
      ...a,
      accessToken: null,
      refreshToken: null,
    }));
  }
  return clone;
}

/** 把账号写回 CodeBuddy 授权文件（先备份，仅合并已知键） */
async function applyAccountToAuth(account) {
  if (!account || !account.accessToken)
    throw new Error("该账号没有 accessToken，无法切换");
  let current = {};
  if (existsSync(AUTH_FILE)) {
    try {
      current = JSON.parse(await readFile(AUTH_FILE, "utf8"));
    } catch {
      throw new Error(`授权文件不是合法 JSON，已拒绝写入：${AUTH_FILE}`);
    }
    const bak = `${AUTH_FILE}.axiao-bak-${Date.now()}`;
    await copyFile(AUTH_FILE, bak);
  }
  const has = (k) => Object.prototype.hasOwnProperty.call(current, k);
  const set = (camel, snake, value) => {
    if (has(camel)) current[camel] = value;
    else if (has(snake)) current[snake] = value;
    else current[camel] = value;
  };
  set("accessToken", "access_token", account.accessToken);
  if (account.refreshToken) set("refreshToken", "refresh_token", account.refreshToken);
  if (account.expiresAt) set("expiresAt", "expires_at", account.expiresAt);
  await writeFile(AUTH_FILE, JSON.stringify(current, null, 2), "utf8");
  return { authFile: AUTH_FILE, merged: true };
}

async function readBody(req) {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  const raw = Buffer.concat(chunks).toString("utf8");
  if (!raw) return {};
  return JSON.parse(raw);
}

async function readRaw(req) {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  return Buffer.concat(chunks);
}

const STRIP_HEADERS = new Set([
  "x-frame-options",
  "content-security-policy",
  "content-security-policy-report-only",
  "cross-origin-opener-policy",
  "cross-origin-embedder-policy",
  "cross-origin-resource-policy",
  "content-encoding",
  "content-length",
  "transfer-encoding",
  "connection",
]);

/** 反向代理 7863 面板，去 iframe 限制并注入 api_key，嵌进一个界面 */
async function proxyPanel(req, res) {
  const u = new URL(req.url, `http://127.0.0.1:${PORT}`);
  const target = GATEWAY_ORIGIN + u.pathname + u.search;
  const headers = {};
  for (const [k, v] of Object.entries(req.headers)) {
    const lk = k.toLowerCase();
    if (["host", "connection", "content-length"].includes(lk)) continue;
    headers[k] = v;
  }
  const init = { method: req.method, headers, redirect: "manual" };
  if (!["GET", "HEAD"].includes(req.method)) init.body = await readRaw(req);
  const r = await fetch(target, init);
  const ct = r.headers.get("content-type") || "";
  const out = {};
  r.headers.forEach((v, k) => {
    if (!STRIP_HEADERS.has(k.toLowerCase())) out[k] = v;
  });
  const loc = r.headers.get("location");
  if (loc) out.location = loc.replace(GATEWAY_ORIGIN, "");
  let buf = Buffer.from(await r.arrayBuffer());
  if (ct.includes("text/html") && GATEWAY_KEY && buf.length > 0) {
    let html = buf.toString("utf8");
    const inject = `<script>try{localStorage.setItem('wb2api.key',${JSON.stringify(
      GATEWAY_KEY,
    )});localStorage.setItem('wb2api.theme','light')}catch(e){}</script>`;
    html = html.includes("</head>")
      ? html.replace("</head>", inject + "</head>")
      : inject + html;
    buf = Buffer.from(html, "utf8");
  }
  out["content-length"] = String(buf.length);
  res.writeHead(r.status, out);
  res.end(buf);
}

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
};

async function serveStatic(res, urlPath) {
  const rel = urlPath === "/" ? "/index.html" : urlPath;
  const file = join(__dirname, "public", rel.replace(/^\/+/, ""));
  if (!file.startsWith(join(__dirname, "public"))) return text(res, 403, "forbidden");
  try {
    const body = await readFile(file);
    res.writeHead(200, { "content-type": MIME[extname(file)] || "application/octet-stream" });
    res.end(body);
  } catch {
    text(res, 404, "not found");
  }
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${PORT}`);
  const p = url.pathname;

  try {
    if (p === "/panel") {
      res.writeHead(302, { location: "/panel/" });
      return res.end();
    }
    if (p.startsWith("/panel/")) return proxyPanel(req, res);

    if (p === "/api/status" && req.method === "GET") {
      return json(res, 200, {
        ok: true,
        wbHome: WB_HOME,
        storeFile: STORE_FILE,
        switchAccounts: SWITCH_ACCOUNTS,
        switchAccountsExists: existsSync(SWITCH_ACCOUNTS),
        authFile: AUTH_FILE,
        authFileExists: existsSync(AUTH_FILE),
        signinExe: SIGNIN_EXE,
        signinExeExists: existsSync(SIGNIN_EXE),
        gatewayBase: GATEWAY_BASE,
      });
    }

    if (p === "/api/clients" && req.method === "GET")
      return json(res, 200, { clientRules: CLIENT_RULES });

    if (p === "/api/components" && req.method === "GET") {
      const switchExe = join(__dirname, "vendor", "wb-switch", "wb-switch-rust.exe");
      const reaDir = join(
        dirname(__dirname),
        "..",
        "中心汇总",
        "03-逆向工程智能体-REA",
      );
      return json(res, 200, {
        components: [
          {
            id: "panel",
            name: "workbuddy2api-panel",
            version: "v1.13.0",
            role: "网关内核 · 账号池/任务/模型/配置/日志",
            type: "embedded",
            exists: true,
            running: true,
            detail: "已嵌入本页（7863）",
          },
          {
            id: "switch",
            name: "workbuddy-switch",
            role: "多账号切换 · 积分过期监控",
            type: "exe",
            path: switchExe,
            exists: existsSync(switchExe),
            running: false,
          },
          {
            id: "signin",
            name: "WorkBuddy签到助手",
            role: "签到 · 成长任务 · 导入导出",
            type: "exe",
            path: SIGNIN_EXE,
            exists: existsSync(SIGNIN_EXE),
            running: false,
          },
          {
            id: "rea",
            name: "REA MCP",
            role: "逆向分析 · 规则 Schema 标准",
            type: "mcp",
            path: reaDir,
            exists: existsSync(reaDir),
            running: false,
            detail: "需 Node ≥22.23.4，本机 v22.22.2，暂不可编译",
          },
        ],
      });
    }

    if (p === "/api/launch" && req.method === "POST") {
      const body = await readBody(req);
      if (body.confirm !== true)
        return json(res, 200, { ok: false, needConfirm: true });
      const map = {
        switch: join(__dirname, "vendor", "wb-switch", "wb-switch-rust.exe"),
        signin: SIGNIN_EXE,
      };
      const exe = map[body.id];
      if (!exe || !existsSync(exe))
        return json(res, 404, { ok: false, error: "组件不存在" });
      const child = spawn(exe, [], { detached: true, stdio: "ignore" });
      child.unref();
      return json(res, 200, { ok: true, launched: exe });
    }

    if (p === "/api/accounts" && req.method === "GET") {
      const store = await loadStore();
      return json(res, 200, store);
    }

    if (p === "/api/accounts" && req.method === "POST") {
      const body = await readBody(req);
      const store = await loadStore();
      if (Array.isArray(body.accounts)) {
        const byId = new Map(store.accounts.map((a) => [a.id, a]));
        for (const a of body.accounts) {
          if (!a || typeof a.id !== "string") continue;
          byId.set(a.id, { ...(byId.get(a.id) || {}), ...a });
        }
        store.accounts = [...byId.values()];
      }
      await saveStore(store);
      return json(res, 200, { ok: true, store });
    }

    if (p === "/api/delete" && req.method === "POST") {
      const body = await readBody(req);
      const store = await loadStore();
      const before = store.accounts.length;
      store.accounts = store.accounts.filter((a) => a.id !== body.id);
      await saveStore(store);
      return json(res, 200, { ok: true, removed: before - store.accounts.length });
    }

    if (p === "/api/gateway/models" && req.method === "GET") {
      try {
        const r = await fetch(`${GATEWAY_BASE}/models`, {
          signal: AbortSignal.timeout(4000),
        });
        const data = await r.json();
        return json(res, 200, { ok: true, data });
      } catch (e) {
        return json(res, 200, { ok: false, error: String(e?.message || e) });
      }
    }

    if (p === "/api/validate" && req.method === "POST") {
      const body = await readBody(req);
      const errors = validateExport(body);
      return json(res, 200, { ok: errors.length === 0, errors });
    }

    if (p === "/api/import" && req.method === "POST") {
      const body = await readBody(req);
      const errors = validateExport(body);
      if (errors.length) return json(res, 400, { ok: false, errors });
      const store = await loadStore();
      const byId = new Map(store.accounts.map((a) => [a.id, a]));
      let added = 0;
      let updated = 0;
      for (const a of body.accounts) {
        if (byId.has(a.id)) {
          byId.set(a.id, { ...byId.get(a.id), ...a });
          updated++;
        } else {
          byId.set(a.id, a);
          added++;
        }
      }
      store.accounts = [...byId.values()];
      // 规则表以本地为准，仅补充缺失的规则
      const localNames = new Set(store.clientRules.map((c) => c.name));
      for (const r of body.clientRules)
        if (!localNames.has(r.name)) store.clientRules.push(r);
      await saveStore(store);
      return json(res, 200, { ok: true, added, updated, total: store.accounts.length });
    }

    if (p === "/api/export" && req.method === "GET") {
      const store = await loadStore();
      const includeTokens = url.searchParams.get("tokens") === "1";
      return json(res, 200, redact(store, includeTokens));
    }

    if (p === "/api/switch" && req.method === "POST") {
      const body = await readBody(req);
      const store = await loadStore();
      const acc = store.accounts.find((a) => a.id === body.id);
      if (!acc) return json(res, 404, { ok: false, error: "账号不存在" });
      const out = await applyAccountToAuth(acc);
      return json(res, 200, { ok: true, ...out });
    }

    if (p === "/api/signin" && req.method === "POST") {
      if (!existsSync(SIGNIN_EXE))
        return json(res, 404, { ok: false, error: "签到助手不存在", signinExe: SIGNIN_EXE });
      const body = await readBody(req).catch(() => ({}));
      if (body.confirm !== true)
        return json(res, 200, {
          ok: false,
          needConfirm: true,
          signinExe: SIGNIN_EXE,
          message: "将启动本机签到助手 exe，需二次确认（confirm: true）",
        });
      const child = spawn(SIGNIN_EXE, [], { detached: true, stdio: "ignore" });
      child.unref();
      return json(res, 200, { ok: true, launched: SIGNIN_EXE });
    }

    if (p.startsWith("/api/")) return json(res, 404, { ok: false, error: "unknown api" });

    return serveStatic(res, p);
  } catch (err) {
    return json(res, 500, { ok: false, error: String(err?.message || err) });
  }
});

server.listen(PORT, "127.0.0.1", () => {
  console.log(`[axiao-shell] http://127.0.0.1:${PORT}`);
  console.log(`[axiao-shell] WB_HOME = ${WB_HOME}`);
  console.log(`[axiao-shell] AUTH_FILE = ${AUTH_FILE}`);
});
