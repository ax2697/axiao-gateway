const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];

let STATUS = null;
let STORE = { accounts: [], clientRules: [] };

const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
const fmt = (v) => { if (!v) return "—"; const d = new Date(v); return Number.isNaN(d.getTime()) ? esc(v) : d.toLocaleString("zh-CN", { hour12: false }); };

async function api(path, opts = {}) {
  const res = await fetch(path, { headers: { "content-type": "application/json" }, ...opts });
  return { ok: res.ok, status: res.status, data: await res.json().catch(() => ({})) };
}

let toastTimer;
function toast(msg) {
  const el = $("#toast");
  el.textContent = msg;
  el.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove("show"), 2200);
}

/* ---------- 抽屉 ---------- */
$("#fab").addEventListener("click", () => $("#drawer").classList.toggle("open"));
$("#drawer-close").addEventListener("click", () => $("#drawer").classList.remove("open"));
$$("#drawer-tabs button").forEach((b) => {
  b.addEventListener("click", () => {
    $$("#drawer-tabs button").forEach((x) => x.classList.remove("active"));
    b.classList.add("active");
    $$(".d-panel").forEach((p) => p.classList.remove("active"));
    $("#d-" + b.dataset.d)?.classList.add("active");
  });
});

/* ---------- 渲染 ---------- */
function renderStatus() {
  const st = STATUS;
  if (!st) return;
  $("#signin-info").innerHTML = `
    <div><span>签到助手</span><code>${esc(st.signinExe)}</code></div>
    <div><span>存在</span><b class="${st.signinExeExists ? "ok" : "bad"}">${st.signinExeExists ? "是" : "否"}</b></div>
    <div><span>授权文件</span><code>${esc(st.authFile)}</code></div>
    <div><span>授权文件存在</span><b class="${st.authFileExists ? "ok" : "bad"}">${st.authFileExists ? "是" : "否"}</b></div>`;
  $("#settings-info").innerHTML = `
    <div><span>WB_SWITCH_HOME</span><code>${esc(st.wbHome)}</code></div>
    <div><span>统一账号库</span><code>${esc(st.storeFile)}</code></div>
    <div><span>switch 账号</span><code>${esc(st.switchAccounts)}（${st.switchAccountsExists ? "存在" : "未发现"}）</code></div>
    <div><span>网关</span><code>${esc(st.gatewayBase)}</code></div>`;
}

function renderSwitch() {
  const box = $("#switch-list");
  const accs = STORE.accounts || [];
  if (!accs.length) {
    box.innerHTML = `<div class="muted">暂无账号。先到「导入导出」导入统一 JSON。</div>`;
    return;
  }
  box.innerHTML = accs.map((a) => `
    <div class="switch-item">
      <div>
        <b>${esc(a.displayName || a.accountId || a.id)}</b>
        <div class="sub">${esc(a.client)} · 积分 ${a.credits ?? 0} · 过期 ${fmt(a.expiresAt)}</div>
      </div>
      <div style="display:flex;gap:6px;align-items:center">
        <span class="pill ${a.enabled ? "green" : "gray"}">${a.enabled ? "可用" : "禁用"}</span>
        <button class="btn primary" data-switch="${esc(a.id)}">切换</button>
      </div>
    </div>`).join("");
}

async function refresh() {
  const [st, acc] = await Promise.all([api("/api/status"), api("/api/accounts")]);
  if (st.ok) { STATUS = st.data; renderStatus(); }
  if (acc.ok) { STORE = acc.data; renderSwitch(); }
}

/* ---------- 切换 ---------- */
document.addEventListener("click", async (e) => {
  const btn = e.target.closest("[data-switch]");
  if (!btn) return;
  btn.disabled = true;
  const r = await api("/api/switch", { method: "POST", body: JSON.stringify({ id: btn.dataset.switch }) });
  btn.disabled = false;
  toast(r.data.ok ? "已写回授权文件（已备份）" : "切换失败：" + (r.data.error || r.status));
  if (r.data.ok) refresh();
});

/* ---------- 导入导出 ---------- */
$("#file-import").addEventListener("change", async (e) => {
  const f = e.target.files?.[0];
  if (f) $("#import-text").value = await f.text();
});
function ioMsg(msg, ok = true) { const el = $("#io-msg"); el.textContent = msg; el.className = "msg " + (ok ? "ok" : "bad"); }

$("#btn-validate").addEventListener("click", async () => {
  let obj; try { obj = JSON.parse($("#import-text").value); } catch (err) { return ioMsg("JSON 解析失败：" + err.message, false); }
  const r = await api("/api/validate", { method: "POST", body: JSON.stringify(obj) });
  r.data.ok ? ioMsg("校验通过 ✓") : ioMsg("校验失败：" + r.data.errors.join("；"), false);
});
$("#btn-import").addEventListener("click", async () => {
  let obj; try { obj = JSON.parse($("#import-text").value); } catch (err) { return ioMsg("JSON 解析失败：" + err.message, false); }
  const r = await api("/api/import", { method: "POST", body: JSON.stringify(obj) });
  if (r.data.ok) { ioMsg(`导入成功：新增 ${r.data.added}，更新 ${r.data.updated}，共 ${r.data.total}`); refresh(); toast("导入成功"); }
  else ioMsg("导入失败：" + (r.data.errors || [r.data.error]).join("；"), false);
});
async function doExport(download) {
  const r = await api("/api/export?tokens=" + ($("#chk-tokens").checked ? "1" : "0"));
  const txt = JSON.stringify(r.data, null, 2);
  $("#export-text").value = txt;
  if (download) {
    const blob = new Blob([txt], { type: "application/json" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = `workbuddy-accounts-${new Date().toISOString().slice(0, 10)}.json`;
    a.click(); URL.revokeObjectURL(a.href);
  }
}
$("#btn-export").addEventListener("click", () => doExport(false));
$("#btn-download").addEventListener("click", () => doExport(true));

/* ---------- 签到 ---------- */
$("#btn-signin").addEventListener("click", async () => {
  const first = await api("/api/signin", { method: "POST", body: JSON.stringify({}) });
  if (first.data.needConfirm) {
    if (!confirm("将启动本机签到助手 exe（" + first.data.signinExe + "），继续？")) return;
    const r = await api("/api/signin", { method: "POST", body: JSON.stringify({ confirm: true }) });
    const el = $("#signin-msg");
    el.textContent = r.data.ok ? "已启动签到助手" : "启动失败：" + (r.data.error || r.status);
    el.className = "msg " + (r.data.ok ? "ok" : "bad");
  }
});

/* ---------- 组件 ---------- */
async function renderComponents() {
  const r = await api("/api/components");
  const list = r.data.components || [];
  $("#comp-list").innerHTML = list
    .map((c) => `
      <div class="switch-item">
        <div>
          <b>${esc(c.name)}${c.version ? " " + esc(c.version) : ""}</b>
          <div class="sub">${esc(c.role)}</div>
          <div class="sub">${c.path ? esc(c.path) : "已内嵌"}</div>
          ${c.detail ? `<div class="sub">${esc(c.detail)}</div>` : ""}
        </div>
        <div style="display:flex;gap:6px;align-items:center">
          <span class="pill ${c.running ? "green" : c.exists ? "blue" : "gray"}">${c.running ? "运行中" : c.exists ? "已就绪" : "缺失"}</span>
          ${c.type === "embedded" ? `<button class="btn" data-open-panel="1">查看</button>` : c.exists ? `<button class="btn primary" data-launch="${esc(c.id)}">启动</button>` : ""}
        </div>
      </div>`)
    .join("");
}

/* ---------- 开源矩阵 ---------- */
const MATRIX = [
  ["workbuddy2api-panel", "内核/面板（账号池·任务·模型·配置·日志）", "used"],
  ["workbuddy-switch", "多账号切换 + 积分监控", "used"],
  ["WorkBuddy-Daily / bworkbuddy", "签到引擎 + 导入导出", "used"],
  ["REA（MCP）", "规则 Schema 标准", "used"],
  ["workbuddy2api-hub", "三协议（Chat/Responses/Anthropic）", "swap"],
  ["workbuddy2api (HanawaBanana)", "成本账本 / 硬核调度", "swap"],
  ["Free2API", "读本地客户端登录态（免 OAuth）", "swap"],
  ["Agent2API", "15+ 桌面端聚合 + 429 降级", "swap"],
  ["workbuddy-3in1", "多上游三合一（WB+Trae+ZCode）", "swap"],
  ["icebears111/workbuddy2api", "配额感知切号 + 冷却状态机", "swap"],
  ["wb2go", "零依赖单文件 + 热更新 + 内嵌控制台", "swap"],
  ["modelwharf", "全家桶 + 24 家官方 API + MCP", "swap"],
  ["wild-work / multi2api", "WB+Trae+Qoder 互通", "swap"],
  ["magpie / ai-proxy", "免费额度聚合 / Deno+CF 10 上游", "swap"],
  ["workbuddy-manager", "控制面（批量纳管/密钥分发）", "overlay"],
  ["zcode-proxy", "ZCode 上游专用 + off-peak", "ref"],
];
const MATRIX_LABEL = { used: ["已用", "green"], swap: ["可切换内核", "blue"], overlay: ["可叠加", "blue"], ref: ["仅参考", "gray"] };

function renderMatrix() {
  $("#matrix-list").innerHTML = MATRIX.map(([name, role, st]) => {
    const [label, cls] = MATRIX_LABEL[st];
    return `<div class="matrix-row">
      <div><b>${esc(name)}</b><div class="sub">${esc(role)}</div></div>
      <span class="pill ${cls}">${label}</span>
    </div>`;
  }).join("");
}

document.addEventListener("click", async (e) => {
  const open = e.target.closest("[data-open-panel]");
  if (open) { $("#drawer").classList.remove("open"); return; }
  const btn = e.target.closest("[data-launch]");
  if (!btn) return;
  if (!confirm("将启动本地组件：" + btn.dataset.launch + "，继续？")) return;
  const r = await api("/api/launch", { method: "POST", body: JSON.stringify({ id: btn.dataset.launch, confirm: true }) });
  toast(r.data.ok ? "已启动" : "启动失败：" + (r.data.error || r.status));
});

/* ---------- boot ---------- */
renderComponents();
renderMatrix();
refresh();
setInterval(refresh, 60000);
