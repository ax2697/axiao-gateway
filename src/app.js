const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];

const TAURI = window.__TAURI__;
const invoke = TAURI?.core?.invoke;
const hasTauri = typeof invoke === "function";

const state = {
  bundle: { accounts: [], clientRules: [] },
  upstreams: [], ledger: [], hooks: [], components: [],
  panel: { packages: [], tasks: [], models: [], uid: null },
  panelError: null,
  sessions: [], tokens: null, creditCache: null, switchSettings: null, switchError: null, realm: "cn",
  tools: [], auto: null, groups: [], creditExpiry: {},
  quota: [], updateTargets: [], extras: null, mobile: null, access: null,
  growth: { uid: null, tasks: [], streak: {}, error: null, loading: false },
  upstreamEdit: null,
  modelCache: {},
  gwlogs: [], gwlogPath: "", gwlogSize: 0, gwlogFilter: "", gwlogLevel: "", gwlogError: null,
  catalog: null, catFilter: "all", catQ: "",
  breaker: [],
};

const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
const fmt = (v) => { if (!v) return "—"; const d = new Date(v); return Number.isNaN(d.getTime()) ? esc(v) : d.toLocaleString("zh-CN", { hour12: false }); };
const days = (v) => { if (!v) return null; const t = new Date(v).getTime() - Date.now(); return Number.isNaN(t) ? null : Math.ceil(t / 86400000); };
const k = (n) => (n == null ? "—" : n >= 1000 ? (n / 1000).toFixed(0) + "K" : String(n));

let toastTimer;
function toast(msg) { const el = $("#toast"); el.textContent = msg; el.classList.add("show"); clearTimeout(toastTimer); toastTimer = setTimeout(() => el.classList.remove("show"), 2200); }

const PAGE_META = {
  dashboard: ["仪表盘", "一眼看清：几个账号、多少积分、今天用了多少 Token。"],
  accounts: ["账号管理", "所有平台的账号都在这里：积分、状态、切换。API Key 类账号没有签到和积分，仅作为上游渠道。"],
  switch: ["账号切换", "把某个账号设成 CodeBuddy 当前使用的号（写前自动备份）。只影响客户端登录，不影响网关并发。"],
  sessions: ["关联会话", "看各客户端的历史会话，按客户端分布统计；可建会话组做归类。"],
  tokens: ["Token 统计", "本机 transcript 的真实用量：柱为每天总量、线为缓存读。注意这不是网关账本。"],
  usage: ["用量", "经本网关转发的请求逐条记录，上方按上游汇总。"],
  creditstats: ["积分统计", "每个账号用掉多少、还剩多少、什么时候到期。上游没开放按日接口，所以多为累计值。"],
  credits: ["积分构成", "整体使用率、每个账号的剩余与消耗占比。"],
  quota: ["免费额度", "上半是余额账本（能抓的实时抓），下半是 35 家免费厂商的目录——真实模型数、限速、月度预算，看中哪家一键加成上游。"],
  upstreams: ["上游管理", "请求最终发给谁。同一平台可加多条 Key，会按权重轮着用、失败自动切换。"],
  protocols: ["协议与路由", "接入口径与连通性测试。填到客户端的地址和密钥都在这里。"],
  models: ["模型与档位", "按平台看模型清单；思考档位按模型单独设置，改完即存。"],
  create: ["图像与视频", "不写代码直接出图、出视频；本地 API 同时提供对话（/v1/chat/completions）与创作接口。"],
  cost: ["成本账本", "每次经网关调用的 Token 与成本，按上游汇总。"],
  requests: ["请求日志", "逐条请求 + 最近 24 小时分布。只看经网关的调用。"],
  logs: ["运行日志", "上半是网关服务日志（客户端请求进来后发生了什么），下半是你自己的操作留痕。排障先看服务日志。"],
  config: ["网关配置", "便携版 config.json 的全部字段，每项下面都写了它管什么。"],
  tasks: ["任务中心", "成长任务队列，一键完成可自动执行的部分。"],
  growth: ["积分任务", "接受任务、领取奖励；上方环形显示整体完成度。"],
  signin: ["签到中心", "原生直连上游：签到、成长任务、旅行、抽奖、保活。"],
  schedule: ["定时任务", "设置自动签到 / 保活 / 旅行的时间点。"],
  hooks: ["钩子", "关键动作留痕，用来回溯与排查。"],
  io: ["导入导出", "换电脑、备份、给同事：一份 JSON 搬走账号与规则。"],
  update: ["自动更新", "检查本应用与内核组件的新版本；GitHub 不通可配代理。"],
  extras: ["扩展能力", "移动端、控制面、压缩、官方 API、MCP、ZCode、思考档位，逐项开关。"],
  components: ["融合组件", "各组件是「就绪 / 已内置 / 未安装」，内置的能力不需要外部 exe。"],
  guide: ["使用说明", "每个功能怎么用，一页看完；每页右上角「怎么用」可直达对应章节。"],
  coverage: ["能力覆盖", "按能力矩阵核对：已实现 / 部分 / 规划中。"],
  settings: ["设置", "支持工具检测、自动签到、CLI 轮换与外观。"],
  about: ["关于", "版本与本项目的融合来源。"],
  changelog: ["更新日志", "每次改动都记在这里，升级后看这页。"],
};

const VENDOR = [
  ["CangShui/workbuddy-gateway", "Go", "2.8M", "cooldown / sticky / 3protocol / zcode / credit"],
  ["HanawaBanana/workbuddy2api", "Go", "5.6M", "cooldown / sticky / 3protocol / 成本账本"],
  ["HitMargin/ai-proxy", "Deno/TS", "61K", "3protocol / checkin / 10 上游"],
  ["aimod-cc/Agent2API", "Node/TS", "37M", "15+ 客户端 / 优先级队列 / 429 降级"],
  ["ardeyouxipianyi/workbuddy2api-hub", "Python", "12M", "双区域 / 3protocol / 指纹隔离"],
  ["icebears111/workbuddy2api", "Go", "1.7M", "配额感知切号 / 冷却状态机 / 混挂"],
  ["ikunops/free2api", "Go", "8.8M", "读本地登录态 / 轮转 / 熔断"],
  ["jjjane233/workbuddy-3in1", "Go", "4.1M", "WB+Trae+ZCode 三合一"],
  ["practice019/multi2api", "Go", "27M", "多上游抽象 / 可扩展"],
  ["wangct233-source/wb2go", "Go", "1.2M", "零依赖 / 热更新 / 内嵌控制台"],
  ["xu20060420fan-cell/...-panel", "Go", "52M", "7+ 上游全家桶"],
  ["yanghuan/wild-work", "Go", "2.8M", "WB+Trae+Qoder / 托盘"],
  ["yetone/magpie", "Go", "230M", "免费额度聚合 / 菜单栏"],
  ["zouzhenglu/modelwharf", "Node/TS", "1.9M", "24 家官方 API / MCP"],
];

/* nav */
$$("#nav .nav-item").forEach((b) => {
  b.addEventListener("click", () => {
    $$("#nav .nav-item").forEach((x) => x.classList.remove("active"));
    b.classList.add("active");
    $$(".view").forEach((v) => v.classList.remove("active"));
    state.view = b.dataset.view;
    const target = $("#view-" + b.dataset.view);
    if (!target) {
      // 视图容器缺失时不要留白屏：提示出来，方便定位
      console.warn("[nav] 缺少视图容器 view-" + b.dataset.view);
      const stage = $(".content");
      if (stage) {
        let tip = $("#view-missing");
        if (!tip) {
          tip = document.createElement("div");
          tip.id = "view-missing";
          tip.className = "view active";
          stage.appendChild(tip);
        }
        tip.innerHTML = `<section class="card"><div class="empty">这一页的容器还没建（view-${esc(b.dataset.view)}）。已记录到控制台。</div></section>`;
        tip.classList.add("active");
      }
      return;
    }
    target.classList.add("active");
    const meta = PAGE_META[b.dataset.view] || ["", ""];
    $("#page-title").textContent = meta[0];
    $("#page-sub").textContent = meta[1];
  });
});
function goto(view) { $(`#nav .nav-item[data-view="${view}"]`)?.click(); }
document.addEventListener("click", (e) => { const g = e.target.closest("[data-goto]"); if (g) goto(g.dataset.goto); });

/* ---------- 账号 ---------- */
function renderStats() {
  const a = state.bundle.accounts || [];
  const enabled = a.filter((x) => x.enabled).length;
  const credits = a.reduce((s, x) => s + (Number(x.credits) || 0), 0);
  const used = a.reduce((s, x) => s + (Number(x.creditUsage) || 0), 0);
  const html = [
    [a.length, "账号总数", ""], [enabled, "可用", "green"],
    [a.length - enabled, "已禁用", a.length - enabled ? "red" : "orange"],
    [`${credits}/${credits + used}`, "积分余额/总额", ""], [used, "已用积分", ""],
  ].map(([v, l, c]) => `<div class="stat"><div class="v ${c}">${esc(v)}</div><div class="l">${l}</div></div>`).join("");
  const el = $("#stats");
  if (el) el.innerHTML = html;
}

function statusPill(a) {
  if (!a.enabled) return `<span class="pill gray">已禁用</span>`;
  const d = days(a.expiresAt);
  if (d !== null && d <= 0) return `<span class="pill red">已过期</span>`;
  if (d !== null && d <= 3) return `<span class="pill orange">即将过期</span>`;
  return `<span class="pill green">可用</span>`;
}


/* 账号行里的积分：剩余/已用两个数字 + 一条细占比条（比单独一张图表卡更省地方） */
function accCreditCell(a) {
  const left = Number(a.credits) || 0;
  const used = Number(a.creditUsage) || 0;
  const all = left + used;
  if (!all) return `<span class="muted">—</span>`;
  const p = (used / all) * 100;
  return `<div class="acc-credit">
    <div class="acc-credit-num"><b>${fmtNum(left)}</b><span>剩</span><span class="used">已用 ${fmtNum(used)}</span></div>
    <div class="acc-credit-bar"><i style="width:${p.toFixed(1)}%"></i></div>
  </div>`;
}

function renderAccounts() {
  const tb = $("#tb-accounts");
  const all = state.bundle.accounts || [];
  const realm = state.realm || "cn";
  const a = all.filter((x) => {
    const r = String(x.region || "cn").toLowerCase();
    return realm === "global" ? (r === "global" || r === "intl") : !(r === "global" || r === "intl");
  });
  const cnt = $("#acc-count");
  if (cnt) cnt.textContent = String(a.length);
  if (!a.length) {
    tb.innerHTML = `<tr><td colspan="7" style="padding:0"><div class="empty">暂无账号。点击上方「OAuth 扫码添加」接入账号；<br/>本机已登录的账号也请一并添加，以便随时切回。</div></td></tr>`;
    return;
  }
  tb.innerHTML = a.map((x) => {
    const d = days(x.expiresAt);
    const isKey = x.client === "apikey";
    return `<tr>
      <td><div class="acc"><b>${esc(x.displayName || x.accountId || x.id)}</b><span>${esc(x.accountId || x.id)}</span></div></td>
      <td>${isKey
        ? (() => {
            const u = (state.upstreams || []).find((y) => y.id === x.accountId);
            const hasKey = !!u?.apiKey;
            const isLocal = /127\.0\.0\.1|localhost/.test(u?.baseUrl || "");
            return `<span class="pill blue">API Key</span> <span class="pill ${hasKey || isLocal ? "green" : "orange"}">${hasKey ? "Key 已配" : isLocal ? "本地桥" : "缺 Key"}</span>`;
          })()
        : statusPill(x)}</td>
      <td>${isKey ? `<span class="muted">—</span>` : accCreditCell(x)}</td>
      <td>${isKey ? "—" : (d === null ? "—" : d + " 天")}</td><td>${esc(x.source || "manual")}</td>
      <td class="op"><div class="btn-row" style="justify-content:flex-end">
        ${isKey
          ? `<button class="btn sm primary" data-use-platform="${esc(x.accountId)}">创作</button><button class="btn sm" data-up-edit="${esc(x.accountId)}">编辑</button>`
          : `<button class="btn sm" data-switch="${esc(x.id)}">切换</button>`}
        <button class="btn sm" data-toggle="${esc(x.id)}">${x.enabled ? "禁用" : "启用"}</button>
        <button class="btn sm danger" data-del="${esc(x.id)}">删除</button>
      </div></td></tr>`;
  }).join("");
  try { renderPageCharts(); } catch {}
}

function renderSwitchList() {
  const a = state.bundle.accounts || [];
  $("#switch-list").innerHTML = a.length ? a.map((x) => `
    <div class="comp-item"><div><b>${esc(x.displayName || x.accountId || x.id)}</b>
      <div class="sub">${esc(x.client)} · 积分 ${x.credits ?? 0} · 过期 ${fmt(x.expiresAt)}</div></div>
      <div class="btn-row"><span class="pill ${x.enabled ? "green" : "gray"}">${x.enabled ? "可用" : "禁用"}</span>
        <button class="btn sm primary" data-switch="${esc(x.id)}">切换</button></div></div>`).join("") : `<div class="empty">暂无账号</div>`;
}

/* ---------- 用量 / 积分 ---------- */
function ledgerEmpty(cols) {
  return `<tr><td colspan="${cols}"><div class="empty">
    这一页只统计「经本机网关 <code>127.0.0.1:7863/v1</code> 转发」的请求。<br/>
    客户端直连官方时，请求没经过网关，所以这里必然是空的——不是没刷新。
    <div class="btn-row" style="justify-content:center;margin-top:10px">
      <button class="btn sm" data-copy-base>复制接入地址</button>
      <button class="btn sm primary" data-goto="tokens">看本机实际用量（Token 统计）</button>
    </div>
  </div></td></tr>`;
}

function renderLedger() {
  const rows = state.ledger || [];
  const spend = rows.reduce((s, r) => s + (Number(r.cost) || 0), 0);
  const sum = rows.length
    ? `共 ${rows.length} 条 · prompt ${rows.reduce((s, r) => s + r.promptTokens, 0)} · completion ${rows.reduce((s, r) => s + r.completionTokens, 0)} · 约 ${money(spend)}`
    : "共 0 条 · 统计口径：经网关转发的请求";
  $("#usage-sum").textContent = sum;
  $("#cost-sum").textContent = sum;
  $("#tb-ledger").innerHTML = rows.length ? rows.map((r) => `<tr><td>${fmt(r.ts)}</td><td>${esc(r.accountId)}</td><td>${esc(r.upstream)}</td><td>${esc(r.model)}</td><td class="num">${r.promptTokens}</td><td class="num">${r.completionTokens}</td><td class="num">${money(r.cost)}</td></tr>`).join("")
    : ledgerEmpty(7);
  renderUsageChart();
  $("#tb-cost").innerHTML = rows.length ? rows.map((r) => `<tr><td>${fmt(r.ts)}</td><td>${esc(r.upstream)}</td><td>${esc(r.model)}</td><td class="num">${r.promptTokens}</td><td class="num">${r.completionTokens}</td><td class="num">${money(r.cost)}</td></tr>`).join("")
    : ledgerEmpty(6);
  try { renderPageCharts(); } catch {}
}

document.addEventListener("click", async (e) => {
  if (!e.target.closest("[data-copy-base]")) return;
  try {
    await navigator.clipboard.writeText("http://127.0.0.1:7863/v1");
    toast("已复制 Base URL：http://127.0.0.1:7863/v1");
  } catch {
    toast("复制失败，手动填 http://127.0.0.1:7863/v1");
  }
});

/* 对话测试：真实走一次 gateway_chat，验证上游 Key，并写入用量/成本账本 */
function renderTestUpstreams() {
  const sel = $("#pt-upstream");
  if (!sel) return;
  const list = (state.upstreams || []).filter((u) => u.enabled !== false);
  const keep = sel.value;
  sel.innerHTML = list.length
    ? list.map((u) => `<option value="${esc(u.id)}">${esc(u.name)}${u.apiKey ? "" : "（未填 Key）"}</option>`).join("")
    : `<option value="">（还没有上游，先去上游管理添加）</option>`;
  if (keep && list.some((u) => u.id === keep)) sel.value = keep;
}

$("#btn-pt-send")?.addEventListener("click", async () => {
  const upstream = $("#pt-upstream").value;
  const log = $("#pt-log");
  if (!upstream) {
    log.textContent = "没有可用上游。先去「上游管理」添加一个，或从「添加账号 → 其他平台」加。";
    return;
  }
  const model = $("#pt-model").value.trim() || "gpt-4o-mini";
  const content = $("#pt-msg").value.trim() || "ping";
  const btn = $("#btn-pt-send");
  btn.disabled = true;
  log.textContent = `→ ${upstream} / ${model} 发送中…`;
  const t0 = Date.now();
  try {
    const r = await invoke("gateway_chat", {
      req: {
        upstream,
        protocol: "openai",
        body: { model, messages: [{ role: "user", content }], stream: false, max_tokens: 64 },
      },
    });
    const ms = Date.now() - t0;
    log.textContent =
      `HTTP ${r.status} · ${r.upstream} · ${ms}ms${r.cooldown ? " · 已进入冷却" : ""}\n` +
      JSON.stringify(r.data, null, 2).slice(0, 1500);
    toast(r.status >= 200 && r.status < 300 ? "上游可用" : `上游返回 ${r.status}`);
    await invoke("append_hook", { kind: "test", detail: `对话测试 ${upstream} ${r.status}` });
    loadAll();
  } catch (err) {
    log.textContent = `失败：${err}`;
    toast("测试失败：" + err);
  }
  btn.disabled = false;
});


/* ============ 通用小图表：横条 / 环形 / 迷你柱 ============ */
const CHART_COLORS = ["#10b981", "#2563eb", "#d97706", "#8b5cf6", "#0891b2", "#94a3b8"];

/** 横向条形：items = [{label, value, hint?}] */
function miniBars(host, items, opts = {}) {
  if (!host) return;
  const list = (items || []).filter((x) => Number(x.value) > 0).sort((a, b) => b.value - a.value);
  if (!list.length) {
    host.innerHTML = `<div class="empty" style="padding:18px 0">${opts.empty || "暂无数据"}</div>`;
    return;
  }
  const max = Math.max(...list.map((x) => Number(x.value)), 1);
  const unit = opts.unit || "";
  host.innerHTML = list
    .map((x, i) => {
      const w = (Number(x.value) / max) * 100;
      const color = opts.color || CHART_COLORS[i % CHART_COLORS.length];
      return `<div class="mb-row" title="${esc(x.label)}${x.hint ? "｜" + esc(x.hint) : ""}">
        <div class="mb-name">${esc(x.label)}</div>
        <div class="mb-track"><div class="mb-fill" style="--w:${w.toFixed(1)}%;background:linear-gradient(90deg,${color}cc,${color});animation-delay:${i * 60}ms"></div></div>
        <div class="mb-val">${fmtNum(Number(x.value))}${unit}</div>
      </div>`;
    })
    .join("");
}

/** 环形：segments = [{label, value, color?}] */
function miniDonut(host, segments, opts = {}) {
  if (!host) return;
  const list = (segments || []).filter((x) => Number(x.value) > 0);
  if (!list.length) {
    host.innerHTML = `<div class="empty" style="padding:18px 0">${opts.empty || "暂无数据"}</div>`;
    return;
  }
  const sum = list.reduce((s, x) => s + Number(x.value), 0) || 1;
  const R = 58, C = 2 * Math.PI * R;
  let off = 0;
  const arcs = list
    .map((x, i) => {
      const len = (Number(x.value) / sum) * C;
      const color = x.color || CHART_COLORS[i % CHART_COLORS.length];
      const seg = `<circle cx="80" cy="80" r="${R}" fill="none" stroke="${color}" stroke-width="16"
        stroke-dasharray="${len.toFixed(1)} ${(C - len).toFixed(1)}" stroke-dashoffset="${(-off).toFixed(1)}"
        transform="rotate(-90 80 80)" style="animation:tkDraw .9s ease ${i * 90}ms both"/>`;
      off += len;
      return seg;
    })
    .join("");
  const center = opts.center != null ? opts.center : fmtNum(sum);
  host.innerHTML = `<div class="md-wrap">
      <svg viewBox="0 0 160 160" class="md-svg" role="img" aria-label="${esc(opts.aria || "分布")}">
        <circle cx="80" cy="80" r="${R}" fill="none" stroke="#f1f2f4" stroke-width="16"/>
        ${arcs}
        <text x="80" y="78" text-anchor="middle" font-size="18" font-weight="650" fill="#1f2430">${esc(String(center))}</text>
        <text x="80" y="96" text-anchor="middle" font-size="10" fill="#6b7280">${esc(opts.sub || "合计")}</text>
      </svg>
      <div class="md-legend">${list
        .slice(0, 6)
        .map((x, i) => `<span><i style="background:${x.color || CHART_COLORS[i % CHART_COLORS.length]}"></i>${esc(x.label)} · ${fmtNum(Number(x.value))}</span>`)
        .join("")}</div>
    </div>`;
}

/** 迷你柱：series = [{label, value}]（按时间顺序） */
function miniColumns(host, series, opts = {}) {
  if (!host) return;
  const list = series || [];
  if (!list.length) {
    host.innerHTML = `<div class="empty" style="padding:18px 0">${opts.empty || "暂无数据"}</div>`;
    return;
  }
  const max = Math.max(...list.map((x) => Number(x.value) || 0), 1);
  const bars = list
    .map((x, i) => {
      const h = Math.max(2, ((Number(x.value) || 0) / max) * 100);
      return `<div class="mc-col" title="${esc(x.label)}：${fmtNum(Number(x.value) || 0)}${opts.unit || ""}">
        <div class="mc-bar" style="--h:${h.toFixed(1)}%;animation-delay:${i * 22}ms"></div>
        <span>${esc(x.tick || "")}</span>
      </div>`;
    })
    .join("");
  host.innerHTML = `<div class="mc-wrap">${bars}</div>`;
}

/* 数字滚动动画：从 0 数到目标值（可带后缀/小数位） */
function countUp(el, target, ms = 700, suffix = "", decimals = 0) {
  if (!el) return;
  const t0 = performance.now();
  const step = (now) => {
    const p = Math.min(1, (now - t0) / ms);
    const v = target * (1 - Math.pow(1 - p, 3));
    el.textContent = (decimals ? v.toFixed(decimals) : fmtNum(Math.round(v))) + suffix;
    if (p < 1) requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
}

function renderCredits() {
  const accs = (state.bundle.accounts || []).filter((a) => a.client !== "apikey");
  const bars = $("#credit-bars");
  if (!bars) return;

  const totalLeft = accs.reduce((s, a) => s + (Number(a.credits) || 0), 0);
  const totalUsed = accs.reduce((s, a) => s + (Number(a.creditUsage) || 0), 0);
  const totalAll = totalLeft + totalUsed;
  const rate = totalAll > 0 ? (totalUsed / totalAll) * 100 : 0;

  // 顶部三张汇总卡
  const kpis = $("#credit-kpis");
  if (kpis) {
    kpis.innerHTML = `<div class="ck"><div class="v green" id="ck-left">0</div><div class="l">剩余积分</div></div>
      <div class="ck"><div class="v orange" id="ck-used">0</div><div class="l">已用积分</div></div>
      <div class="ck"><div class="v" id="ck-rate">0%</div><div class="l">使用率 · ${accs.length} 个账号</div></div>`;
    countUp($("#ck-left"), totalLeft);
    countUp($("#ck-used"), totalUsed);
    countUp($("#ck-rate"), rate, 800, "%", 1);
  }

  if ($("#credit-sum-sub")) {
    $("#credit-sum-sub").textContent = accs.length
      ? `合计 ${fmtNum(totalAll)} · 剩余 ${fmtNum(totalLeft)}`
      : "按账号汇总";
  }

  // 环形：整体使用率
  const donut = $("#credit-donut");
  if (donut) {
    const R = 74, C = 2 * Math.PI * R;
    const usedLen = (rate / 100) * C;
    donut.innerHTML = accs.length
      ? `<svg viewBox="0 0 200 200" role="img" aria-label="整体积分使用率">
          <defs>
            <linearGradient id="cgUsed" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0%" stop-color="#f59e0b"/><stop offset="100%" stop-color="#d97706"/>
            </linearGradient>
            <linearGradient id="cgLeft" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0%" stop-color="#34d399"/><stop offset="100%" stop-color="#10b981"/>
            </linearGradient>
          </defs>
          <g class="ring" style="transform: rotate(-90deg); transform-origin: 100px 100px">
            <circle cx="100" cy="100" r="${R}" fill="none" stroke="url(#cgLeft)" stroke-width="18"/>
            <circle cx="100" cy="100" r="${R}" fill="none" stroke="url(#cgUsed)" stroke-width="18"
              stroke-linecap="round" stroke-dasharray="${usedLen.toFixed(1)} ${(C - usedLen).toFixed(1)}"
              style="animation: tkDraw 1.1s ease both"/>
          </g>
          <text class="num" x="100" y="94" text-anchor="middle" font-size="26" font-weight="650" fill="#1f2430">${rate.toFixed(1)}%</text>
          <text class="num" x="100" y="116" text-anchor="middle" font-size="11" fill="#6b7280">已用 / 总额</text>
        </svg>`
      : `<div class="empty">暂无账号</div>`;
  }

  // 每账号分段条
  if (!accs.length) {
    bars.innerHTML = `<div class="empty">还没有账号。去「账号管理」添加后这里会显示每个号的积分构成。</div>`;
    return;
  }
  const max = Math.max(...accs.map((a) => (Number(a.credits) || 0) + (Number(a.creditUsage) || 0)), 1);
  bars.innerHTML = accs
    .map((a, i) => {
      const left = Number(a.credits) || 0;
      const used = Number(a.creditUsage) || 0;
      const all = left + used || 1;
      const pUsed = (used / max) * 100;
      const pLeft = (left / max) * 100;
      const useRate = ((used / all) * 100).toFixed(0);
      const exp = a.creditExpiry ? String(a.creditExpiry).slice(0, 10) : "";
      return `<div class="cl-row" title="${esc(a.displayName || a.accountId || a.id)}｜剩余 ${left}｜已用 ${used}${exp ? "｜到期 " + exp : ""}">
        <div class="cl-name">${esc(a.displayName || a.accountId || a.id)}<span>${exp ? "到期 " + esc(exp) : "无到期"} · 用掉 ${useRate}%</span></div>
        <div class="cl-track">
          <div class="cl-used" style="--w:${pUsed.toFixed(1)}%;animation-delay:${i * 80}ms"></div>
          <div class="cl-left" style="--w:${pLeft.toFixed(1)}%;animation-delay:${i * 80 + 60}ms"></div>
        </div>
        <div class="cl-num"><b>${fmtNum(left)}</b>剩余 · 已用 ${fmtNum(used)}</div>
      </div>`;
    })
    .join("");
}

/* ---------- 任务（实时抓取） ---------- */
function taskStatus(t) {
  if (t.claimed || t.accept_status === "claimed") return ["已领取", "green"];
  const cur = Number(t.current) || 0, tar = Number(t.target) || 0;
  if (tar > 0 && cur >= tar) return ["可领取", "blue"];
  return ["进行中", "orange"];
}

function renderTasks() {
  const p = state.panel;
  const src = $("#task-source");
  if (state.panelError) {
    src.textContent = "抓取失败：" + state.panelError;
    src.className = "muted";
  } else {
    src.textContent = `实时抓取 · ${p.packages.length} 账号 · ${p.tasks.length} 条任务`;
  }
  const sel = $("#task-uid");
  sel.innerHTML = (p.packages || []).map((a) => `<option value="${esc(a.uid)}">${esc(a.nickname || a.uid)}（${a.remain ?? 0}）</option>`).join("");
  if (p.uid) sel.value = p.uid;

  const rows = p.tasks || [];
  $("#tb-tasks").innerHTML = rows.length
    ? rows.map((t) => {
        const [label, cls] = taskStatus(t);
        return `<tr>
          <td>${esc(t.title || t.task_desc || t.task_code)}</td>
          <td>${t.current ?? 0} / ${t.target ?? 0}</td>
          <td>+${t.credit ?? 0} 分 +${t.energy ?? 0} 笔</td>
          <td><span class="pill ${cls}">${label}</span></td>
        </tr>`;
      }).join("")
    : `<tr><td colspan="4" class="empty">${state.panelError ? "面板不可达，先启动便携版网关（7863）" : "该账号暂无任务"}</td></tr>`;
}

/* ---------- 积分任务（面板「积分任务」弹窗的原生版） ---------- */
function growthStatus(t) {
  const cur = Number(t?.progress?.current ?? 0);
  const tar = Number(t?.progress?.target ?? 0);
  const st = t?.accept_status || "not_accepted";
  if (st === "claimed") return ["已领取", "gray"];
  if (tar > 0 && cur >= tar) return ["可领取", "green"];
  if (st === "not_accepted") return ["未接受", "orange"];
  return ["进行中", "blue"];
}

function growthClaimable(t) {
  const cur = Number(t?.progress?.current ?? 0);
  const tar = Number(t?.progress?.target ?? 0);
  return tar > 0 && cur >= tar && (t?.accept_status || "") !== "claimed";
}

function renderGrowth() {
  const g = state.growth;
  const sel = $("#growth-uid");
  if (sel) {
    const accs = state.bundle.accounts || [];
    sel.innerHTML = accs
      .map((a) => `<option value="${esc(a.accountId || a.id)}">${esc(a.displayName || a.accountId || a.id)}（${a.credits ?? 0}）</option>`)
      .join("");
    if (g.uid) sel.value = g.uid;
  }

  const tasks = g.tasks || [];
  const pending = tasks.filter((t) => (t.accept_status || "") === "not_accepted");
  const claimable = tasks.filter(growthClaimable);
  const claimed = tasks.filter((t) => t.accept_status === "claimed");
  const st = $("#growth-stats");
  if (st) {
    st.innerHTML = [
      [tasks.length, "任务总数", ""],
      [pending.length, "待接受", pending.length ? "orange" : ""],
      [claimable.length, "可领取", claimable.length ? "green" : ""],
      [claimed.length, "已领取", ""],
    ]
      .map(([v, l, c]) => `<div class="stat"><div class="v ${c}">${esc(v)}</div><div class="l">${l}</div></div>`)
      .join("");
  }

  const tb = $("#tb-growth");
  if (!tb) return;
  if (g.error) {
    tb.innerHTML = `<tr><td colspan="5" class="empty">读取失败：${esc(g.error)}</td></tr>`;
    return;
  }
  if (!tasks.length) {
    tb.innerHTML = `<tr><td colspan="5" class="empty">${g.loading ? "读取中…" : "该账号暂无积分任务"}</td></tr>`;
    return;
  }
  tb.innerHTML = tasks
    .map((t) => {
      const [label, cls] = growthStatus(t);
      const cur = Number(t?.progress?.current ?? 0);
      const tar = Number(t?.progress?.target ?? 0);
      const reward = [`+${t.reward_credit ?? 0} 分`];
      if (t.reward_energy) reward.push(`+${t.reward_energy} 张`);
      if (t.reward_buddy) reward.push("+Buddy");
      const claimable = growthClaimable(t);
      const canDo = !claimable && t.accept_status !== "claimed" && t.jump_url;
      const action = claimable
        ? `<button class="btn sm primary" data-claim="${esc(t.task_code)}">领取</button>`
        : canDo
        ? `<button class="btn sm" data-open-url="${esc(t.jump_url)}" title="${esc(t.jump_url)}">去完成</button>`
        : `<span class="muted">—</span>`;
      return `<tr>
        <td><div class="acc"><b>${esc(t.title || t.task_code)}</b><span>${esc(t.task_desc || t.description || "")}</span></div></td>
        <td class="num">${cur} / ${tar || "—"}</td>
        <td>${esc(reward.join(" "))}</td>
        <td><span class="pill ${cls}">${label}</span></td>
        <td class="op">${action}</td>
      </tr>`;
    })
    .join("");
}

async function loadGrowth(uid) {
  const g = state.growth;
  const target = uid || g.uid || (state.bundle.accounts || [])[0]?.accountId;
  if (!target) {
    g.error = "没有可用账号，先去签到处添加账号";
    renderGrowth();
    return;
  }
  g.uid = target;
  g.loading = true;
  g.error = null;
  renderGrowth();
  try {
    const r = await invoke("growth_tasks", { uid: target });
    g.tasks = r.tasks || [];
    g.streak = r.streak || {};
  } catch (e) {
    g.error = String(e);
    g.tasks = [];
  }
  g.loading = false;
  renderGrowth();
}

/* 本地创作 API 服务 */
async function renderMediaServer() {
  const el = $("#ms-status");
  if (!el) return;
  try {
    const s = await invoke("media_server_status");
    el.textContent = s.running ? `运行中 · 端口 ${s.port}${s.authRequired ? " · 需 Key" : ""}` : "未启动";
    el.className = s.running ? "msg ok" : "muted";
    if ($("#ms-base")) $("#ms-base").textContent = s.base;
    if ($("#ms-sample")) {
      $("#ms-sample").textContent = `curl -X POST ${s.base}/images/generations ^
  -H "content-type: application/json" ^
  -d "{\\"prompt\\":\\"一只戴红头巾的奶牛，商业海报风\\",\\"size\\":\\"1024x1024\\"}"`;
    }
  } catch {
    el.textContent = "状态未知";
  }
}
$("#btn-ms-start")?.addEventListener("click", async () => {
  try {
    const r = await invoke("media_server_start", { port: null });
    toast(`创作 API 已启动：${r.base}`);
    await renderMediaServer();
  } catch (e) {
    toast("启动失败：" + e);
  }
});
$("#btn-ms-stop")?.addEventListener("click", async () => {
  try {
    await invoke("media_server_stop");
    toast("已停止");
    await renderMediaServer();
  } catch (e) {
    toast("停止失败：" + e);
  }
});
/* ---------- 上游熔断状态 ---------- */
async function loadBreaker() {
  try {
    state.breaker = await invoke("breaker_status");
  } catch {
    state.breaker = [];
  }
}

/* ---------- 响应缓存 ---------- */
async function renderCache() {
  const el = $("#cache-stat");
  const tg = $("#cache-toggle");
  if (!el && !tg) return;
  try {
    const s = await invoke("cache_stats");
    if (tg) tg.checked = !!s.enabled;
    if (el) {
      const rate = Math.round((s.hitRate || 0) * 100);
      el.textContent = s.entries
        ? `${s.entries}/${s.maxEntries} 条 · 命中 ${s.hits} / 未命中 ${s.misses} · 命中率 ${rate}%`
        : `暂无缓存 · 命中 ${s.hits} / 未命中 ${s.misses}`;
    }
  } catch {
    if (el) el.textContent = "缓存状态读取失败";
  }
}

$("#cache-toggle")?.addEventListener("change", async (e) => {
  try {
    await invoke("cache_set_enabled", { on: e.target.checked });
    toast(e.target.checked ? "响应缓存已开启" : "响应缓存已关闭（并清空）");
    renderCache();
  } catch (err) {
    toast("切换失败：" + err);
  }
});

$("#btn-cache-clear")?.addEventListener("click", async () => {
  try {
    await invoke("cache_clear");
    toast("缓存已清空");
    renderCache();
  } catch (err) {
    toast("清空失败：" + err);
  }
});

/* ---------- 版本号（唯一来源：编译时的 Cargo.toml） ---------- */
async function loadVersion() {
  try {
    const v = await invoke("app_version");
    const a = $("#app-ver");
    if (a) a.textContent = `v${v} · 本地运行`;
    const b = $("#about-ver");
    if (b) b.textContent = v;
  } catch {}
}

/* ---------- 选路策略 ---------- */
async function loadRouting() {
  const sel = $("#routing-strategy");
  const hint = $("#routing-hint");
  if (!sel) return;
  try {
    const ex = await invoke("extras_get");
    const st = ex?.routing?.strategy === "health" ? "health" : "rr";
    sel.value = st;
    if (hint) {
      hint.textContent =
        st === "health"
          ? "按最近 300 次调用的成功率排，最健康的先用"
          : "按顺序轮流用，多个 Key 平均分摊";
    }
  } catch {}
}

$("#routing-strategy")?.addEventListener("change", async (e) => {
  try {
    await invoke("extras_save", { key: "routing", value: { strategy: e.target.value } });
    toast(e.target.value === "health" ? "已切到按健康度选路" : "已切回轮转");
    loadRouting();
  } catch (err) {
    toast("保存失败：" + err);
  }
});

/* ---------- 访问 Key（把网关给别人用） ---------- */
async function loadAccess() {
  let d;
  try {
    d = await invoke("access_keys_get");
  } catch {
    return;
  }
  state.access = d;
  renderAccess();
}

function renderAccess() {
  const box = $("#access-list");
  const meta = $("#access-meta");
  const d = state.access;
  if (!box || !d) return;

  const keys = d.keys || [];
  if (meta) {
    meta.textContent = d.required
      ? `${keys.length} 个 Key · 必须带 Key 才能调`
      : "未设 Key：本机任何程序都能调";
    meta.style.color = d.required ? "var(--green-dark)" : "";
  }

  if (!keys.length) {
    box.innerHTML = "";
    return;
  }

  box.innerHTML = keys
    .map(
      (k) => `<div class="access-row ${k.enabled ? "" : "off"}" data-krow="${esc(k.id)}">
      <input class="acc-name" value="${esc(k.name)}" placeholder="给谁用的" />
      <code class="acc-key" data-access-reveal="${esc(k.id)}" title="点一下显示完整 Key 并复制">…${esc(k.keyTail || "")}</code>
      <div style="display:flex;align-items:center;gap:4px">
        <input class="acc-limit-in" type="number" min="0" value="${k.dailyLimit ?? ""}" placeholder="不限" />
        <span class="muted" style="font-size:11px">/天</span>
      </div>
      <div style="display:flex;align-items:center;gap:4px">
        <input class="acc-rpm-in" type="number" min="0" value="${k.rpmLimit ?? ""}" placeholder="不限" />
        <span class="muted" style="font-size:11px">/分</span>
      </div>
      <div class="acc-used" title="今日请求数">今日 ${k.usedToday}${k.dailyLimit ? " / " + k.dailyLimit : ""}</div>
      <div class="acc-ops">
        <button class="btn sm" data-access-toggle="${esc(k.id)}">${k.enabled ? "停用" : "启用"}</button>
        <button class="btn sm danger" data-access-del="${esc(k.id)}">删除</button>
      </div>
    </div>`
    )
    .join("");
}

async function saveAccessKey(id, patch) {
  const k = (state.access?.keys || []).find((x) => x.id === id);
  if (!k) return;
  try {
    state.access = await invoke("access_key_save", {
      id,
      name: patch.name !== undefined ? patch.name : k.name,
      dailyLimit: patch.dailyLimit !== undefined ? patch.dailyLimit : k.dailyLimit,
      rpmLimit: patch.rpmLimit !== undefined ? patch.rpmLimit : k.rpmLimit,
      enabled: patch.enabled !== undefined ? patch.enabled : k.enabled,
    });
    renderAccess();
  } catch (e) {
    toast("保存失败：" + e);
  }
}

$("#btn-access-add")?.addEventListener("click", async () => {
  try {
    const n = (state.access?.keys || []).length + 1;
    state.access = await invoke("access_key_save", {
      id: null,
      name: `Key ${n}`,
      dailyLimit: null,
      rpmLimit: null,
      enabled: true,
    });
    renderAccess();
    toast("已生成。把 …后 4 位对应的完整 Key 复制给对方");
  } catch (e) {
    toast("生成失败：" + e);
  }
});

// 名字 / 限次失焦保存（不用弹窗，改了就走）
document.addEventListener("change", (e) => {
  const row = e.target.closest("[data-krow]");
  if (!row) return;
  const id = row.dataset.krow;
  if (e.target.classList.contains("acc-name")) {
    saveAccessKey(id, { name: e.target.value.trim() || "未命名" });
  } else if (e.target.classList.contains("acc-limit-in")) {
    const v = e.target.value.trim();
    saveAccessKey(id, { dailyLimit: v === "" ? null : Math.max(0, parseInt(v, 10) || 0) || null });
  } else if (e.target.classList.contains("acc-rpm-in")) {
    const v = e.target.value.trim();
    saveAccessKey(id, { rpmLimit: v === "" ? null : Math.max(0, parseInt(v, 10) || 0) || null });
  }
});

document.addEventListener("click", async (e) => {
  // 点 Key 格：取完整值显示出来，顺手尝试复制一份
  const rv = e.target.closest("[data-access-reveal]");
  if (rv) {
    const id = rv.dataset.accessReveal;
    if (rv.dataset.shown === "1") {
      // 再点一次收起来
      const k = (state.access?.keys || []).find((x) => x.id === id);
      rv.textContent = "…" + (k?.keyTail || "");
      rv.dataset.shown = "";
      return;
    }
    try {
      const full = await invoke("access_key_reveal", { id });
      rv.textContent = full;
      rv.dataset.shown = "1";
      // 复制失败不打断（有些人关剪贴板权限），提示手动选中
      try {
        await navigator.clipboard.writeText(full);
        toast("完整 Key 已复制，发给对方即可");
      } catch {
        toast("已显示完整 Key，手动选中复制");
      }
    } catch (err) {
      toast("读取失败：" + err);
    }
    return;
  }

  const tg = e.target.closest("[data-access-toggle]");
  if (tg) {
    const k = (state.access?.keys || []).find((x) => x.id === tg.dataset.accessToggle);
    if (k) await saveAccessKey(k.id, { enabled: !k.enabled });
    return;
  }
  const del = e.target.closest("[data-access-del]");
  if (del) {
    if (!confirm("删除这个 Key？对方会立刻不能再调。")) return;
    try {
      state.access = await invoke("access_key_delete", { id: del.dataset.accessDel });
      renderAccess();
      toast("已删除");
    } catch (err) {
      toast("删除失败：" + err);
    }
  }
});

$("#btn-ms-logs")?.addEventListener("click", () => {
  $("#nav .nav-item[data-view=\"logs\"]")?.click();
  setTimeout(() => loadGatewayLogs(), 150);
});

/* 上游：手动解除熔断 */
document.addEventListener("click", async (e) => {
  const rst = e.target.closest("[data-breaker-reset]");
  if (!rst) return;
  const id = rst.dataset.breakerReset;
  try {
    await invoke("breaker_reset", { id });
    toast("已恢复：" + id);
    await loadBreaker();
    renderUpstreams();
  } catch (err) {
    toast("恢复失败：" + err);
  }
});




/* ---------- Token 趋势图（SVG，带入场动画与悬停提示） ---------- */
function renderTokenChart() {
  const box = $("#token-chart");
  if (!box) return;
  const t = state.tokens || {};
  const days = (t.days || []).slice(-14);
  const sub = $("#tk-chart-sub");
  if (sub) sub.textContent = days.length ? `最近 ${days.length} 天 · 柱为总 Token，线为缓存读` : "";

  if (!days.length) {
    box.innerHTML = `<div class="empty" style="padding:26px 0">还没有用量记录</div>`;
    return;
  }

  const W = 760, H = 200, PL = 52, PR = 14, PT = 12, PB = 26;
  const innerW = W - PL - PR, innerH = H - PT - PB;
  const maxV = Math.max(...days.map((d) => Number(d.total) || 0), 1);
  const nice = (v) => {
    const p = Math.pow(10, String(Math.ceil(v)).length - 1);
    return Math.ceil(v / p) * p;
  };
  const top = nice(maxV * 1.15);
  const stepX = days.length > 1 ? innerW / (days.length - 1) : 0;
  const barW = Math.min(26, days.length > 1 ? innerW / days.length - 8 : 26);

  const y = (v) => PT + innerH - (v / top) * innerH;
  const x = (i) => PL + (days.length > 1 ? i * stepX : innerW / 2);

  // 网格 + Y 轴刻度
  const grids = [0, 0.25, 0.5, 0.75, 1]
    .map((r) => {
      const v = top * r;
      const yy = y(v);
      return `<line class="grid" x1="${PL}" y1="${yy.toFixed(1)}" x2="${W - PR}" y2="${yy.toFixed(1)}"/>
        <text class="axis" x="${PL - 8}" y="${(yy + 3).toFixed(1)}" text-anchor="end">${fmtNum(v)}</text>`;
    })
    .join("");

  // 柱：总 Token
  const bars = days
    .map((d, i) => {
      const v = Number(d.total) || 0;
      const h = Math.max(1, (v / top) * innerH);
      return `<rect class="bar" x="${(x(i) - barW / 2).toFixed(1)}" y="${(PT + innerH - h).toFixed(1)}"
        width="${barW.toFixed(1)}" height="${h.toFixed(1)}" rx="4" fill="#10b981" opacity=".16"
        style="animation-delay:${i * 55}ms"/>`;
    })
    .join("");

  // 折线：缓存读
  const pts = days.map((d, i) => [x(i), y(Number(d.cached) || 0)]);
  const line = pts.map((p, i) => `${i ? "L" : "M"}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" ");
  const area = `${line} L${pts[pts.length - 1][0].toFixed(1)},${PT + innerH} L${pts[0][0].toFixed(1)},${PT + innerH} Z`;

  // 点 + 悬停热区
  const dots = days
    .map((d, i) => {
      const [cx, cy] = pts[i];
      const label = `${d.date}｜总 ${fmtNum(d.total)}｜输入 ${fmtNum(d.input)}｜输出 ${fmtNum(d.output)}｜缓存读 ${fmtNum(d.cached)}｜${d.calls} 次`;
      return `<rect class="hit" x="${(cx - stepX / 2 || cx - 14).toFixed(1)}" y="${PT}" width="${(stepX || 28).toFixed(1)}" height="${innerH}"
          data-tip="${esc(label)}"/>
        <circle class="dot" cx="${cx.toFixed(1)}" cy="${cy.toFixed(1)}" r="3.4" fill="#fff" stroke="#10b981" stroke-width="2"
          style="animation-delay:${300 + i * 55}ms"/>
        <text class="axis" x="${cx.toFixed(1)}" y="${H - 8}" text-anchor="middle">${esc(String(d.date).slice(5))}</text>`;
    })
    .join("");

  box.innerHTML = `<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="Token 趋势" style="max-height:200px">
      <defs>
        <linearGradient id="tkArea" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="#10b981" stop-opacity=".22"/>
          <stop offset="100%" stop-color="#10b981" stop-opacity="0"/>
        </linearGradient>
      </defs>
      ${grids}
      ${bars}
      <path class="area" d="${area}" fill="url(#tkArea)"/>
      <path class="line" d="${line}" fill="none" stroke="#059669" stroke-width="2" stroke-linejoin="round"
        stroke-linecap="round" style="--len:${Math.round(pts.reduce((s, p, i) => i ? s + Math.hypot(p[0] - pts[i - 1][0], p[1] - pts[i - 1][1]) : 0, 0)) + 6}"/>
      ${dots}
    </svg>`;

  // 悬停提示
  let tip = $("#tk-tip");
  if (!tip) {
    tip = document.createElement("div");
    tip.id = "tk-tip";
    tip.className = "tk-tip";
    document.body.appendChild(tip);
  }
  box.querySelectorAll(".hit").forEach((h) => {
    h.addEventListener("mouseenter", (e) => {
      tip.textContent = h.dataset.tip || "";
      tip.classList.add("on");
    });
    h.addEventListener("mousemove", (e) => {
      tip.style.left = Math.min(e.clientX + 14, window.innerWidth - 240) + "px";
      tip.style.top = e.clientY + 16 + "px";
    });
    h.addEventListener("mouseleave", () => tip.classList.remove("on"));
  });
}


/* 用量页：按上游汇总的横向条形图 */
function renderUsageChart() {
  const box = $("#usage-chart");
  if (!box) return;
  const rows = state.ledger || [];
  const sub = $("#usage-chart-sub");
  if (!rows.length) {
    box.innerHTML = `<div class="empty" style="padding:22px 0">还没有经网关的请求。去「协议与路由」点一次「对话测试」就会出现。</div>`;
    if (sub) sub.textContent = "";
    return;
  }
  const map = new Map();
  for (const r of rows) {
    const k = r.upstream || "未知";
    const cur = map.get(k) || { p: 0, c: 0, n: 0 };
    cur.p += Number(r.promptTokens) || 0;
    cur.c += Number(r.completionTokens) || 0;
    cur.n += 1;
    map.set(k, cur);
  }
  const list = [...map.entries()]
    .map(([k, v]) => ({ k, ...v, total: v.p + v.c }))
    .sort((a, b) => b.total - a.total);
  const max = Math.max(...list.map((x) => x.total), 1);
  if (sub) sub.textContent = `${list.length} 个上游 · 共 ${rows.length} 次调用`;
  box.innerHTML =
    list
      .map(
        (x, i) => `<div class="ub-row">
        <div class="ub-name" title="${esc(x.k)}">${esc(x.k)}</div>
        <div class="ub-track"><div class="ub-fill" style="--w:${((x.total / max) * 100).toFixed(1)}%;animation-delay:${i * 70}ms"></div></div>
        <div class="ub-val">${fmtNum(x.total)}</div>
      </div>`,
      )
      .join("") +
    `<div class="hint" style="margin:8px 0 4px">条形长度 = 该上游的 Prompt + Completion 占比；调用次数见右侧总计。</div>`;
}


/* ============ 各页面的图表装配 ============ */
function renderPageCharts() {
  const ledger = state.ledger || [];

  // 1) 成本账本：按上游
  if ($("#cost-chart")) {
    const map = new Map();
    ledger.forEach((r) => {
      const k = r.upstream || "未知";
      const v = Number(r.cost) || 0;
      const tokens = (Number(r.promptTokens) || 0) + (Number(r.completionTokens) || 0);
      map.set(k, (map.get(k) || 0) + (v > 0 ? v : tokens));
    });
    miniBars($("#cost-chart"), [...map.entries()].map(([label, value]) => ({ label, value })), {
      empty: "还没有经网关的调用，成本图暂时是空的",
    });
  }

  // 2) 请求日志：最近 24 个整点
  if ($("#req-chart")) {
    const buckets = [];
    const now = new Date();
    for (let i = 23; i >= 0; i--) {
      const d = new Date(now.getTime() - i * 3600 * 1000);
      buckets.push({ key: d.getHours(), label: `${String(d.getHours()).padStart(2, "0")}:00`, value: 0, tick: i % 4 === 0 ? String(d.getHours()).padStart(2, "0") : "" });
    }
    ledger.forEach((r) => {
      const t = new Date(r.ts * (String(r.ts).length <= 10 ? 1000 : 1));
      if (isNaN(t)) return;
      const diffH = Math.floor((now - t) / 3600000);
      if (diffH < 24) buckets[23 - diffH].value += 1;
    });
    miniColumns($("#req-chart"), buckets, { unit: " 次", empty: "还没有经网关的请求" });
  }

  // 3) 运行日志：类型分布
  if ($("#log-chart")) {
    const map = new Map();
    (state.hooks || []).forEach((h) => map.set(h.kind || "其它", (map.get(h.kind || "其它") || 0) + 1));
    miniDonut(
      $("#log-chart"),
      [...map.entries()].map(([label, value]) => ({ label, value })),
      { sub: "条记录", aria: "日志类型分布", empty: "还没有日志" },
    );
  }

  // 4) 积分任务：完成度
  if ($("#growth-chart")) {
    const tasks = state.growth?.tasks || [];
    const done = tasks.filter((t) => t.claimed || t.accept_status === "claimed").length;
    const ready = tasks.filter((t) => {
      const cur = Number(t.current) || 0;
      const tar = Number(t.target) || 0;
      return !(t.claimed || t.accept_status === "claimed") && tar > 0 && cur >= tar;
    }).length;
    const doing = Math.max(0, tasks.length - done - ready);
    if (!tasks.length) {
      miniDonut($("#growth-chart"), [], { empty: "先选一个账号，加载后再看完成度" });
    } else {
      miniDonut(
        $("#growth-chart"),
        [
          { label: "已领取", value: done, color: "#10b981" },
          { label: "可领取", value: ready, color: "#2563eb" },
          { label: "进行中", value: doing, color: "#d97706" },
        ],
        { center: `${Math.round((done / tasks.length) * 100)}%`, sub: `${done}/${tasks.length} 已完成`, aria: "任务完成度" },
      );
    }
  }

  // 5) 账号管理：积分已并入表格行内，不再单独图纸

  // 6) 关联会话：客户端分布
  if ($("#sess-chart")) {
    const map = new Map();
    (state.sessions || []).forEach((s) => map.set(s.client || "未知客户端", (map.get(s.client || "未知客户端") || 0) + 1));
    miniDonut(
      $("#sess-chart"),
      [...map.entries()].map(([label, value]) => ({ label, value })),
      { sub: "个会话", aria: "客户端会话分布", empty: "还没有会话记录" },
    );
  }

  // 7) 积分统计：按账号已用
  if ($("#cs-chart")) {
    const accs = (state.bundle.accounts || []).filter((a) => a.client !== "apikey");
    miniBars(
      $("#cs-chart"),
      accs.map((a) => ({ label: a.displayName || a.accountId || a.id, value: Number(a.creditUsage) || 0 })),
      { empty: "暂无消耗记录", color: "#f59e0b" },
    );
  }

  // 8) 免费额度：各平台剩余
  if ($("#quota-chart")) {
    const rows = (state.quota || []).filter((c) => c.remaining != null && Number(c.remaining) > 0);
    miniBars(
      $("#quota-chart"),
      rows.map((c) => ({ label: c.name, value: Number(c.remaining) })),
      { empty: "还没有能抓到数字的额度（多数平台未开放接口）" },
    );
  }
}

/* ---------- 仪表盘图表（纯 SVG，无第三方库） ---------- */
function renderDashboardCharts() {
  // 1) 近 7 天 token 面积图
  const box = $("#chart-tokens");
  if (box) {
    const days = ((state.tokens && state.tokens.days) || []).slice(-7);
    if (!days.length) {
      box.innerHTML = `<div class="spark-empty">还没有 token 记录。<br/>用一次 CodeBuddy / 本应用后这里会出现曲线。</div>`;
      if ($("#spark-sum")) $("#spark-sum").textContent = "—";
    } else {
      const W = 560, H = 170, P = 26;
      const max = Math.max(...days.map((d) => Number(d.total) || 0), 1);
      const step = days.length > 1 ? (W - P * 2) / (days.length - 1) : 0;
      const pts = days.map((d, i) => {
        const v = Number(d.total) || 0;
        return [P + i * step, H - P - (v / max) * (H - P * 2), v, d.date];
      });
      const line = pts.map((p, i) => `${i ? "L" : "M"}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" ");
      const area = `${line} L${pts[pts.length - 1][0].toFixed(1)},${H - P} L${P},${H - P} Z`;
      const dots = pts
        .map(
          (p) => `<circle cx="${p[0].toFixed(1)}" cy="${p[1].toFixed(1)}" r="3" fill="#10b981" stroke="#fff" stroke-width="2"/>
        <text x="${p[0].toFixed(1)}" y="${H - 8}" text-anchor="middle" font-size="10" fill="#9aa1ac">${esc(String(p[3]).slice(5))}</text>`,
        )
        .join("");
      const sum = days.reduce((s, d) => s + (Number(d.total) || 0), 0);
      if ($("#spark-sum")) $("#spark-sum").textContent = `合计 ${fmtNum(sum)}`;
      box.innerHTML = `<svg viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" role="img" aria-label="近 7 天 token 曲线">
        <defs>
          <linearGradient id="gSpark" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stop-color="#10b981" stop-opacity=".28"/>
            <stop offset="100%" stop-color="#10b981" stop-opacity="0"/>
          </linearGradient>
        </defs>
        <line x1="${P}" y1="${H - P}" x2="${W - P}" y2="${H - P}" stroke="#e4e6eb"/>
        <path d="${area}" fill="url(#gSpark)"/>
        <path d="${line}" fill="none" stroke="#10b981" stroke-width="2.2" stroke-linejoin="round" stroke-linecap="round"/>
        ${dots}
      </svg>`;
    }
  }

  // 2) 积分环形图
  const db = $("#chart-credits");
  if (db) {
    const accs = (state.bundle.accounts || []).filter((a) => a.client !== "apikey");
    const total = accs.reduce((s, a) => s + (Number(a.credits) || 0), 0);
    const used = accs.reduce((s, a) => s + (Number(a.creditUsage) || 0), 0);
    if (!accs.length) {
      db.innerHTML = `<div class="spark-empty">还没有账号。<br/>添加账号后这里显示积分构成。</div>`;
      if ($("#donut-sum")) $("#donut-sum").textContent = "—";
    } else {
      const colors = ["#10b981", "#2563eb", "#d97706", "#8b5cf6", "#94a3b8"];
      const top = accs.slice(0, 4).map((a) => ({ name: a.displayName || a.accountId || a.id, v: Number(a.credits) || 0 }));
      const restV = accs.slice(4).reduce((s, a) => s + (Number(a.credits) || 0), 0);
      if (restV > 0) top.push({ name: "其它", v: restV });
      const sum = top.reduce((s, x) => s + x.v, 0) || 1;
      const R = 62, C = 2 * Math.PI * R;
      let offset = 0;
      const arcs = top
        .map((x, i) => {
          const len = (x.v / sum) * C;
          const seg = `<circle cx="90" cy="90" r="${R}" fill="none" stroke="${colors[i % colors.length]}"
            stroke-width="16" stroke-dasharray="${len.toFixed(1)} ${(C - len).toFixed(1)}"
            stroke-dashoffset="${(-offset).toFixed(1)}" transform="rotate(-90 90 90)" stroke-linecap="butt"/>`;
          offset += len;
          return seg;
        })
        .join("");
      if ($("#donut-sum")) $("#donut-sum").textContent = `剩 ${fmtNum(total)} · 已用 ${fmtNum(used)}`;
      db.innerHTML = `<svg viewBox="0 0 180 180" style="max-height:180px" role="img" aria-label="积分构成">
        <circle cx="90" cy="90" r="${R}" fill="none" stroke="#f1f2f4" stroke-width="16"/>
        ${arcs}
        <text x="90" y="86" text-anchor="middle" font-size="20" font-weight="650" fill="#1f2430">${esc(fmtNum(total))}</text>
        <text x="90" y="106" text-anchor="middle" font-size="11" fill="#6b7280">剩余积分</text>
      </svg>
      <div class="donut-legend">${top
        .map((x, i) => `<span><i style="background:${colors[i % colors.length]}"></i>${esc(x.name)} · ${fmtNum(x.v)}</span>`)
        .join("")}</div>`;
    }
  }
}

/* ---------- 全功能教程 ---------- */
const GUIDE = [
  {
    id: "accounts", title: "账号管理", what: "看所有账号的积分、状态，决定用哪个号。",
    steps: [
      "第一次用先点右上「OAuth 扫码添加」，或在「添加账号」里选浏览器登录。",
      "国内版填 CN、国际版填 Global，登录完账号会自动出现在列表里。",
      "「切换」= 把这个号写成 CodeBuddy 的当前登录（写前自动备份）。",
    ],
    tip: "账号池为空时所有接口都会失败，这是新手第一个坑。",
  },
  {
    id: "switch", title: "账号切换", what: "把某个账号设成客户端当前使用的号。",
    steps: ["在列表里点「切换」即可。", "切换会改 CodeBuddy 的授权文件，写前存一份备份。"],
    tip: "切换只影响 CodeBuddy 系（IDE / CLI / 插件），不影响网关并发。",
  },
  {
    id: "sessions", title: "关联会话", what: "看你各客户端的历史会话，按会话组同步。",
    steps: ["打开页面自动读 hook-events.jsonl。", "想分组就在下面新建会话组。"],
    tip: "这里空是因为 hook.sh 还没被触发过——用一次 CodeBuddy 就有了。",
  },
  {
    id: "tokens", title: "Token 统计", what: "你实际用了多少 token、缓存命中多少。数据来自本机 transcript。",
    steps: ["选时间范围（今日 / 7 天 / 30 天 / 全年）。", "表格按天列出输入、输出、缓存读、总 Token、调用次数。"],
    tip: "这页和「用量」不是一回事：这里算的是你本机聊天记录，跟走不走网关无关。",
  },
  {
    id: "usage", title: "用量", what: "经网关转发的请求账本。",
    steps: ["去「协议与路由」点一次「对话测试」，这里就会出现第一条。"],
    tip: "客户端直连官方时这里永远是空的，不是坏了。",
  },
  {
    id: "creditstats", title: "积分统计", what: "每个账号累计用了多少积分、还剩多少、什么时候到期。",
    steps: ["打开即读 data/state.json。", "按天明细只有上游给了才有；没有就显示账号累计。"],
    tip: "上游没有开放按日积分接口，所以按天那栏经常是空的。",
  },
  {
    id: "quota", title: "免费额度", what: "两块：上半是本机账号的真实余额，下半是 35 家免费厂商的公开目录（410 个模型）。",
    steps: [
      "上半：单卡点「抓取」拉最新余额，「刷新全部」一次全拉。",
      "下半「免费模型目录」：每家都标了模型数、RPM/RPD 限额、月度预算，都是实数。",
      "看中哪家点「加为上游」——地址和协议自动填好，去「上游管理」补个 Key 就能走。",
      "点「看模型」能展开该家的全部模型，带视觉/工具/上下文标记。",
    ],
    tip: "「官方直营」额度小但能长期用；「聚合站」是第三方中转，额度宽松但稳定性和数据合规自己判断——别把重要 Key 挂上去。抓不到余额是正常的，那家没开放接口。",
  },
  {
    id: "upstreams", title: "上游管理", what: "配置「请求最终发给谁」——WorkBuddy、智谱、Kimi、豆包桥…",
    steps: [
      "点上面的平台小按钮快速加一家（24 家预设）。",
      "填 API Key；有「模型/接入点」字段的（如火山方舟）填 ep-xxxx。",
      "每行右侧可「编辑 / 停用 / 删除」。",
    ],
    tip: "同一平台能加多条（多把 Key），按权重轮着用、失败自动切换。协议选 Claude / Gemini 时会**真的转换请求与响应**（不是只换 URL）；OpenAI 形状的客户端始终看到同一种结构。",
  },
  {
    id: "protocols", title: "协议与路由", what: "看接入地址，并用「对话测试」验证上游通不通。",
    steps: ["复制 Base URL 和密钥，填到你的客户端里。", "选一个上游点「发送测试」，能看到状态码和返回内容。"],
    tip: "测试成功会写进「用量 / 成本账本」，这两页的数据就是这么来的。",
  },
  {
    id: "models", title: "模型与档位", what: "按平台看模型清单，并按模型设思考档位。",
    steps: ["WorkBuddy 那张卡是面板实时目录，带积分倍率。", "其它平台点各自卡片右上「刷新」拉 /models。", "思考档位在本行下拉里选，改完即存。"],
    tip: "只有该模型支持的档位才会出现在下拉里；xhigh/max 在非 Anthropic 上游会降级成 high。",
  },
  {
    id: "create", title: "图像与视频", what: "不写代码直接画图、生成视频；同时对外提供 OpenAI 兼容的创作 API。",
    steps: [
      "选上游、选模式（文生图 / 图生图 / 文生视频 / 图生视频）。",
      "填提示词；尺寸可手填；参考素材支持图 / 视频 / 音频多选；视频可设时长。",
      "私有参数写进「高级参数 JSON」，会原样并进请求体。",
    ],
    tip: "对话与创作都走同一个本地端口 7864。stream:true 是<b>真·逐字流式</b>——上游吐一个 token 就转一个，首字延迟等于上游首字时间；上游是 Anthropic / Gemini 协议会自动翻成 OpenAI 的 SSE。上游连续 90 秒没新内容才断开。窗口不开也能跑：命令行加 --serve-only。创作需要上游支持 /images 或 /videos。",
  },
  {
    id: "cost", title: "成本账本", what: "每次经网关调用的 token 与成本。",
    steps: ["和「用量」同源，按上游聚合看更省事。"],
    tip: "没有网关调用就是空的。",
  },
  {
    id: "requests", title: "请求日志", what: "逐条看经网关转发的请求。",
    steps: ["按上游筛选。", "没有记录时按提示去「对话测试」发一条。"],
    tip: "这里只记经网关的请求。",
  },
  {
    id: "logs", title: "运行日志", what: "两块：上面是网关服务日志（客户端请求进来后发生了什么），下面是你自己的操作留痕。",
    steps: [
      "客户端报错但看不出原因 → 先看上半「网关服务日志」。",
      "一条都没有＝请求根本没到网关，去查客户端填的地址 / 端口对不对。",
      "出现 401＝Key 不匹配；出现「上游 5xx」＝对方服务或网络不通。",
      "下半「操作事件」筛类型或搜关键字，查签到 / 切号这类动作。",
    ],
    tip: "停在日志页时服务日志每 8 秒自动刷新。清空只删日志文件，不动账本和统计。",
  },
  {
    id: "config", title: "网关配置", what: "改便携版网关的 config.json，46 个字段。",
    steps: ["改完点最下面「保存配置」。", "每项下面都写了它管什么、建议填多少。"],
    tip: "拿不准别动，出厂值能跑；改监听地址/密钥要重启网关。",
  },
  {
    id: "tasks", title: "任务中心", what: "面板的实时任务队列，可一键完成。",
    steps: ["选账号 → 「一键完成可自动任务」。"],
    tip: "这里只显示能自动做的；行为类任务去「积分任务」。",
  },
  {
    id: "growth", title: "积分任务", what: "成长任务列表：接受、领取、去完成。",
    steps: ["「全部接受」把未接受的接掉。", "「一键完成可领任务」把达标的奖励一次领完。", "「进行中」的点「去完成」会打开对应页面。"],
    tip: "行为类任务（体验专家、关注公众号）只能你真去做，脚本代不了。",
  },
  {
    id: "signin", title: "签到中心", what: "原生直连上游签到，不启动任何 exe。",
    steps: ["选账号 → 「立即签到」；要全签点「批量签到全部」。", "成长任务 / 旅行 / 抽奖 / 保活都在这一排按钮里。"],
    tip: "报 401 说明 token 过期，点「刷新 Token」或重新授权。",
  },
  {
    id: "schedule", title: "定时任务", what: "设置自动签到、保活、旅行的时间点。",
    steps: ["填小时（逗号分隔，如 9,21）→ 保存。"],
    tip: "这里只是本机的提醒配置；真正的定时由网关配置里的 schedule 决定。",
  },
  {
    id: "hooks", title: "钩子", what: "记录关键动作，方便回溯。",
    steps: ["点「写测试钩子」验证链路。"],
    tip: "钩子事件同时喂给「运行日志」和「关联会话」。",
  },
  {
    id: "io", title: "导入导出", what: "把账号搬到另一台电脑 / 备份 / 交给同事。",
    steps: ["① 导入：选文件或粘贴 JSON → 「导入合并」（同 ID 更新，其它追加）。", "② 导出：决定要不要勾「含明文凭证」→ 「生成」→ 「另存为文件」。"],
    tip: "勾了明文凭证等于把账号给出去，别传网盘 / 群。",
  },
  {
    id: "update", title: "自动更新", what: "检查本应用和各内核组件有没有新版。",
    steps: ["点某个组件的「检查更新」。", "GitHub 不通时在上面填代理。"],
    tip: "有的仓库没发 Release，会显示最新 tag，这也是正常的。",
  },
  {
    id: "extras", title: "扩展能力", what: "移动端、控制面、压缩、官方 API、MCP、ZCode、思考档位。",
    steps: ["每张卡下面都写了「这是干啥的、怎么用」，照着做就行。"],
    tip: "控制面只在把网关分给别人用时才需要；自己用留空。",
  },
  {
    id: "settings", title: "设置", what: "本机检测、支持工具开关、CLI 轮换。",
    steps: ["「支持工具」列出 23 个平台，绿=已安装，灰=未检测到。", "CLI 检测能把本机已登录的客户端登录态导成账号。"],
    tip: "灰的不代表不支持，只是本机没装。",
  },
  {
    id: "changelog", title: "更新日志", what: "每次改了什么，按版本记录。",
    steps: ["升级后不知道变了什么就看这一页。"],
    tip: "它随 exe 打包，不联网也能看。",
  },
];

function renderGuide(q) {
  const body = $("#guide-body");
  if (!body) return;
  const kw = String(q || "").trim().toLowerCase();
  // 搜索范围要包含「什么时候看这条」：用户遇到问题时搜的是症状
  // （白屏 / 401 / 没流式），不是功能名。
  const hay = (g) =>
    `${g.title} ${g.what} ${g.steps.join(" ")} ${g.tip || ""} ${GUIDE_WHEN[g.id] || ""}`.toLowerCase();
  const rows = kw ? GUIDE.filter((g) => hay(g).includes(kw)) : GUIDE;
  const cnt = $("#guide-count");
  if (cnt) cnt.textContent = kw ? `匹配 ${rows.length} / ${GUIDE.length} 项` : `${GUIDE.length} 个功能，逐个写了用法`;
  if (!rows.length) {
    body.innerHTML = `<section class="card"><div class="empty">没找到匹配的功能。<br/>换个说法试试，比如「白屏」「401」「没流式」「积分是 0」——这些症状词都能搜到。</div></section>`;
    return;
  }
  body.innerHTML = rows
    .map(
      (g) => `<section class="card" id="g-${esc(g.id)}">
      <div class="card-head"><h2>${esc(g.title)}</h2><span class="muted">${esc(g.what)}</span></div>
      ${GUIDE_WHEN[g.id] ? `<p class="gw"><b>什么时候看这条：</b>${esc(GUIDE_WHEN[g.id])}</p>` : ""}
      <ol class="hint" style="padding:0 24px 6px 44px">${g.steps.map((s) => `<li>${s}</li>`).join("")}</ol>
      ${g.tip ? `<p class="hint"><b>注意：</b>${esc(g.tip)}</p>` : ""}
    </section>`,
    )
    .join("");
}

$("#guide-q")?.addEventListener("input", (e) => renderGuide(e.target.value));
$("#btn-page-guide")?.addEventListener("click", () => {
  const id = state.view || "accounts";
  goto("guide");
  renderGuide("");
  if ($("#guide-q")) $("#guide-q").value = "";
  setTimeout(() => {
    const el = document.getElementById("g-" + id);
    if (el) el.scrollIntoView({ behavior: "smooth", block: "start" });
    else toast("这一页还没有单独教程，已打开总目录");
  }, 60);
});


/* ---------- 模型映射 ---------- */
async function loadModelMap() {
  const ta = $("#model-map");
  if (!ta) return;
  try {
    const m = await invoke("model_map_get");
    ta.value = m && typeof m === "object"
      ? Object.entries(m).map(([k, v]) => `${k}=${v}`).join("\n")
      : "";
    if ($("#mmap-msg")) $("#mmap-msg").textContent = "";
  } catch (e) {
    if ($("#mmap-msg")) $("#mmap-msg").textContent = "读取失败：" + e;
  }
}
$("#btn-mmap-reload")?.addEventListener("click", loadModelMap);
$("#btn-mmap-save")?.addEventListener("click", async () => {
  const ta = $("#model-map");
  const msg = $("#mmap-msg");
  const obj = {};
  const bad = [];
  String(ta.value || "")
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l && !l.startsWith("#"))
    .forEach((line) => {
      const i = line.indexOf("=");
      if (i <= 0) return bad.push(line);
      obj[line.slice(0, i).trim()] = line.slice(i + 1).trim();
    });
  if (bad.length) {
    if (msg) msg.textContent = "这几行格式不对（要 a=b）：" + bad.join(" / ");
    return;
  }
  try {
    await invoke("model_map_set", { map: obj });
    if (msg) msg.textContent = `已保存 ${Object.keys(obj).length} 条映射`;
    toast("模型映射已保存");
  } catch (e) {
    if (msg) msg.textContent = "保存失败：" + e;
  }
});

/* ---------- 更新日志（编译期嵌进 exe，离线可看） ---------- */
function mdToHtml(md) {
  const inline = (t) =>
    esc(t)
      .replace(/\*\*(.+?)\*\*/g, "<b>$1</b>")
      .replace(/`([^`]+)`/g, "<code>$1</code>");
  const raw = String(md || "").split("\n");
  const merged = [];
  for (const l of raw) {
    if (/^ {2,}\S/.test(l) && merged.length) merged[merged.length - 1] += " " + l.trim();
    else merged.push(l);
  }
  const out = [];
  let inUl = false;
  for (const line of merged) {
    const t = line.replace(/\s+$/, "");
    if (/^- /.test(t)) {
      if (!inUl) { out.push("<ul>"); inUl = true; }
      out.push(`<li>${inline(t.slice(2))}</li>`);
      continue;
    }
    if (inUl) { out.push("</ul>"); inUl = false; }
    if (!t.trim()) continue;
    if (t.startsWith("### ")) out.push(`<h4>${inline(t.slice(4))}</h4>`);
    else if (t.startsWith("## ")) out.push(`<h3>${inline(t.slice(3))}</h3>`);
    else if (t.startsWith("# ")) out.push(`<h2>${inline(t.slice(2))}</h2>`);
    else out.push(`<p>${inline(t)}</p>`);
  }
  if (inUl) out.push("</ul>");
  return out.join("\n");
}

async function renderChangelog(force) {
  const el = $("#cl-body");
  if (!el) return;
  if (state.changelog?.text && !force) {
    el.innerHTML = mdToHtml(state.changelog.text);
    return;
  }
  try {
    const r = await invoke("changelog");
    state.changelog = r;
    if ($("#cl-version")) $("#cl-version").textContent = "当前版本 v" + r.version;
    el.innerHTML = mdToHtml(r.text || "（CHANGELOG.md 为空）");
  } catch (e) {
    el.textContent = "读取失败：" + e;
  }
}
$("#btn-cl-github")?.addEventListener("click", async () => {
  try {
    await invoke("open_external", { url: "https://github.com/ax2697/axiao-gateway/releases" });
  } catch (e) {
    toast(String(e));
  }
});


/* ---------- 创作页交互：分段模式 / 字数 / 素材清单 / 结果区 ---------- */
document.addEventListener("click", (e) => {
  const seg = e.target.closest("#cr-mode-seg .seg-item");
  if (!seg) return;
  $$("#cr-mode-seg .seg-item").forEach((b) => b.classList.toggle("on", b === seg));
  const sel = $("#cr-mode");
  if (sel) {
    sel.value = seg.dataset.mode;
    sel.dispatchEvent(new Event("change"));
  }
});
$("#cr-prompt")?.addEventListener("input", (e) => {
  if ($("#cr-len")) $("#cr-len").textContent = String(e.target.value.length);
});
$("#cr-image")?.addEventListener("change", (e) => {
  const box = $("#cr-files");
  if (!box) return;
  const fs = Array.from(e.target.files || []);
  box.innerHTML = fs.length
    ? fs
        .map(
          (f) =>
            `<span title="${esc(f.name)}">${f.type.startsWith("image/") ? "图" : f.type.startsWith("video/") ? "视频" : "音频"} · ${esc(
              f.name.length > 18 ? f.name.slice(0, 16) + "…" : f.name,
            )}</span>`,
        )
        .join("")
    : "";
});
function showCrResult() {
  const r = $("#cr-result");
  if (r) r.hidden = false;
}

/* ---------- 创作：图像 / 视频 ---------- */
function renderCreate() {
  const sel = $("#cr-upstream");
  if (!sel) return;
  const list = (state.upstreams || []).filter((u) => u.enabled !== false);
  const keep = sel.value;
  sel.innerHTML = list.length
    ? list.map((u) => `<option value="${esc(u.id)}">${esc(u.name)}${u.apiKey ? "" : "（未填 Key）"}</option>`).join("")
    : `<option value="">（先去「上游管理」添加上游）</option>`;
  if (keep && list.some((u) => u.id === keep)) sel.value = keep;
  syncCrMode();
}

function crMode() {
  return $("#cr-mode")?.value || "t2i";
}
function syncCrMode() {
  const m = crMode();
  const isVideo = m === "t2v" || m === "i2v";
  const needAsset = m !== "t2i"; // 视频模式都允许给参考素材（首帧图 / 参考视频 / 配乐）
  if ($("#cr-img-wrap")) $("#cr-img-wrap").hidden = !needAsset;
  if ($("#cr-dur-wrap")) $("#cr-dur-wrap").hidden = !isVideo;
}

function readFileAsDataURL(file) {
  return new Promise((res, rej) => {
    const fr = new FileReader();
    fr.onload = () => res(String(fr.result));
    fr.onerror = rej;
    fr.readAsDataURL(file);
  });
}

function showRaw(box, status, data) {
  box.innerHTML = `<pre class="log">HTTP ${status}\n${esc(JSON.stringify(data, null, 2)).slice(0, 1800)}</pre>`;
}

async function pollVideo(up, taskId, zhipu, box, rounds = 20) {
  const tip = document.createElement("div");
  tip.className = "muted";
  box.appendChild(tip);
  for (let i = 1; i <= rounds; i++) {
    await new Promise((r) => setTimeout(r, 5000));
    tip.textContent = `任务 ${taskId} 第 ${i} 次查询…`;
    try {
      const r = await invoke("video_poll", { upstream: up, taskId, zhipu });
      const d = r.data || {};
      const st = d.task_status || d.status || d.state || "";
      const url = d.video_result?.[0]?.url || d.url || d.output?.url || d.data?.url || "";
      if (url) {
        tip.textContent = "生成完成";
        box.insertAdjacentHTML(
          "afterbegin",
          `<video src="${esc(url)}" controls style="max-width:100%;border-radius:10px"></video>
           <div class="btn-row" style="margin:10px 0"><button class="btn sm" data-save-media="${esc(url)}" data-ext="mp4">保存视频</button></div>`,
        );
        return;
      }
      if (String(st).toLowerCase().includes("fail")) {
        tip.textContent = "任务失败：" + JSON.stringify(d).slice(0, 200);
        return;
      }
    } catch (e) {
      tip.textContent = "查询出错：" + e;
      return;
    }
  }
  tip.textContent = "还没出结果，稍后点「重新查询」或用任务 ID 去平台后台看图。";
}

$("#cr-mode")?.addEventListener("change", syncCrMode);
$("#btn-cr-run")?.addEventListener("click", async () => {
  const up = $("#cr-upstream").value;
  const prompt = $("#cr-prompt").value.trim();
  const msg = $("#cr-msg");
  const box = $("#cr-result");
  const btn = $("#btn-cr-run");
  if (!up) return toast("先选一个上游（去「上游管理」添加）");
  if (!prompt) return toast("先写提示词");
  const m = crMode();
  const files = Array.from($("#cr-image")?.files || []);
  // 按 MIME 分流：图 → image，视频 → video，音频 → audio
  let image = null, video = null, audio = null;
  for (const f of files) {
    const data = await readFileAsDataURL(f);
    if (f.type.startsWith("image/")) image = data;
    else if (f.type.startsWith("video/")) video = data;
    else if (f.type.startsWith("audio/")) audio = data;
  }
  if (m === "i2i" && !image) return toast("图生图需要一张参考图");
  let extra = null;
  const rawExtra = $("#cr-extra").value.trim();
  if (rawExtra) {
    try {
      extra = JSON.parse(rawExtra);
    } catch {
      return toast("高级参数不是合法 JSON");
    }
  }
  const size = $("#cr-size").value.trim() || null;
  const duration = Number($("#cr-duration")?.value || 0) || null;
  const model = $("#cr-model").value.trim() || null;
  btn.disabled = true;
  msg.textContent = "生成中…（图片十几秒，视频要几分钟）";
  box.innerHTML = "";
  showCrResult();
  try {
    if (m === "t2i" || m === "i2i") {
      const r = await invoke("image_generate", { upstream: up, prompt, model, size, image, extra });
      const item = (r.data?.data || [])[0] || {};
      const src = item.url || (item.b64_json ? `data:image/png;base64,${item.b64_json}` : "");
      if (r.status >= 300 || !src) {
        showRaw(box, r.status, r.data);
        msg.textContent = `失败（HTTP ${r.status}）${r.path || ""}`;
      } else {
        box.innerHTML = `<img src="${esc(src)}" style="max-width:100%;border-radius:10px;border:1px solid var(--border-soft)" />
          <div class="btn-row" style="margin-top:10px"><button class="btn sm" data-save-media="${esc(src)}" data-ext="png">保存到图片文件夹</button></div>`;
        msg.textContent = "完成";
        await invoke("append_hook", { kind: "media", detail: `文生图 ${up} ${model || ""}`.trim() });
      }
    } else if (m === "t2a") {
      // 音乐：豆包桥的 /v1/audio/generations，字段整体透传
      const r = await invoke("audio_generate", { upstream: up, prompt, model, extra });
      const d = r.data || {};
      const item = (d.data || [])[0] || d;
      const src = item.url || item.audio_url || item.audio ||
        (item.b64_json ? `data:audio/mp3;base64,${item.b64_json}` : "");
      if (r.status >= 300) {
        showRaw(box, r.status, r.data);
        msg.textContent = `失败（HTTP ${r.status}）${r.path || ""}`;
      } else if (!src) {
        showRaw(box, r.status, r.data);
        msg.textContent = "上游返回了内容，但没找到音频地址（看下面的原始返回）";
      } else {
        box.innerHTML = `<audio controls src="${esc(src)}" style="width:100%;margin-bottom:10px"></audio>
          <div class="btn-row"><button class="btn sm" data-save-media="${esc(src)}" data-ext="mp3">保存到文件</button></div>`;
        msg.textContent = "完成";
        await invoke("append_hook", { kind: "media", detail: `文生音乐 ${up} ${model || ""}`.trim() });
      }
    } else {
      const r = await invoke("video_generate", { upstream: up, prompt, model, image, video, audio, duration, extra });
      const d = r.data || {};
      const taskId = d.id || d.task_id || d.data?.id || d.request_id || "";
      if (r.status >= 300 || !taskId) {
        showRaw(box, r.status, r.data);
        msg.textContent = `失败（HTTP ${r.status}）：该平台可能没有视频接口`;
      } else {
        msg.textContent = `已提交，任务 ${taskId}`;
        box.innerHTML = `<pre class="log">${esc(JSON.stringify(d, null, 2)).slice(0, 800)}</pre>`;
        pollVideo(up, taskId, r.zhipu, box);
      }
    }
  } catch (e) {
    msg.textContent = "失败：" + e;
  }
  btn.disabled = false;
});

document.addEventListener("click", async (e) => {
  const s = e.target.closest("[data-save-media]");
  if (!s) return;
  try {
    const p = await invoke("media_save", { urlOrB64: s.dataset.saveMedia, ext: s.dataset.ext || "png" });
    toast("已保存：" + p);
  } catch (err) {
    toast("保存失败：" + err);
  }
});

/* ---------- 模型（实时抓取） ---------- */
function modelsIsWorkBuddy(u) {
  return /copilot\.tencent\.com/i.test(u?.baseUrl || "") || /workbuddy/i.test(u?.id || "");
}

function effortSelect(m) {
  const cur = (state.extras?.thinking?.models || {})[m.id] || "default";
  const sup = (m.supported_efforts || []).map((x) => String(x)).filter(Boolean);
  const list = sup.length ? sup : ["minimal", "low", "medium", "high", "xhigh", "max"];
  const opts = [`<option value="default"${cur === "default" ? " selected" : ""}>默认</option>`]
    .concat(list.map((e) => `<option value="${e}"${cur === e ? " selected" : ""}>${e}</option>`));
  return `<select class="btn sm" data-model-effort="${esc(m.id)}" title="${sup.length ? "该模型支持：" + sup.join(" / ") : "该模型未标注支持的档位"}">${opts.join("")}</select>`;
}

function wbModelRows(rows) {
  return rows.length
    ? rows.map((m) => {
        const caps = [m.supports_images ? "视觉" : "", m.only_reasoning ? "思考" : "工具"].filter(Boolean).join(" / ");
        return `<tr>
          <td>${esc(m.name || m.id)}<div class="sub" style="color:var(--muted);font-size:11px">${esc(m.id || "")}</div></td>
          <td>${esc(m.credits || "—")}</td>
          <td>${esc(caps || "—")}</td>
          <td>${effortSelect(m)}</td>
          <td class="num">${k(m.context_length)}</td>
          <td class="num">${k(m.max_output_tokens)}</td>
        </tr>`;
      }).join("")
    : `<tr><td colspan="6" class="empty">${state.panelError ? "面板不可达，先启动便携版网关（7863）" : "暂无模型"}</td></tr>`;
}

/** 从各平台 /models 的返回里尽量捞出价格字段（各家字段名不统一） */
function modelPrice(m) {
  const p = m.pricing || m.price;
  if (p && typeof p === "object") {
    const i = p.input ?? p.prompt ?? p.input_price;
    const o = p.output ?? p.completion ?? p.output_price;
    if (i != null || o != null) return `in ${i ?? "?"} / out ${o ?? "?"}`;
    if (typeof p === "string") return p;
  }
  if (typeof p === "string") return p;
  if (m.input_price != null || m.output_price != null) return `in ${m.input_price ?? "?"} / out ${m.output_price ?? "?"}`;
  return "";
}

function otherModelBody(u) {
  const c = (state.modelCache || {})[u.id] || {};
  if (c.loading) return `<div class="empty">拉取中…</div>`;
  if (c.error) return `<div class="empty">拉取失败：${esc(c.error)}</div>`;
  if (c.models?.length) {
    return `<div style="display:flex;flex-wrap:wrap;gap:6px;padding:12px 4px 4px">
      ${c.models
        .map((m) => {
          const id = m.id || m.name || m;
          const pr = modelPrice(m);
          return `<span class="pill gray" title="${esc(m.owned_by || "")}">${esc(id)}${pr ? ` · ${esc(pr)}` : ""}</span>`;
        })
        .join("")}
    </div>
    <p class="hint" style="padding:8px 4px 12px">上面是 ${esc(u.name)} 自己 <code>/models</code> 返回的清单${c.models.some(modelPrice) ? "（带它返回的价格字段）" : "，它没有返回价格字段"}。<b>同一个模型在不同平台价格不一样</b>，以该平台计费页为准；WorkBuddy 的价格在上面的「积分倍率」列。</p>`;
  }
  if (c.status) {
    return `<div class="empty">该平台没返回模型列表（HTTP ${c.status}）${c.raw ? `：<code>${esc(JSON.stringify(c.raw).slice(0, 200))}</code>` : ""}</div>`;
  }
  return `<div class="empty">还没拉取。点右上「刷新」按 ${esc(u.baseUrl)}/models 取一次${u.apiKey ? "" : "（该上游没填 Key，多半会 401）"}。</div>`;
}

function renderModels() {
  const rows = state.panel.models || [];
  const ups = state.upstreams || [];
  const wb = ups.find(modelsIsWorkBuddy);
  const others = ups.filter((u) => !modelsIsWorkBuddy(u));
  const el = $("#model-groups");
  if (!el) return;

  const pulled = Object.values(state.modelCache || {}).reduce((s, c) => s + ((c.models || []).length), 0);
  const src = $("#model-source");
  if (src) {
    src.textContent = state.panelError
      ? `面板未连上 · 其它平台 ${others.length} 个`
      : `WorkBuddy ${rows.length} 个 · 其它平台 ${others.length} 个 · 已拉 ${pulled} 个`;
  }

  let html = `<div class="card" style="margin:0">
    <div class="card-head"><h3 style="font-size:14px">${upLogo("workbuddy")}${esc(wb?.name || "WorkBuddy / CodeBuddy")}</h3>
      <span class="muted">${state.panelError ? "面板不可达（先启动便携版网关 7863）" : `${rows.length} 个 · 面板实时目录`}</span></div>
    <div class="table-wrap"><table class="tbl">
      <thead><tr><th>模型</th><th>积分倍率</th><th>能力</th><th>思考档位</th><th class="num">上下文</th><th class="num">最大输出</th></tr></thead>
      <tbody>${wbModelRows(rows)}</tbody></table></div>
    <p class="hint" style="padding:8px 4px 12px">「积分倍率」是 <b>WorkBuddy 平台自己的计价单位</b>，同一个模型在别家平台的价格完全不同（见下面各平台卡片）。思考档位按模型单独设，只列该模型支持的档位。</p>
  </div>`;

  html += others.length
    ? others.map((u) => `<div class="card" style="margin:14px 0 0">
        <div class="card-head"><h3 style="font-size:14px">${upLogo(u.id + " " + u.name)}${esc(u.name)}</h3>
          <div class="btn-row"><span class="muted">${esc(u.baseUrl)}${u.apiKey ? "" : " · 无 Key"}</span>
          <button class="btn sm" data-model-refresh="${esc(u.id)}">刷新</button></div></div>
        ${otherModelBody(u)}
      </div>`).join("")
    : `<div class="card" style="margin:14px 0 0"><div class="empty">还没有其它平台。去「添加账号 → 其他平台」加一个，模型会按平台分开列在这里。</div></div>`;

  el.innerHTML = html;
}

document.addEventListener("change", async (e) => {
  const s = e.target.closest("[data-model-effort]");
  if (!s) return;
  const t = state.extras?.thinking || { level: "default", per: {} };
  const models = { ...(t.models || {}) };
  const mid = s.dataset.modelEffort;
  if (s.value === "default") delete models[mid];
  else models[mid] = s.value;
  try {
    await invoke("extras_save", { key: "thinking", value: { ...t, models } });
    state.extras = await invoke("extras_get");
    toast(`已设置 ${mid} → ${s.value}`);
  } catch (err) {
    toast("保存失败：" + err);
  }
});

async function loadUpstreamModelList(id) {
  state.modelCache = state.modelCache || {};
  state.modelCache[id] = { ...(state.modelCache[id] || {}), loading: true, error: null };
  renderModels();
  try {
    const r = await invoke("upstream_models", { id });
    state.modelCache[id] = { loading: false, status: r.status, models: r.models || [], raw: r.raw };
  } catch (err) {
    state.modelCache[id] = { loading: false, error: String(err) };
  }
  renderModels();
}

document.addEventListener("click", async (e) => {
  const b = e.target.closest("[data-model-refresh]");
  if (b) await loadUpstreamModelList(b.dataset.modelRefresh);
});
$("#btn-models-refresh")?.addEventListener("click", async () => {
  const others = (state.upstreams || []).filter((u) => !modelsIsWorkBuddy(u));
  await Promise.all(others.map((u) => loadUpstreamModelList(u.id)));
  toast(others.length ? `已刷新 ${others.length} 个平台` : "没有其它平台");
});

/* ---------- 其它 ---------- */
/* 上游 / 平台图标：按 id 或名字关键词匹配，匹配不到用首字母色块 */
const UP_LOGO_RULES = [
  [/workbuddy|codebuddy/i, "workbuddy.png"],
  [/trae/i, "trae.png"],
  [/qoder/i, "qoder.png"],
  [/zcode|z\.ai|zhipu_?z/i, "zcode.svg"],
  [/minimax/i, "minimax.ico"],
  [/doubao|豆包/i, "doubao.png"],
  [/dsh|deepseek/i, "deepseek.ico"],
  [/codearts|华为/i, "codearts.ico"],
  [/opencode/i, "opencode.png"],
  [/kimi|moonshot/i, "kimi.png"],
  [/bigmodel|智谱|glm/i, "bigmodel.png"],
  [/commandcode/i, "commandcode.svg"],
  [/baichuan|百川/i, "baichuan.png"],
  [/lingyi|零一/i, "lingyi.png"],
  [/spark|讯飞/i, "spark.png"],
  [/hunyuan|混元/i, "hunyuan.png"],
  [/siliconflow|硅基/i, null],
  [/openai|gpt/i, null],
  [/anthropic|claude/i, null],
  [/gemini|google/i, null],
  [/vscode/i, "vscode.png"],
  [/jetbrains|idea|pycharm/i, "jetbrains.png"],
];
function upLogo(nameOrId) {
  const key = String(nameOrId || "");
  for (const [re, file] of UP_LOGO_RULES) {
    if (re.test(key)) {
      if (file) return `<img class="logo-img sm" src="assets/logos/${file}" alt="" />`;
      break;
    }
  }
  return `<span class="logo-fallback">${esc(key.trim().charAt(0) || "?")}</span>`;
}

function renderUpstreams() {
  const rows = state.upstreams || [];
  const tb = $("#tb-upstreams");
  if (!tb) return;
  tb.innerHTML = rows.length
    ? rows.map((u) => {
        const h = (state.health || []).find((x) => x.id === u.id);
        const bk = (state.breaker || []).find((x) => x.id === u.id);
        // 冷却中的渠道比"历史成功率"更值得优先显示——它现在就是不可用的
        const health = bk && bk.cooling
          ? `<span class="pill red">冷却中 ${bk.coolRemainSec}s</span>
             <div class="sub" style="font-size:11px;color:var(--muted)">${esc(bk.lastError || "")}</div>
             <button class="btn sm" style="margin-top:4px" data-breaker-reset="${esc(u.id)}">立即恢复</button>`
          : h
          ? `<span class="pill ${h.okRate >= 90 ? "green" : h.okRate >= 60 ? "orange" : "red"}">${h.okRate}%</span>
             <div class="sub" style="font-size:11px;color:var(--muted)">${h.ok}/${h.calls} 成功${h.avgMs ? ` · ${h.avgMs}ms` : ""}</div>`
          : `<span class="muted" style="font-size:11.5px">无记录</span>`;
        const same = rows.filter((x) => x.name === u.name);
        const idx = same.findIndex((x) => x.id === u.id) + 1;
        const dup = same.length > 1 ? `<span class="pill blue" style="margin-left:6px">#${idx}/${same.length}</span>` : "";
        return `<tr>
        <td><code>${esc(u.id)}</code></td>
        <td>${upLogo(u.id + " " + u.name)}<b style="font-weight:600">${esc(u.name)}</b>${dup}</td>
        <td>${esc(u.baseUrl)}${u.model ? `<div class="sub" style="font-size:11px;color:var(--muted)">模型 ${esc(u.model)}</div>` : ""}</td>
        <td>${esc(u.protocol)}</td>
        <td class="num">${u.weight}</td>
        <td>${health}</td>
        <td>${u.enabled ? `<span class="pill green">启用</span>` : `<span class="pill gray">停用</span>`}${u.apiKey ? "" : ` <span class="pill orange">无 Key</span>`}</td>
        <td class="op"><div class="btn-row" style="justify-content:flex-end">
          <button class="btn sm" data-up-edit="${esc(u.id)}">编辑</button>
          <button class="btn sm" data-up-dup="${esc(u.id)}" title="同平台再加一把 Key，会自动轮着用">再加一条</button>
          <button class="btn sm" data-up-toggle="${esc(u.id)}">${u.enabled ? "停用" : "启用"}</button>
          <button class="btn sm danger" data-up-del="${esc(u.id)}">删除</button>
        </div></td>
      </tr>`;
      }).join("")
    : `<tr><td colspan="8" class="empty">暂无上游。点右上「新增上游」，或在「添加账号 → 其他平台」里选平台填 Key。</td></tr>`;
}

/* 上游：编辑 / 启停 / 删除 */
function openUpstreamModal(u) {
  state.upstreamEdit = u ? u.id : null;
  $("#u-id").value = u?.id || "";
  $("#u-id").disabled = !!u;
  $("#u-name").value = u?.name || "";
  $("#u-base").value = u?.baseUrl || "";
  $("#u-key").value = u?.apiKey || "";
  $("#u-proto").value = u?.protocol || "openai";
  $("#u-weight").value = u?.weight ?? 1;
  if ($("#u-model")) $("#u-model").value = u?.model || "";
  $("#modal-upstream").classList.add("show");
}
document.addEventListener("click", async (e) => {
  const dup = e.target.closest("[data-up-dup]");
  if (dup) {
    const u = (state.upstreams || []).find((x) => x.id === dup.dataset.upDup);
    if (!u) return;
    // 复制平台/地址/协议/模型，Key 留空，id 由后端或保存时自动生成
    openUpstreamModal({ ...u, id: "", apiKey: "", name: u.name });
    toast("已复制平台信息，填上新的 Key 保存即可（会自动轮询）");
    return;
  }
  const ed = e.target.closest("[data-up-edit]");
  if (ed) {
    const u = (state.upstreams || []).find((x) => x.id === ed.dataset.upEdit);
    if (u) openUpstreamModal(u);
    return;
  }
  const tg = e.target.closest("[data-up-toggle]");
  if (tg) {
    const u = (state.upstreams || []).find((x) => x.id === tg.dataset.upToggle);
    if (!u) return;
    try {
      await invoke("save_upstream", { upstream: { ...u, enabled: !u.enabled } });
      toast(u.enabled ? "已停用 " + u.name : "已启用 " + u.name);
      loadAll();
    } catch (err) {
      toast("操作失败：" + err);
    }
    return;
  }
  const del = e.target.closest("[data-up-del]");
  if (del) {
    const u = (state.upstreams || []).find((x) => x.id === del.dataset.upDel);
    if (!u) return;
    if (!confirm(`删除上游「${u.name}」？对应账号条目也会一起移除。`)) return;
    try {
      await invoke("delete_upstream", { id: u.id });
      // 同步删掉影子账号
      const acc = (state.bundle.accounts || []).find((a) => a.accountId === u.id && a.client === "apikey");
      if (acc) await invoke("delete_account", { id: acc.id }).catch(() => {});
      toast("已删除 " + u.name);
      loadAll();
    } catch (err) {
      toast("删除失败：" + err);
    }
  }
});

/* ---------- 网关服务日志（本地 API 的收/发/错） ---------- */
// 格式："2026-11-04 02:30:11.123 [INFO] 正文"
const GWLOG_RE = /^(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d+)\s+\[(\w+)\]\s*([\s\S]*)$/;

async function loadGatewayLogs() {
  let r;
  try {
    r = await invoke("gateway_logs", { tailLines: 800 });
  } catch (e) {
    state.gwlogs = [];
    state.gwlogError = String(e);
    renderGatewayLogs();
    return;
  }
  state.gwlogError = null;
  state.gwlogs = r.lines || [];
  state.gwlogPath = r.path || "";
  state.gwlogSize = r.size || 0;
  renderGatewayLogs();
}

function renderGatewayLogs() {
  const tb = $("#tb-gwlog");
  if (!tb) return;
  const all = state.gwlogs || [];
  const rows = all
    .map((l) => {
      const m = String(l).match(GWLOG_RE);
      return m ? { ts: m[1], level: m[2], msg: m[3] } : { ts: "", level: "", msg: String(l) };
    })
    .reverse(); // 最新的排最上面，排障先看刚发生的

  const lv = state.gwlogLevel || "";
  const q = (state.gwlogFilter || "").toLowerCase();
  const shown = rows.filter((r) => {
    if (lv && r.level !== lv) return false;
    if (q && !r.msg.toLowerCase().includes(q)) return false;
    return true;
  });

  const meta = $("#gwlog-meta");
  if (meta) {
    meta.textContent = state.gwlogError
      ? "读取失败"
      : `${shown.length}/${rows.length} 条 · ${((state.gwlogSize || 0) / 1024).toFixed(1)} KB`;
    meta.title = state.gwlogPath || "";
  }

  if (!shown.length) {
    const empty = state.gwlogError
      ? `读不到日志文件：${esc(state.gwlogError)}`
      : all.length
        ? "没有匹配的记录，换个级别或关键字试试"
        : "还没有服务日志。到「本地 API」页点「启动服务」，或用客户端请求一次 127.0.0.1:7864 就会出现。";
    tb.innerHTML = `<tr><td colspan="3" class="empty">${empty}</td></tr>`;
    return;
  }

  tb.innerHTML = shown
    .map((r) => {
      const color = r.level === "WARN" ? "var(--amber, #d97706)" : "var(--muted)";
      const tag = r.level ? `<span style="color:${color};font-size:12px">${esc(r.level)}</span>` : "—";
      return `<tr><td class="mono" style="font-size:12px">${esc(r.ts || "—")}</td><td>${tag}</td><td>${esc(r.msg)}</td></tr>`;
    })
    .join("");
}

$("#gwlog-q")?.addEventListener("input", (e) => {
  state.gwlogFilter = e.target.value.trim();
  renderGatewayLogs();
});
$("#gwlog-level")?.addEventListener("change", (e) => {
  state.gwlogLevel = e.target.value;
  renderGatewayLogs();
});
$("#btn-gwlog-refresh")?.addEventListener("click", () => loadGatewayLogs());
$("#btn-gwlog-clear")?.addEventListener("click", async () => {
  if (!confirm("清空网关服务日志？只删日志文件，不动账本和统计数据。")) return;
  try {
    await invoke("gateway_logs_clear");
    toast("已清空");
    loadGatewayLogs();
  } catch (e) {
    toast("清空失败：" + e);
  }
});
// 服务日志会被后台线程不停写入，停在日志页时每 8 秒自动拉一次
setInterval(() => {
  if (state.view === "logs" && hasTauri) loadGatewayLogs();
}, 8000);

function renderHooks() {
  const rows = state.hooks || [];

  // 运行日志页：类型 + 关键字过滤
  const sel = $("#log-kind");
  if (sel) {
    const cur = sel.value;
    const kinds = Array.from(new Set(rows.map((h) => h.kind).filter(Boolean))).sort();
    sel.innerHTML =
      `<option value="">全部类型（${rows.length}）</option>` +
      kinds
        .map((k) => `<option value="${esc(k)}">${esc(k)}（${rows.filter((h) => h.kind === k).length}）</option>`)
        .join("");
    if (cur && kinds.includes(cur)) sel.value = cur;
  }
  const f = state.logFilter || { kind: "", q: "" };
  const shown = rows.filter((h) => {
    if (f.kind && h.kind !== f.kind) return false;
    if (f.q && !String(h.detail || "").toLowerCase().includes(f.q.toLowerCase())) return false;
    return true;
  });
  const cnt = $("#log-count");
  if (cnt) cnt.textContent = shown.length === rows.length ? `${rows.length} 条` : `${shown.length} / ${rows.length} 条`;

  const html = shown.length
    ? shown.map((h) => `<tr><td>${fmt(h.ts)}</td><td>${esc(h.kind)}</td><td>${esc(h.detail)}</td></tr>`).join("")
    : `<tr><td colspan="3" class="empty">${rows.length ? "没有匹配的记录，换个类型或关键字试试" : "暂无事件（签到 / 切号 / 测试等操作后会写入）"}</td></tr>`;

  $("#tb-logs").innerHTML = html;
  $("#tb-hooks").innerHTML = html;
  $("#tb-dash-hooks").innerHTML = rows.length
    ? rows.slice(0, 5).map((h) => `<tr><td>${fmt(h.ts)}</td><td>${esc(h.kind)}</td><td>${esc(h.detail)}</td></tr>`).join("")
    : `<tr><td colspan="3" class="empty">暂无事件</td></tr>`;
  try { renderPageCharts(); } catch {}
}

state.logFilter = { kind: "", q: "" };
$("#log-kind")?.addEventListener("change", (e) => {
  state.logFilter.kind = e.target.value;
  renderHooks();
});
$("#log-q")?.addEventListener("input", (e) => {
  state.logFilter.q = e.target.value.trim();
  renderHooks();
});
$("#btn-log-refresh")?.addEventListener("click", () => loadAll());

function renderRequests() {
  const rows = state.ledger || [];
  const sel = $("#req-upstream");
  if (sel) {
    const cur = sel.value;
    const ups = Array.from(new Set(rows.map((r) => r.upstream).filter(Boolean))).sort();
    sel.innerHTML =
      `<option value="">全部上游（${rows.length}）</option>` +
      ups.map((u) => `<option value="${esc(u)}">${esc(u)}（${rows.filter((r) => r.upstream === u).length}）</option>`).join("");
    if (cur && ups.includes(cur)) sel.value = cur;
  }
  const pick = sel?.value || "";
  const shown = pick ? rows.filter((r) => r.upstream === pick) : rows;
  const cnt = $("#req-count");
  if (cnt) cnt.textContent = shown.length === rows.length ? `${rows.length} 条` : `${shown.length} / ${rows.length} 条`;
  const tb = $("#tb-requests");
  if (!tb) return;
  tb.innerHTML = shown.length
    ? shown.map((r) => `<tr>
        <td>${fmt(r.ts)}</td>
        <td>${esc((r.accountId || "—").slice(0, 12))}</td>
        <td>${esc(r.upstream)}</td>
        <td>${esc(r.model)}</td>
        <td class="num">${r.promptTokens}</td>
        <td class="num">${r.completionTokens}</td>
      </tr>`).join("")
    : `<tr><td colspan="6" class="empty">${rows.length ? "该上游没有记录" : "还没有经网关的请求。去「协议与路由」点一次「对话测试」就能看到第一条。"}</td></tr>`;
}

$("#req-upstream")?.addEventListener("change", () => renderRequests());

/* ---------- 配置（真实读写面板 config） ---------- */
function cv(id, v) { const el = document.getElementById(id); if (el) el.value = v ?? ""; }
function cc(id, v) { const el = document.getElementById(id); if (el) el.checked = !!v; }
function gv(id) { return document.getElementById(id)?.value ?? ""; }
function gc(id) { return !!document.getElementById(id)?.checked; }
const hoursStr = (v) => Array.isArray(v) ? v.join(",") : String(v ?? "");
const parseHours = (s) => String(s).split(",").map((x) => parseInt(x.trim(), 10)).filter((n) => !Number.isNaN(n));
const num = (v) => { const n = Number(v); return Number.isNaN(n) ? 0 : n; };

async function loadConfig() {
  if (!hasTauri) return;
  try {
    let c, path;
    try {
      const r = await invoke("local_config_get");
      c = r.config;
      path = r.path;
    } catch {
      const r = await invoke("panel_get_config");
      c = r.config || r;
      path = r.path;
    }
    state.panelConfig = c;
    $("#cfg-path").textContent = path ? "config: " + path : "";
    cv("cfg-listen", c.listen); cv("cfg-api-key", c.api_key); cv("cfg-pkg-limit", c.panel?.package_detail_limit);
    cv("cfg-checkin", hoursStr(c.schedule?.checkin_hours)); cv("cfg-keepalive", hoursStr(c.schedule?.keepalive_hours));
    cv("cfg-travel", hoursStr(c.schedule?.travel_hours)); cv("cfg-activity", hoursStr(c.schedule?.activity_hours));
    cv("cfg-balance", c.schedule?.balance_refresh_minutes); cv("cfg-growth", hoursStr(c.schedule?.growth_hours));
    cc("cfg-checkin-on", c.schedule?.checkin_enabled); cc("cfg-keepalive-on", c.schedule?.keepalive_enabled);
    cc("cfg-travel-on", c.schedule?.travel_enabled); cc("cfg-activity-on", c.schedule?.activity_enabled);
    cc("cfg-growth-on", c.schedule?.growth_enabled); cc("cfg-balance-on", c.schedule?.balance_refresh_enabled);
    cv("cfg-max-inflight", c.pool?.max_in_flight); cv("cfg-max-global", c.pool?.max_in_flight_global);
    cv("cfg-breaker-threshold", c.pool?.breaker_threshold); cv("cfg-soft-rate", c.cooldown?.soft_rate);
    cv("cfg-soft-rate-max", c.cooldown?.soft_rate_max); cv("cfg-breaker-cooldown", c.pool?.breaker_cooldown);
    cv("cfg-breaker-max", c.pool?.breaker_cooldown_max); cv("cfg-degrade-threshold", c.pool?.degrade_threshold);
    cv("cfg-degrade-cooldown", c.pool?.degrade_cooldown); cv("cfg-degrade-max", c.pool?.degrade_cooldown_max);
    cv("cfg-idle-weight", c.pool?.idle_weight_per_hour); cv("cfg-idle-max", c.pool?.idle_weight_max);
    cv("cfg-cost-window", c.pool?.cost_explore_interval); cv("cfg-credit-floor", c.pool?.credit_floor);
    cv("cfg-expiring-soon", c.pool?.expiring_soon); cv("cfg-sticky-ttl", c.session_sticky?.ttl);
    cc("cfg-prefer-expiring", c.pool?.prefer_expiring);
    cv("cfg-timeout", c.upstream?.timeout_seconds); cv("cfg-header-timeout", c.upstream?.header_timeout_seconds);
    cv("cfg-idle-timeout", c.upstream?.idle_timeout_seconds); cv("cfg-user-agent", c.upstream?.user_agent);
    cv("cfg-prompt-mode", c.prompt?.mode); cv("cfg-prompt-file", c.prompt?.file);
    cc("cfg-sanitize", c.features?.sanitize_blacklist_fingerprints); cc("cfg-sticky-on", c.session_sticky?.enabled);
    cv("cfg-retention", c.logging?.request_retention_days); cv("cfg-archive-mb", c.logging?.request_archive_max_mb);
    cc("cfg-archive", c.logging?.request_archive_enabled); cc("cfg-client-info", c.logging?.request_client_info);
  } catch (err) {
    $("#cfg-path").textContent = "配置读取失败：" + err;
  }
}

function collectConfig() {
  const c = JSON.parse(JSON.stringify(state.panelConfig || {}));
  c.listen = gv("cfg-listen"); c.api_key = gv("cfg-api-key");
  c.panel = { ...(c.panel || {}), package_detail_limit: num(gv("cfg-pkg-limit")) };
  c.schedule = {
    ...(c.schedule || {}),
    checkin_hours: parseHours(gv("cfg-checkin")), keepalive_hours: parseHours(gv("cfg-keepalive")),
    travel_hours: parseHours(gv("cfg-travel")), activity_hours: parseHours(gv("cfg-activity")),
    growth_hours: parseHours(gv("cfg-growth")), balance_refresh_minutes: num(gv("cfg-balance")),
    checkin_enabled: gc("cfg-checkin-on"), keepalive_enabled: gc("cfg-keepalive-on"),
    travel_enabled: gc("cfg-travel-on"), activity_enabled: gc("cfg-activity-on"),
    growth_enabled: gc("cfg-growth-on"), balance_refresh_enabled: gc("cfg-balance-on"),
  };
  c.pool = {
    ...(c.pool || {}),
    max_in_flight: num(gv("cfg-max-inflight")), max_in_flight_global: num(gv("cfg-max-global")),
    breaker_threshold: num(gv("cfg-breaker-threshold")), breaker_cooldown: gv("cfg-breaker-cooldown"),
    breaker_cooldown_max: gv("cfg-breaker-max"), degrade_threshold: num(gv("cfg-degrade-threshold")),
    degrade_cooldown: gv("cfg-degrade-cooldown"), degrade_cooldown_max: gv("cfg-degrade-max"),
    idle_weight_per_hour: num(gv("cfg-idle-weight")), idle_weight_max: num(gv("cfg-idle-max")),
    cost_explore_interval: gv("cfg-cost-window"), credit_floor: num(gv("cfg-credit-floor")),
    expiring_soon: gv("cfg-expiring-soon"), prefer_expiring: gc("cfg-prefer-expiring"),
  };
  c.cooldown = { ...(c.cooldown || {}), soft_rate: gv("cfg-soft-rate"), soft_rate_max: gv("cfg-soft-rate-max") };
  c.upstream = {
    ...(c.upstream || {}),
    timeout_seconds: num(gv("cfg-timeout")), header_timeout_seconds: num(gv("cfg-header-timeout")),
    idle_timeout_seconds: num(gv("cfg-idle-timeout")), user_agent: gv("cfg-user-agent"),
  };
  c.features = { ...(c.features || {}), sanitize_blacklist_fingerprints: gc("cfg-sanitize") };
  c.prompt = { ...(c.prompt || {}), mode: gv("cfg-prompt-mode"), file: gv("cfg-prompt-file") };
  c.session_sticky = { ...(c.session_sticky || {}), enabled: gc("cfg-sticky-on"), ttl: gv("cfg-sticky-ttl") };
  c.logging = {
    ...(c.logging || {}),
    request_retention_days: num(gv("cfg-retention")), request_archive_max_mb: num(gv("cfg-archive-mb")),
    request_archive_enabled: gc("cfg-archive"), request_client_info: gc("cfg-client-info"),
  };
  return c;
}

function renderComponents() {
  const rows = state.components || [];
  $("#comp-list").innerHTML = rows
    .map((c) => {
      // 三态：就绪（在跑/文件在） / 已内置（不需要外部文件） / 未安装（可选组件）
      const state2 = c.exists ? { pill: "green", text: "就绪" } : c.builtin ? { pill: "blue", text: "已内置" } : { pill: "gray", text: "未安装" };
      return `<div class="comp-item"><div><b>${esc(c.name)}</b><div class="sub">${esc(c.role)}</div><div class="sub mono">${esc(c.path)}</div>${c.detail ? `<div class="sub">${esc(c.detail)}</div>` : ""}</div>
      <div class="btn-row"><span class="pill ${state2.pill}">${state2.text}</span></div></div>`;
    })
    .join("");
  const cnt = $("#comp-count");
  if (cnt) cnt.textContent = `${rows.length} 个组件 · ${rows.filter((c) => c.exists).length} 个在跑`;
  $("#tb-vendor").innerHTML = VENDOR.map(([n, l, s, c]) => `<tr><td>${esc(n)}</td><td>${esc(l)}</td><td>${esc(s)}</td><td>${esc(c)}</td></tr>`).join("");
}

function renderSignin() {
  const sel = $("#sign-uid");
  if (!sel) return;
  const accs = state.bundle.accounts || [];
  sel.innerHTML = accs.map((a) => `<option value="${esc(a.accountId || a.id)}">${esc(a.displayName || a.accountId || a.id)}（${a.credits ?? 0}）</option>`).join("");
  $("#sign-source").textContent = accs.length ? `原生直连 · ${accs.length} 个账号` : "未读取到账号";
}

/* ---------- switch 非重复功能 ---------- */
const fmtNum = (n) => {
  const v = Number(n) || 0;
  if (v >= 1e9) return (v / 1e9).toFixed(2) + "B";
  if (v >= 1e6) return (v / 1e6).toFixed(2) + "M";
  if (v >= 1e3) return (v / 1e3).toFixed(1) + "K";
  return String(v);
};

function renderSessions() {
  const rows = state.sessions || [];
  const el = $("#sessions-sum");
  if (el) el.textContent = state.switchError ? "读取失败：" + state.switchError : `${rows.length} 个会话 · 来自 hook-events.jsonl`;
  const tb = $("#tb-sessions");
  if (!tb) return;
  tb.innerHTML = rows.length ? rows.map((s) => `<tr>
    <td><code>${esc((s.sessionId || "").slice(0, 8))}</code></td>
    <td>${esc(s.client || "—")}</td><td>${esc(s.model || "—")}</td>
    <td>${esc(s.cwd || "—")}</td><td>${esc(s.lastEvent || "—")}</td><td>${fmt(s.lastAt)}</td>
  </tr>`).join("") : `<tr><td colspan="6" class="empty">暂无会话（switch 的 hook.sh 触发后会写入）</td></tr>`;
  try { renderPageCharts(); } catch {}
}

function renderTokens() {
  const t = state.tokens || { total: 0, input: 0, output: 0, cached: 0, calls: 0, hitRate: 0, days: [] };
  const el = $("#token-stats");
  if (el) {
    el.innerHTML = [
      [fmtNum(t.total), "总 Token", ""],
      [fmtNum(t.input), "输入 Token", ""],
      [fmtNum(t.output), "输出 Token", ""],
      [Number(t.hitRate || 0).toFixed(1) + "%", "缓存命中率", "green"],
    ].map(([v, l, c]) => `<div class="stat"><div class="v ${c}">${v}</div><div class="l">${l}</div></div>`).join("");
  }
  const tb = $("#tb-tokens");
  if (!tb) return;
  const days = (t.days || []).slice().reverse();
  const rows = days.map((d) => `<tr>
    <td>${esc(d.date)}</td><td class="num">${fmtNum(d.input)}</td><td class="num">${fmtNum(d.output)}</td>
    <td class="num">${fmtNum(d.cached)}</td><td class="num">${fmtNum(d.total)}</td><td class="num">${d.calls}</td>
  </tr>`);
  // 合计行：让「卡片数字 = 明细相加」一眼可验
  if (days.length > 1) {
    const sum = days.reduce(
      (a, d) => ({
        input: a.input + d.input,
        output: a.output + d.output,
        cached: a.cached + d.cached,
        total: a.total + d.total,
        calls: a.calls + d.calls,
      }),
      { input: 0, output: 0, cached: 0, total: 0, calls: 0 },
    );
    rows.push(`<tr style="font-weight:600;background:var(--muted-bg)">
      <td>合计 ${days.length} 天</td><td class="num">${fmtNum(sum.input)}</td><td class="num">${fmtNum(sum.output)}</td>
      <td class="num">${fmtNum(sum.cached)}</td><td class="num">${fmtNum(sum.total)}</td><td class="num">${sum.calls}</td>
    </tr>`);
  }
  const note = $("#token-note");
  if (note) {
    note.innerHTML =
      `口径：<b>总 Token = 输入 + 输出</b>；<b>缓存读已包含在输入里</b>，所以命中率 = 缓存读 ÷ 输入。` +
      `数据来自本机 transcript（不是网关账本），点「刷新」重读。` +
      (t.normalized ? ` 有 <b>${t.normalized}</b> 条上游给的 total 与 input+output 不符，已按后者修正。` : "");
    // 窗口选大了数字不变时，得说清楚：是数据只有这些天，不是筛选坏了
    const wantDays = Number($("#token-days")?.value || 30);
    if (t.coveredDays) {
      note.innerHTML +=
        `<br/>当前窗口 <b>${wantDays}</b> 天，实际有数据 <b>${t.coveredDays}</b> 天` +
        (t.firstDay ? `（${esc(t.firstDay)} ~ ${esc(t.lastDay)}）` : "") +
        (t.coveredDays < wantDays ? "——窗口再选大也不会变多，因为更早没有记录。" : "");
    }
  }
  tb.innerHTML = rows.length
    ? rows.join("")
    : `<tr><td colspan="6" class="empty">暂无 token 记录（transcript 里未找到 /message/usage）</td></tr>`;
  renderTokenChart();
}

function renderCreditStats() {
  const p = state.creditCache?.payload || state.creditCache || {};
  const all = p.daily || [];
  const daily = all.filter((d) => Number(d.usage) > 0);
  const accs = state.bundle.accounts || [];
  const remain = accs.reduce((s, a) => s + (Number(a.credits) || 0), 0);
  const used = accs.reduce((s, a) => s + (Number(a.creditUsage) || 0), 0);

  const el = $("#creditstats-sum");
  if (el) {
    el.textContent = daily.length
      ? `${daily.length} 天有记录 · 合计 ${daily.reduce((s, d) => s + Number(d.usage), 0).toFixed(2)}`
      : `${accs.length} 个账号 · 剩余 ${remain} · 已用 ${used}`;
  }

  const tb = $("#tb-creditstats");
  if (!tb) return;
  if (daily.length) {
    tb.innerHTML = daily.slice().reverse().map((d) => `<tr>
      <td>${esc(d.date)}</td><td class="num">${(Number(d.usage) || 0).toFixed(2)}</td>
      <td>${esc((d.models || []).map((m) => (typeof m === "string" ? m : m.name || m.model || "")).join(", ") || "—")}</td>
    </tr>`).join("");
    return;
  }
  if (accs.length) {
    tb.innerHTML =
      accs
        .map((a) => `<tr>
          <td><div class="acc"><b>${esc(a.displayName || a.accountId || a.id)}</b><span>${esc(a.accountId || a.id)}</span></div></td>
          <td class="num">${Number(a.creditUsage) || 0}</td>
          <td>剩余 ${Number(a.credits) || 0}${a.creditExpiry ? ` · 到期 ${esc(String(a.creditExpiry).slice(0, 10))}` : ""}</td>
        </tr>`)
        .join("") +
      `<tr><td colspan="3" class="empty">上游没有开放「按日积分消耗」接口，switch 那份缓存 31 天也全为 0；这里改显示每个账号的累计已用（总额 − 剩余，来自 data/state.json）。按天看用量请去「Token 统计」。</td></tr>`;
    return;
  }
  tb.innerHTML = `<tr><td colspan="3" class="empty">暂无账号数据</td></tr>`;
  try { renderPageCharts(); } catch {}
}

/* 支持工具的图标：没有官方图标的用首字母色块兜底 */
const TOOL_LOGO = {
  workbuddy: "workbuddy.png",
  codebuddy_ide: "codebuddy.png",
  codebuddy_cli: "codebuddy.png",
  vscode_ext: "vscode.png",
  jetbrains: "jetbrains.png",
  trae: "trae.png",
  qoder: "qoder.png",
  zcode: "zcode.svg",
  dsh: "deepseek.ico",
  codearts: "codearts.ico",
  opencode_cli: "opencode.png",
  minimax: "minimax.ico",
  doubao: "doubao.png",
  monkeycode: "monkeycode.png",
  opencode_zen: "opencode.png",
  kimi: "kimi.png",
  bigmodel: "bigmodel.png",
  commandcode: "commandcode.svg",
  baichuan: "baichuan.png",
  lingyi: "lingyi.png",
  spark: "spark.png",
  hunyuan: "hunyuan.png",
};
function toolLogo(t) {
  const f = TOOL_LOGO[t.id];
  if (f) return `<img class="logo-img sm" src="assets/logos/${f}" alt="${esc(t.name)}" />`;
  return `<span class="logo-fallback">${esc((t.name || "?").trim().charAt(0))}</span>`;
}

function renderSwitchSettings() {
  const tb = $("#tb-tools");
  if (tb) {
    const enabledMap = state.extras?.tools || {};
    tb.innerHTML = (state.tools || []).length
      ? state.tools.map((t) => {
          const on = enabledMap[t.id] !== undefined ? enabledMap[t.id] : t.installed;
          return `<tr>
            <td>${toolLogo(t)}<b style="font-weight:600">${esc(t.name)}</b></td>
            <td><span class="pill ${t.installed ? "green" : "gray"}">${t.installed ? "已安装" : "未检测到"}</span></td>
            <td><code>${esc(t.path || "—")}</code></td>
            <td>${esc(t.detail || "")}</td>
            <td><input type="checkbox" class="tgl" data-tool-toggle="${esc(t.id)}" ${on ? "checked" : ""} /></td>
          </tr>`;
        }).join("")
      : `<tr><td colspan="5" class="empty">检测中…</td></tr>`;
  }
  const a = state.auto || {};
  const set = (id, v) => { const el = document.getElementById(id); if (el) el.value = v ?? ""; };
  const chk = (id, v) => { const el = document.getElementById(id); if (el) el.checked = !!v; };
  set("sw-checkin-hours", a.checkinHours);
  set("sw-rotate-min", a.rotateMinutes);
  chk("sw-auto-signin", a.autoCheckin);
  chk("sw-auto-travel", a.autoTravel);
  chk("sw-auto-rotate", a.autoRotate);
  chk("sw-launch", a.launchAtLogin);
  chk("sw-companion", a.companion);

  const cli = (state.tools || []).find((t) => t.id === "codebuddy_cli");
  const el = $("#cli-info");
  if (el) el.innerHTML = `
    <div><span>CLI 配置</span><code>${esc(cli?.path || "未检测到")}</code></div>
    <div><span>接入状态</span><b class="${cli?.installed ? "ok" : "bad"}">${cli?.installed ? "已接入" : "未接入"}</b></div>
    <div><span>switch 目录</span><code>${esc(state.switchSettings?.dir || "—")}</code></div>`;
}

function renderSessionGroups() {
  const el = $("#group-list");
  if (!el) return;
  const groups = state.groups || [];
  el.innerHTML = groups.length
    ? groups.map((g) => `<div class="comp-item">
        <div><b>${esc(g.name)}</b><div class="sub">${(g.members || []).length} 个会话 · ${fmt(g.createdAt)}</div></div>
        <button class="btn sm danger" data-del-group="${esc(g.id)}">删除</button>
      </div>`).join("")
    : `<div class="empty">暂无会话组。输入组名点「新建」；同步沿用 switch 的会话组模型。</div>`;
}

async function loadSwitch() {
  if (!hasTauri) return;
  try {
    state.sessions = await invoke("switch_sessions");
    state.tokens = await invoke("switch_token_stats", { days: Number($("#token-days")?.value || 30) });
    state.creditCache = await invoke("switch_usage_cache");
    state.switchSettings = await invoke("switch_settings");
    state.tools = await invoke("switch_tool_status");
    state.auto = await invoke("switch_get_auto");
    state.groups = await invoke("switch_session_groups");
    state.creditExpiry = await invoke("switch_credit_expiry");
    state.switchError = null;
  } catch (err) {
    state.switchError = String(err);
  }
  renderSessions(); renderTokens(); renderCreditStats();
  renderSwitchSettings(); renderSessionGroups();
}

/* ---------- 免费额度 ---------- */
function renderQuota() {
  const el = $("#quota-grid");
  if (!el) return;
  // 这里只放「能查到余额」的卡片。免费厂商的目录信息在下面独立的
  // 「免费模型目录」板块里（有模型数、限速、月度预算），别再往这儿塞
  // remaining=null 的空壳卡——以前那样做，整页全是「未开放接口」。
  const rows = (state.quota || []).slice();
  if (!rows.length) {
    el.innerHTML = `<div class="empty">没读到任何额度卡片。先确保「上游管理」里至少有一家、并且便携版目录能读到 <code>data/state.json</code>。</div>`;
    return;
  }
  el.innerHTML = rows
    .map((c) => {
      const ok = c.status === "已接入" || c.status === "可探测";
      const needKey = /需.*Key/.test(c.status);
      const noApi = c.status === "未开放接口";
      const pill = ok ? "green" : noApi ? "gray" : "orange";
      const hint = ok
        ? "点「抓取」刷新数字"
        : noApi
          ? "这家没开放额度接口，只能去它控制台看——卡片上的来源就是入口"
          : needKey
            ? "怎么解锁：去「上游管理」给这家填上 Key，回来再抓"
            : "怎么解锁：" + c.status;
      return `<div class="quota-card">
      <header><h3>${esc(c.name)}</h3><span class="pill gray">${esc(c.kind)}</span></header>
      <div class="num">${c.remaining != null ? fmtNum(c.remaining) : "—"}${c.total != null ? ` <span class="sub">/ ${fmtNum(c.total)}</span>` : ""}</div>
      <div class="sub">${esc(c.detail)}</div>
      <div class="sub" style="font-size:11px;color:var(--muted)">数据来自 ${esc(c.source)}</div>
      <div class="sub" style="margin-top:6px;${ok ? "" : noApi ? "color:var(--muted)" : "color:#c77700"}">${esc(hint)}</div>
      <div class="actions"><span class="pill ${pill}">${esc(c.status)}</span>${
        noApi ? "" : `<button class="btn sm" data-quota="${esc(c.id)}">抓取</button>`
      }</div>
    </div>`;
    })
    .join("");
}

async function loadQuota() {
  if (!hasTauri) return;
  try { state.quota = await invoke("quota_cards"); } catch { state.quota = []; }
  renderQuota();
}

/* ---------- 自动更新 ---------- */
function renderUpdate() {
  const el = $("#update-grid");
  if (!el) return;
  el.innerHTML = (state.updateTargets || []).map((t) => `<div class="quota-card">
    <header><h3>${esc(t.name)}</h3><span class="pill gray">${esc(t.kind)}</span></header>
    <div class="num">${esc(t.current)}</div>
    <div class="sub">${esc(t.repo || "本地构建")}</div>
    <div class="sub">${esc(t.note)}</div>
    <div class="actions">
      ${t.repo ? `<button class="btn sm" data-open-repo="${esc(t.repo)}">更新源</button>` : ""}
      <button class="btn sm primary" data-check-update="${esc(t.id)}">检查更新</button>
    </div>
    <div class="sub" id="upd-res-${esc(t.id)}"></div>
  </div>`).join("");
}

async function loadUpdate() {
  if (!hasTauri) return;
  try {
    state.updateTargets = await invoke("update_targets");
    const proxy = await invoke("update_get_proxy");
    if ($("#upd-proxy")) $("#upd-proxy").value = proxy || "";
  } catch { /* ignore */ }
  renderUpdate();
}

/* ---------- 便携版说明 ---------- */
async function loadPortableInfo() {
  if (!hasTauri) return;
  try {
    const p = await invoke("portable_info");
    const mask = (k) => (k ? k.slice(0, 6) + "…" + k.slice(-4) : "未设置");
    const el = $("#portable-info");
    if (el) el.innerHTML = `
      <div><span>便携版目录</span><code>${esc(p.root)}</code></div>
      <div><span>配置文件</span><code>${esc(p.configPath)}</code></div>
      <div><span>监听地址</span><code>${esc(p.listen)}</code></div>
      <div><span>API Key</span><code>${esc(mask(p.apiKey))}</code></div>
      <div><span>已授权账号</span><code>${p.authCount} 个</code></div>
      <div><span>版本</span><code>${esc(p.version)}</code></div>`;
  } catch (err) {
    const el = $("#portable-info");
    if (el) el.innerHTML = `<div><span>读取失败</span><code>${esc(err)}</code></div>`;
  }
}

/* ---------- 上游预设 ---------- */

/* 来自 FreeLLMAPI 目录的免费厂商（已剔除搜索/抓取/语音类，只留有对话接口的） */
const FREEL_API_CHAT = [
  ["ACLIDE", "https://aclide.com/v1"],
  ["apinex", "https://api.apinex.bond/v1"],
  ["BeatAPI", "https://api.beatapi.io/v1"],
  ["blockrun", "https://api.blockrun.ai/v1"],
  ["electronhub", "https://api.electronhub.ai/v1"],
  ["Free.ai", "https://api.free.ai"],
  ["gizmo", "https://api.gizmoplatforms.com/api/v1"],
  ["inferbase", "https://api.inferbase.ai/v1"],
  ["iteracompute", "https://api.iteracompute.com/v1"],
  ["Keenable", "https://api.keenable.ai/v1"],
  ["kenari", "https://kenari.id/v1"],
  ["kiosapi", "https://api.kiosapi.id/v1"],
  ["llmtr", "https://llmtr.com/v1"],
  ["modelscope", "https://api-inference.modelscope.cn/v1"],
  ["Moondream", "https://api.moondream.ai/v1"],
  ["onomeo", "https://onomeo.com/v1"],
  ["plugsky", "https://plugsky.com/v1"],
  ["pollinations", "https://gen.pollinations.ai/v1"],
  ["router9", "https://api.router9.com/v1"],
  ["Sail Research", "https://api.sailresearch.com/v1"],
  ["simplellm", "https://api.simplellm.eu/v1"],
  ["speka", "https://speka.me/v1"],
  ["typhoon", "https://api.opentyphoon.ai/v1"],
  ["zhipu", "https://open.bigmodel.cn/api/paas/v4"],
];

const UPSTREAM_PRESETS = [
  ["WorkBuddy CN", "https://www.workbuddy.cn/v2"],
  ["Trae CN", "https://api.trae.cn"],
  ["Trae Intl", "https://api.trae.ai"],
  ["Z.AI", "https://api.z.ai/api/coding/paas/v4"],
  ["ZCode", "https://api.z.ai/api/coding/paas/v4"],
  ["BigModel 智谱", "https://open.bigmodel.cn/api/paas/v4"],
  ["MiniMax", "https://api.minimaxi.com/v1"],
  ["豆包桥（本地）", "http://127.0.0.1:9090/v1"],
  ["FreeLLMAPI（本地）", "http://127.0.0.1:3001/v1"],
  ["火山方舟（豆包）", "https://ark.cn-beijing.volces.com/api/v3"],
  ["DSH", "http://127.0.0.1:8317/v1"],
  ["Qoder", "https://api.qoder.com/v1"],
  ["OpenCode Zen", "https://opencode.ai/zen/v1"],
  ["Kimi", "https://api.moonshot.cn/v1"],
  ["百川", "https://api.baichuan-ai.com/v1"],
  ["零一万物", "https://api.lingyiwanwu.com/v1"],
  ["讯飞星火", "https://spark-api-open.xf-yun.com/v1"],
  ["混元", "https://api.hunyuan.cloud.tencent.com/v1"],
  ["硅基流动", "https://api.siliconflow.cn/v1"],
  ["CommandCode", "https://api.commandcode.ai/v1"],
  ["MonkeyCode", "https://api.monkeycode.ai/v1"],
  ["华为云 CodeArts", "https://api.codearts.huaweicloud.com/v1"],
  ["OpenAI", "https://api.openai.com/v1"],
  ["Anthropic", "https://api.anthropic.com/v1"],
  ["Google Gemini", "https://generativelanguage.googleapis.com/v1beta/openai"],
  ["DeepSeek", "https://api.deepseek.com/v1"],
];

/* 各平台「去哪儿拿 Key」*/
const KEY_PAGES = {
  "Z.AI": "https://z.ai/manage-apikey/apikey-list",
  ZCode: "https://z.ai/manage-apikey/apikey-list",
  "BigModel 智谱": "https://open.bigmodel.cn/usercenter/apikeys",
  BigModel: "https://open.bigmodel.cn/usercenter/apikeys",
  MiniMax: "https://platform.minimaxi.com/user-center/basic-information/interface-key",
  "火山方舟（豆包）": "https://console.volcengine.com/ark/region:ark+cn-beijing/apiKey",
  Kimi: "https://platform.moonshot.cn/console/api-keys",
  "百川": "https://platform.baichuan-ai.com/console/apikey",
  "零一万物": "https://platform.lingyiwanwu.com/apikeys",
  "讯飞星火": "https://console.xfyun.cn/services/cbm",
  "混元": "https://console.cloud.tencent.com/hunyuan/api-key",
  "硅基流动": "https://cloud.siliconflow.cn/account/ak",
  CommandCode: "https://commandcode.ai",
  MonkeyCode: "https://monkeycode-ai.com",
  "华为云 CodeArts": "https://devcloud.cn-north-4.huaweicloud.com/codearts",
  OpenAI: "https://platform.openai.com/api-keys",
  Anthropic: "https://console.anthropic.com/settings/keys",
  "Google Gemini": "https://aistudio.google.com/app/apikey",
  DeepSeek: "https://platform.deepseek.com/api_keys",
  "Qoder": "https://qoder.com",
  "Trae CN": "https://www.trae.cn",
  "Trae Intl": "https://www.trae.ai",
  DSH: "https://github.com/deepseek-ai",
  "OpenCode Zen": "https://opencode.ai/auth",
  "WorkBuddy CN": "https://www.workbuddy.cn",
};

/** IDE 类客户端：不用 Key，直接读本机登录态 */
const IDE_PLATFORMS = /Trae|Qoder|CodeArts|DSH|CodeBuddy/i;


/** 消费级 App 与 API 平台不是一回事；另有本地桥接方案 */
const PLATFORM_NOTES = {
  "FreeLLMAPI（本地）":
    "<b>本机可选组件</b>：一个把 34 家免费模型（635 个端点）聚合成 <code>/v1</code> 的开源服务。装好并启动后，地址是 <code>http://127.0.0.1:3001/v1</code>，<b>Key 按它自己的配置填</b>（多数情况留空即可）。它自己会做限流与故障转移；没启动时这条不通，点「检测本机服务」确认。",
  "豆包桥（本地）":
    "<b>⚠️ 这是本机可选组件，不是所有电脑都有。</b>它靠一个跑在本机 9090 端口的小服务（阿晓豆包桥）把你的豆包账号包成 OpenAI 兼容接口，<b>不用 Key、不按 token 付费</b>。<br/><b>你本机没装/没启动的话，选了也用不了</b>——先点下面的「检测本机服务」，能连上再用；连不上就换「火山方舟（豆包）」走官方 API。别人拿到你的安装包，这条默认也是不通的，需要他自己有那个桥。",
  "火山方舟（豆包）":
    "<b>豆包 App、豆包工作、网页版都没有 API Key</b>——它们是产品端。要官方 API 只能走<b>火山引擎 → 火山方舟</b>：①实名认证 ②「API 密钥管理」建 Key（<code>ek-</code> 开头，只显示一次，默认有效期约 50 分钟，建时改成 24h）③「开通管理」开通要用的豆包模型 ④「在线推理」创建<b>推理接入点</b>拿 <code>ep-xxxxxxxx</code>。填进来时 <b>Key 填 ek-、模型填 ep-</b>。接入点按小时计费，用完记得释放。",
  Kimi: "要 API 得去<b>开放平台</b> platform.moonshot.cn（不是 kimi.com 网页版）。",
  "智谱": "在 open.bigmodel.cn 的「API Keys」里建，智谱清言 App 里没有。",
  MiniMax: "在 platform.minimaxi.com 的「接口密钥」里建，海螺 App 里没有。",
  "混元": "在腾讯云控制台 → 混元大模型 → API Key 里建，腾讯元宝里没有。",
  "讯飞星火": "在 console.xfyun.cn 创建应用，会给你 AppID / APIKey / APISecret 三件套，别只复制一个。",
  "华为云 CodeArts": "在华为云 CodeArts 控制台的「个人凭证」里建，IDE 客户端里没有。",
};

function renderPlatTip() {
  const el = $("#add-plat-tip");
  if (!el) return;
  const idx = Number($("#add-plat-select")?.value || 0);
  const p = PLATFORM_PRESETS[idx] || ["自定义", ""];
  const name = p[0];
  if (name === "自定义") {
    el.innerHTML = "自定义平台：把它的 OpenAI 兼容 Base URL 和 Key 填进来。多数厂商的 Base URL 形如 <code>https://api.xxx.com/v1</code>。";
    return;
  }
  if (IDE_PLATFORMS.test(name)) {
    el.innerHTML = `<b>${esc(name)} 是 IDE 客户端，不需要 Key</b>——它的登录态就在你本机配置文件里，去「设置 → CLI 检测」能直接读出来当账号用。<br/>如果你确实拿到了它的 API Key，也可以填在下面。`;
    return;
  }
  const url = KEY_PAGES[name];
  const note = PLATFORM_NOTES[name];
  if (note) {
    el.innerHTML =
      note + (url ? `<br/><span class="link" data-open-url="${esc(url)}">打开 ${esc(name)} 的 API Key 页面 →</span>` : "");
    return;
  }
  el.innerHTML = url
    ? `还没有 Key？<span class="link" data-open-url="${esc(url)}">去 ${esc(name)} 开放平台复制 →</span>（登录后在 API Key / 密钥管理页面创建）`
    : `去 ${esc(name)} 的开放平台创建一个 API Key 再粘进来。`;
}


/* ---------- 免费模型目录（数据来自 freellmapi catalog，编译期嵌入 exe） ---------- */
async function loadCatalog() {
  try {
    state.catalog = await invoke("catalog_get");
  } catch (e) {
    state.catalog = null;
    console.warn("[catalog]", e);
  }
  renderCatalog();
}

// 上下文窗口按 1024 进制显示：131072 就是 128K，别写成 131.1K
function ctxLabel(n) {
  const v = Number(n) || 0;
  if (!v) return "";
  if (v >= 1048576) return (v / 1048576).toFixed(v % 1048576 === 0 ? 0 : 1) + "M";
  if (v >= 1024) return Math.round(v / 1024) + "K";
  return String(v);
}

function catLimits(p) {
  const out = [];
  if (p.rpm) out.push(`${p.rpm} RPM`);
  if (p.rpd) out.push(`${p.rpd} RPD`);
  if (p.tpm) out.push(`${fmtNum(p.tpm)} TPM`);
  if (p.tpd) out.push(`${fmtNum(p.tpd)} TPD`);
  return out.join(" · ");
}

function catCardHtml(p) {
  const lim = catLimits(p);
  const ctx = p.context ? `上下文 ${ctxLabel(p.context)}` : "";
  const sub2 = [lim, ctx].filter(Boolean).join(" · ");
  const tag = p.official
    ? `<span class="pill green">官方</span>`
    : `<span class="pill gray">聚合站</span>`;
  const proto =
    p.protocol && p.protocol !== "openai"
      ? `<span class="pill blue">${esc(p.protocol)}</span>`
      : "";
  return `<div class="cat-card" data-card="${esc(p.id)}">
    <header><h3 title="${esc(p.baseUrl)}">${esc(p.name)}</h3><span style="display:flex;gap:4px">${tag}${proto}</span></header>
    <div class="cat-n">${p.enabled ?? p.models}<span class="sub"> 个可用</span></div>
    <div class="sub">${esc(sub2 || "目录没给限速数字")}</div>
    <div class="cat-budget">${esc(p.budget || "目录没给月度预算")}</div>
    <div class="actions">
      <button class="btn sm primary" data-cat-add="${esc(p.id)}">加为上游</button>
      <button class="btn sm" data-cat-models="${esc(p.id)}">看模型</button>
    </div>
  </div>`;
}

function renderCatalog() {
  const el = $("#cat-grid");
  if (!el) return;
  const d = state.catalog;
  if (!d || !Array.isArray(d.providers)) {
    el.innerHTML = `<div class="empty">没读到免费模型目录</div>`;
    return;
  }
  const mode = state.catFilter || "all";
  const q = (state.catQ || "").toLowerCase();
  let rows = d.providers.slice();
  if (mode === "official") rows = rows.filter((p) => p.official);
  if (mode === "pool") rows = rows.filter((p) => !p.official);
  if (q) rows = rows.filter((p) => `${p.name} ${p.id}`.toLowerCase().includes(q));
  rows.sort((a, b) => (b.enabled ?? b.models) - (a.enabled ?? a.models) || a.name.localeCompare(b.name));

  const meta = $("#cat-meta");
  if (meta) {
    const tm = rows.reduce((s, p) => s + (p.enabled ?? p.models ?? 0), 0);
    const all = d.providers.length;
    meta.textContent =
      rows.length === all ? `${all} 家 / ${tm} 个模型` : `${rows.length}/${all} 家 / ${tm} 个模型`;
  }

  el.innerHTML = rows.length
    ? rows.map(catCardHtml).join("")
    : `<div class="empty">没有匹配的厂商，换个词试试</div>`;
}

$$("#cat-seg .seg-item").forEach((b) => {
  b.addEventListener("click", () => {
    $$("#cat-seg .seg-item").forEach((x) => x.classList.remove("on"));
    b.classList.add("on");
    state.catFilter = b.dataset.cat;
    renderCatalog();
  });
});
$("#cat-q")?.addEventListener("input", (e) => {
  state.catQ = e.target.value.trim();
  renderCatalog();
});

async function toggleCatModels(card, platform) {
  const exist = card.querySelector(".cat-models");
  if (exist) {
    exist.remove();
    card.classList.remove("on");
    return;
  }
  card.classList.add("on");
  const box = document.createElement("div");
  box.className = "cat-models";
  box.innerHTML = `<div class="sub">加载中…</div>`;
  card.appendChild(box);
  try {
    const r = await invoke("catalog_models", { platform });
    const rows = r.models || [];
    box.innerHTML = rows.length
      ? rows
          .map((m) => {
            const tags = [m.vision ? "视觉" : "", m.tools ? "工具" : "", m.context ? ctxLabel(m.context) : ""]
              .filter(Boolean)
              .join(" ");
            return `<div class="cm-row"><span class="cm-name" title="${esc(m.id)}">${esc(m.name || m.id)}</span><span class="cm-tags">${esc(tags || "文本")}</span></div>`;
          })
          .join("")
      : `<div class="sub">这家没有模型明细</div>`;
  } catch (err) {
    box.innerHTML = `<div class="sub">读不到：${esc(String(err))}</div>`;
  }
}

document.addEventListener("click", async (e) => {
  const add = e.target.closest("[data-cat-add]");
  if (add) {
    const platform = add.dataset.catAdd;
    add.disabled = true;
    try {
      const r = await invoke("catalog_add_upstream", { platform, apiKey: null });
      toast(
        r.created
          ? `已加为上游：${r.name} —— 去「上游管理」补上 Key 就能用`
          : `已更新上游：${r.name}`
      );
      state.upstreams = await invoke("list_upstreams");
      renderUpstreams();
      renderTestUpstreams();
    } catch (err) {
      toast("加不上：" + err);
    } finally {
      add.disabled = false;
    }
    return;
  }
  const look = e.target.closest("[data-cat-models]");
  if (look) {
    const card = look.closest(".cat-card");
    if (card) await toggleCatModels(card, look.dataset.catModels);
  }
});

/* 更多免费厂商（折叠区） */
function renderFreePresets() {
  const el = $("#free-presets");
  if (!el) return;
  el.innerHTML = FREEL_API_CHAT.map(
    ([n, u]) => `<button class="btn sm preset-chip" data-preset="${esc(n)}" data-url="${esc(u)}" title="点击添加 ${esc(n)}（${esc(u)}）">${upLogo(n)}<span>${esc(n)}</span></button>`,
  ).join("");
  const cnt = $("#free-presets-count");
  if (cnt) cnt.textContent = `${FREEL_API_CHAT.length} 家 · 来自 FreeLLMAPI 目录`;
}

function renderUpstreamPresets() {
  const el = $("#upstream-presets");
  if (!el) return;
  el.innerHTML = UPSTREAM_PRESETS.map(
    ([n, u]) =>
      `<button class="btn sm preset-chip" data-preset="${esc(n)}" data-url="${esc(u)}" title="点击把「${esc(n)}」加成上游">${upLogo(n)}<span>${esc(n)}</span></button>`,
  ).join("");
}

/* ---------- 能力覆盖 ---------- */
const COVERAGE = [
  ["多账号池轮转", "panel 内核", "kernel"],
  ["Token 续期 / 保活", "签到中心（原生）", "done"],
  ["OpenAI 兼容透传", "panel 内核", "kernel"],
  ["熔断 / 冷却", "panel 内核", "kernel"],
  ["会话粘性", "panel 内核", "kernel"],
  ["自动签到 / 活跃保活", "签到中心（原生）", "done"],
  ["工具调用 / 流式", "panel 内核", "kernel"],
  ["双站 / 多区域路由", "panel 内核（global 域）", "kernel"],
  ["三协议（Chat/Responses/Anthropic）", "协议与路由 + 网关", "done"],
  ["设备指纹隔离", "网关按账号注入 device-id", "done"],
  ["成本账本", "成本账本", "done"],
  ["读本地客户端登录态", "账号管理", "done"],
  ["多桌面端聚合 + 优先级队列 + 429 降级", "panel 内核（pool）", "kernel"],
  ["成长任务自动完成 + Web 面板", "任务中心 + panel", "done"],
  ["移动端常驻", "扩展能力（局域网地址）", "done"],
  ["多上游三合一（WB+Trae+ZCode）", "上游预设（16 家）", "done"],
  ["配额感知切号 + Key/Token 混挂", "panel 内核（池参数）", "kernel"],
  ["控制面（批量纳管/密钥分发/白名单）", "网关强制执行 Key/模型白名单", "done"],
  ["极简 CLI 会话复用", "扩展能力（检测 + 导入）", "done"],
  ["CodeBuddy CN OAuth2 + 手动 Token", "添加账号", "done"],
  ["Docker Web 控制台", "桌面内嵌 panel，不需要 Docker", "na"],
  ["上下文压缩", "网关压缩（真实生效）", "done"],
  ["原生 Claude Code / Codex / DSH", "客户端配置导出（写 config）", "done"],
  ["429 退避", "panel 内核", "kernel"],
  ["复用桌面端登录态 + DSH 插件", "账号管理", "done"],
  ["全家桶 7+ 上游", "上游预设（16 家）", "done"],
  ["全家桶 + 自定义 OpenAI", "上游管理", "done"],
  ["24 家官方 API + MCP", "目录 + MCP 脚本 + CF Worker", "done"],
  ["ZCode off-peak 异步队列", "定时器 + 队列（zcode_tick）", "done"],
  ["Go 零依赖 + 热更新", "wb2go 内核属性，本应用不适用", "na"],
  ["Python + WebUI + 浏览器登录", "添加账号", "done"],
  ["panel 快照", "融合组件", "done"],
  ["批量签到", "签到中心", "done"],
  ["用量分析仪表盘", "Token 统计", "done"],
  ["WB+Trae+Qoder 聚合 + 托盘", "聚合 + Tauri tray + 角色转换", "done"],
  ["多上游抽象 + 可扩展", "上游管理", "done"],
  ["免费额度聚合", "免费额度", "done"],
  ["Deno+CF 10 上游 + 思考档位", "CF Worker 代码已提供（需 wrangler 部署）", "done"],
];
const COVERAGE_LABEL = {
  done: ["已实现", "green"],
  kernel: ["内核已实现", "blue"],
  partial: ["控制台部分", "orange"],
  na: ["不适用", "gray"],
  ext: ["需外部设施", "gray"],
};

function renderCoverage() {
  const tb = $("#tb-coverage");
  if (!tb) return;
  tb.innerHTML = COVERAGE.map(([name, src, st]) => {
    const [label, cls] = COVERAGE_LABEL[st];
    return `<tr><td>${esc(name)}</td><td>${esc(src)}</td><td><span class="pill ${cls}">${label}</span></td></tr>`;
  }).join("");
}

/* ---------- 扩展能力 ---------- */
const OFFICIAL = [
  ["OpenAI", "https://api.openai.com/v1"],
  ["Anthropic", "https://api.anthropic.com/v1"],
  ["Google Gemini", "https://generativelanguage.googleapis.com/v1beta"],
  ["DeepSeek", "https://api.deepseek.com/v1"],
  ["Qwen", "https://dashscope.aliyuncs.com/compatible-mode/v1"],
  ["Kimi", "https://api.moonshot.cn/v1"],
  ["Zhipu", "https://open.bigmodel.cn/api/paas/v4"],
  ["MiniMax", "https://api.minimaxi.com/v1"],
,
  ["百川", "https://api.baichuan-ai.com/v1"],
  ["零一万物", "https://api.lingyiwanwu.com/v1"],
  ["讯飞星火", "https://spark-api-open.xf-yun.com/v1"],
  ["腾讯混元", "https://api.hunyuan.cloud.tencent.com/v1"],
  ["硅基流动", "https://api.siliconflow.cn/v1"],
  ["OpenRouter", "https://openrouter.ai/api/v1"],
  ["Groq", "https://api.groq.com/openai/v1"],
  ["Mistral", "https://api.mistral.ai/v1"],
  ["xAI", "https://api.x.ai/v1"],
  ["Together", "https://api.together.xyz/v1"],
  ["Fireworks", "https://api.fireworks.ai/inference/v1"],
  ["Perplexity", "https://api.perplexity.ai"],
  ["Cohere", "https://api.cohere.ai/v1"],
  ["Replicate", "https://api.replicate.com/v1"],
  ["Ollama", "http://127.0.0.1:11434/v1"],
];
const THINKING = ["WorkBuddy", "Trae CN", "ZCode", "Qoder", "MiniMax", "豆包", "CodeArts", "DSH", "OpenAI", "Anthropic"];
const THINKING_LEVELS = ["default", "minimal", "low", "medium", "high", "xhigh", "max"];

function renderExtras() {
  const e = state.extras || {};
  const m = state.mobile || {};
  const mi = $("#mobile-info");
  if (mi) mi.innerHTML = `
    <div><span>局域网 IP</span><code>${esc(m.lanIp || "—")}</code></div>
    <div><span>面板地址</span><code>${esc(m.panelUrl || "—")}</code></div>
    <div><span>API Base</span><code>${esc(m.apiBase || "—")}</code></div>
    <div><span>监听</span><code>${esc(m.listen || "—")} ${m.lanReady ? "（已开放局域网）" : "（仅本机）"}</code></div>`;
  const cp = e.control || {};
  if ($("#cp-keys")) $("#cp-keys").value = (cp.keys || []).join("\n");
  if ($("#cp-whitelist")) $("#cp-whitelist").value = (cp.whitelist || []).join("\n");
  const cli = (state.tools || []).find((t) => t.id === "codebuddy_cli");
  const ci = $("#cli-reuse-info");
  if (ci) ci.innerHTML = `<div><span>CLI 配置</span><code>${esc(cli?.path || "未检测到")}</code></div><div><span>状态</span><b class="${cli?.installed ? "ok" : "bad"}">${cli?.installed ? "可复用" : "未接入"}</b></div>`;
  const c = e.compress || {};
  if ($("#compress-on")) $("#compress-on").checked = !!c.enabled;
  if ($("#compress-max")) $("#compress-max").value = c.maxChars || 12000;
  const oi = $("#official-info");
  if (oi) oi.innerHTML = `<div><span>官方 API 目录</span><code>${OFFICIAL.length} 家</code></div><div><span>已收录</span><code>${OFFICIAL.slice(0, 8).map((x) => x[0]).join(" / ")} …</code></div>`;
  const ms = $("#mcp-snippet");
  if (ms) ms.textContent = JSON.stringify({ mcpServers: { axiao: { command: "axiao-gateway", args: ["mcp"] } } }, null, 2);
  const z = e.zcode || {};
  if ($("#zcode-window")) $("#zcode-window").value = z.window || "00:00-08:00";
  const zi = $("#zcode-info");
  if (zi) zi.innerHTML = `<div><span>队列长度</span><code>${(z.queue || []).length} 个任务</code></div><div><span>窗口</span><code>${esc(z.window || "")}</code></div><div><span>上次执行</span><code>${esc(z.last_run || "—")}</code></div>`;
  const t = e.thinking || {};
  if ($("#thinking-level")) $("#thinking-level").value = t.level || "default";
  const tl = $("#thinking-list");
  if (tl) {
    const per = t.per || {};
    // 用真实上游名做 key —— 后端按 upstream.name 取档位，key 必须对得上
    const names = (state.upstreams || []).length
      ? (state.upstreams || []).map((u) => u.name)
      : THINKING;
    const opts = (cur) => THINKING_LEVELS.map((o) => `<option value="${o}"${cur === o ? " selected" : ""}>${o}</option>`).join("");
    tl.innerHTML = names.map((n) => {
      const cur = per[n] || "default";
      return `<div class="comp-item"><div><b>${esc(n)}</b><div class="sub">可配置权重 / 思考档位${per[n] ? "" : "（跟随默认）"}</div></div>
        <select class="btn sm" data-thinking="${esc(n)}">${opts(cur)}</select></div>`;
    }).join("");
  }
}

async function loadExtras() {
  if (!hasTauri) return;
  try {
    state.extras = await invoke("extras_get");
    state.mobile = await invoke("mobile_info");
  } catch { state.extras = {}; }
  renderExtras();
}

/* ---------- 接入信息 ---------- */
function renderAccessInfo() {
  const a = state.access || {};
  const mask = (k) => (k ? k.slice(0, 6) + "…" + k.slice(-4) : "未设置");
  const el = $("#access-info");
  if (!el) return;
  const base = a.base || "http://127.0.0.1:7863/v1";
  el.innerHTML = `
    <div><span>Base URL</span><code>${esc(base)}</code></div>
    <div><span>API Key</span><code>${esc(mask(a.key))}</code></div>
    <div><span>模型列表</span><code>${esc(base.replace(/\/v1$/, ""))}/v1/models</code></div>
    <div><span>密钥位置</span><code>config.json → api_key（网关配置 → 服务 → API 密钥）</code></div>
    <div><span>环境变量覆盖</span><code>WB2A_API_KEY</code></div>`;
}

async function loadAccessInfo() {
  if (!hasTauri) { renderAccessInfo(); return; }
  try {
    const p = await invoke("portable_info");
    let cfg = {};
    try { cfg = (await invoke("local_config_get")).config || {}; } catch { /* ignore */ }
    const listen = cfg.listen || p.listen || "127.0.0.1:7863";
    const host = listen.replace("0.0.0.0", "127.0.0.1");
    state.access = { base: `http://${host}/v1`, key: cfg.api_key || p.apiKey || "", listen };
  } catch {
    state.access = {};
  }
  renderAccessInfo();
}

/* ---------- 加载 ---------- */
async function loadPanel() {
  try {
    const pkg = await invoke("panel_packages");
    state.panel.packages = pkg.accounts || [];
    if (!state.panel.uid) state.panel.uid = state.panel.packages[0]?.uid || null;
    if (state.panel.uid) {
      const t = await invoke("panel_tasks", { uid: state.panel.uid });
      state.panel.tasks = t.tasks || [];
    }
    const m = await invoke("panel_models");
    state.panel.models = m.models || [];
    state.panelError = null;
  } catch (e) {
    state.panelError = String(e);
  }
}

async function loadAll() {
  if (!hasTauri) { toast("请用 Tauri 运行（npm run dev）"); return; }
  // 逐块容错：任何一块失败都不该把整页拖空（原来一个 await 抛错，后面全不执行）
  const safe = async (label, fn) => {
    try {
      await fn();
    } catch (e) {
      console.warn("[loadAll]", label, e);
      state.loadErrors = state.loadErrors || [];
      state.loadErrors.push(`${label}: ${e}`);
    }
  };
  state.loadErrors = [];

  await safe("账号", async () => { state.bundle = await invoke("get_accounts"); });
  await safe("上游", async () => { state.upstreams = await invoke("list_upstreams"); });
  await safe("上游健康度", async () => { state.health = await invoke("upstream_health"); });
  await safe("熔断状态", loadBreaker);
  await safe("账本", async () => { state.ledger = await invoke("ledger", { limit: 200 }); });
  await safe("日志", async () => { state.hooks = await invoke("list_hooks", { limit: 200 }); });
  await safe("服务日志", loadGatewayLogs);
  await safe("免费目录", loadCatalog);
  await safe("响应缓存", renderCache);
  await safe("访问 Key", loadAccess);
  await safe("选路策略", loadRouting);
  await safe("版本号", loadVersion);
  await safe("组件", async () => { state.components = await invoke("component_status"); });
  await safe("面板", loadPanel);

  try { renderStats(); renderAccounts(); renderSwitchList(); renderLedger(); renderCredits(); renderTasks(); renderModels(); } catch (e) { console.warn(e); }
  try {
    renderUpstreams(); renderRequests(); renderHooks(); renderComponents(); renderSignin();
    renderGrowth(); renderTestUpstreams(); renderCreate(); renderMediaServer(); renderChangelog(); renderGuide(); renderDashboardCharts(); renderTokenChart();
  } catch (e) { console.warn(e); }

  await safe("配置", loadConfig);
  await safe("会话", loadSwitch);
  await safe("免费额度", loadQuota);
  await safe("更新", loadUpdate);
  await safe("扩展", loadExtras);
  await safe("便携版", loadPortableInfo);
  await safe("接入", loadAccessInfo);
  await safe("模型映射", loadModelMap);

  try { renderPageCharts(); } catch (e) { console.warn("[charts]", e); }
  if (state.loadErrors.length) {
    console.warn("部分数据读取失败：", state.loadErrors);
  }
}

/* ---------- 事件 ---------- */
document.addEventListener("click", async (e) => {
  const sw = e.target.closest("[data-switch]");
  if (sw) { if (!hasTauri) return toast("请用 Tauri 运行"); try { const r = await invoke("switch_account", { id: sw.dataset.switch }); toast("已写回授权文件" + (r.backup ? "（已备份）" : "")); await invoke("append_hook", { kind: "switch", detail: "切换到 " + sw.dataset.switch }); loadAll(); } catch (err) { toast("切换失败：" + err); } return; }
  const tg = e.target.closest("[data-toggle]");
  if (tg) { const a = state.bundle.accounts.find((x) => x.id === tg.dataset.toggle); if (a) { await invoke("save_account", { account: { ...a, enabled: !a.enabled } }); loadAll(); } return; }
  const del = e.target.closest("[data-del]");
  if (del) {
    const acc = (state.bundle.accounts || []).find((x) => x.id === del.dataset.del);
    const isKey = acc?.client === "apikey";
    if (!confirm(isKey ? "这是 API Key 账号，删除会一并移除对应上游，继续？" : "确认删除该账号？")) return;
    try {
      await invoke("delete_account", { id: del.dataset.del });
      if (isKey && acc?.accountId) await invoke("delete_upstream", { id: acc.accountId }).catch(() => {});
      loadAll();
    } catch (err) { toast(String(err)); }
    return;
  }
  const l = e.target.closest("[data-launch]");
  if (l) { if (!confirm("将启动本地组件，继续？")) return; try { await invoke("launch_component", { id: l.dataset.launch, confirm: true }); toast("已启动"); } catch (err) { toast("启动失败：" + err); } }
});

$("#btn-refresh").addEventListener("click", loadAll);
$("#btn-sessions-refresh")?.addEventListener("click", loadSwitch);
$("#btn-tokens-refresh")?.addEventListener("click", loadSwitch);
$("#token-days")?.addEventListener("change", loadSwitch);

/* switch 设置 / 会话组 */
$("#btn-save-auto")?.addEventListener("click", async () => {
  const settings = {
    autoCheckin: !!$("#sw-auto-signin")?.checked,
    autoTravel: !!$("#sw-auto-travel")?.checked,
    autoRotate: !!$("#sw-auto-rotate")?.checked,
    checkinHours: $("#sw-checkin-hours")?.value || "",
    rotateMinutes: Number($("#sw-rotate-min")?.value) || 30,
    excludeAccounts: state.auto?.excludeAccounts || [],
    launchAtLogin: !!$("#sw-launch")?.checked,
    companion: !!$("#sw-companion")?.checked,
  };
  try {
    await invoke("switch_save_auto", { settings });
    $("#sw-msg").textContent = "已保存";
    $("#sw-msg").className = "msg ok";
    await invoke("append_hook", { kind: "settings", detail: "保存 switch 自动设置" });
  } catch (err) {
    $("#sw-msg").textContent = "保存失败：" + err;
    $("#sw-msg").className = "msg bad";
  }
});
$("#btn-create-group")?.addEventListener("click", async () => {
  const name = $("#grp-name")?.value.trim();
  if (!name) return toast("请输入组名");
  await invoke("switch_create_group", { name, members: [] });
  $("#grp-name").value = "";
  await loadSwitch();
  toast("已新建会话组");
});
$("#btn-cli-check")?.addEventListener("click", () => toast("CLI 轮换检查需 CodeBuddy CLI 已接入"));
$("#btn-switch-signall")?.addEventListener("click", () => $("#btn-native-checkin").click());
document.addEventListener("click", async (e) => {
  const d = e.target.closest("[data-del-group]");
  if (d) {
    if (!confirm("删除该会话组？")) return;
    await invoke("switch_delete_group", { id: d.dataset.delGroup });
    await loadSwitch();
  }
});

/* 免费额度 / 自动更新 */
$("#btn-quota-refresh")?.addEventListener("click", async () => { await loadQuota(); toast("已刷新额度"); });
$("#btn-save-proxy")?.addEventListener("click", async () => {
  try {
    await invoke("update_save_proxy", { proxy: $("#upd-proxy")?.value.trim() || "" });
    $("#upd-msg").textContent = "已保存";
    $("#upd-msg").className = "msg ok";
  } catch (err) {
    $("#upd-msg").textContent = "保存失败：" + err;
    $("#upd-msg").className = "msg bad";
  }
});
document.addEventListener("click", async (e) => {
  const q = e.target.closest("[data-quota]");
  if (q) {
    const id = q.dataset.quota;
    const log = $("#quota-log");
    if (log) log.textContent = `抓取 ${id} …`;
    try {
      const r = await invoke("quota_refresh", { id });
      if (log) log.textContent = `${id} 结果：\n` + JSON.stringify(r, null, 2);
      await loadQuota();
    } catch (err) {
      if (log) log.textContent = `${id} 失败：${err}`;
    }
    return;
  }
  const c = e.target.closest("[data-check-update]");
  if (c) {
    const id = c.dataset.checkUpdate;
    const box = document.getElementById("upd-res-" + id);
    if (box) box.textContent = "检查中…";
    try {
      const r = await invoke("update_check", { id });
      if (box) {
        const when = r.publishedAt ? String(r.publishedAt).slice(0, 10) : "";
        box.textContent = r.noRelease
          ? `${r.name || "无 Release"} · 最近推送 ${when}`
          : `最新 ${r.latest || "?"}${when ? " · " + when : ""}`;
        box.className = r.noRelease ? "msg" : "msg ok";
      }
    } catch (err) {
      if (box) box.textContent = "失败：" + err;
    }
    return;
  }
  const o = e.target.closest("[data-open-repo]");
  if (o) {
    try { await invoke("update_open", { url: "https://github.com/" + o.dataset.openRepo }); }
    catch (err) { toast(String(err)); }
  }
});

/* 上游预设一键添加 */
document.addEventListener("click", async (e) => {
  const p = e.target.closest("[data-preset]");
  if (!p) return;
  const name = p.dataset.preset;
  // 同一平台加多条（多 Key）时必须生成不同 id，否则会覆盖上一条
  const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "") || "custom";
  const taken = new Set((state.upstreams || []).map((u) => u.id));
  let id = slug, n = 2;
  while (taken.has(id)) id = `${slug}_${n++}`;
  try {
    await invoke("save_upstream", {
      upstream: { id, name, baseUrl: p.dataset.url, apiKey: null, protocol: "openai", enabled: true, weight: 1 },
    });
    toast("已添加 " + name);
    loadAll();
  } catch (err) {
    toast("添加失败：" + err);
  }
});

/* 支持工具开关（全部平台） */
document.addEventListener("change", async (e) => {
  const t = e.target.closest("[data-tool-toggle]");
  if (!t) return;
  const id = t.dataset.toolToggle;
  const tools = { ...(state.extras?.tools || {}), [id]: t.checked };
  try {
    await invoke("extras_save", { key: "tools", value: tools });
    state.extras = { ...(state.extras || {}), tools };
    toast("已保存");
  } catch (err) {
    toast("保存失败：" + err);
  }
});

/* 接入信息：复制 / 重置密钥 */
$("#btn-copy-base")?.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(state.access?.base || "http://127.0.0.1:7863/v1");
    toast("Base URL 已复制");
  } catch { toast("复制失败"); }
});
$("#btn-copy-key")?.addEventListener("click", async () => {
  const k = state.access?.key;
  if (!k) return toast("未读取到密钥");
  try { await navigator.clipboard.writeText(k); toast("密钥已复制"); } catch { toast("复制失败"); }
});
$("#btn-rotate-key")?.addEventListener("click", async () => {
  if (!confirm("重置网关密钥？已填旧 key 的客户端需要同步修改。")) return;
  try {
    const r = await invoke("local_config_get");
    const cfg = r.config || {};
    const arr = new Uint8Array(16);
    crypto.getRandomValues(arr);
    cfg.api_key = "sk-" + [...arr].map((b) => b.toString(16).padStart(2, "0")).join("");
    await invoke("local_config_save", { config: cfg });
    try { await invoke("panel_save_config", { config: cfg }); } catch { /* panel 未启动 */ }
    toast("已重置，新密钥写入 config.json");
    await loadAccessInfo();
  } catch (err) {
    toast("重置失败：" + err);
  }
});

/* 扩展能力 */
$("#btn-lan-open")?.addEventListener("click", async () => {
  try {
    const r = await invoke("local_config_get");
    const cfg = r.config || {};
    cfg.listen = "0.0.0.0:7863";
    await invoke("local_config_save", { config: cfg });
    toast("已改为 0.0.0.0:7863，重启网关后生效");
    await loadExtras();
  } catch (err) { toast("失败：" + err); }
});
$("#btn-cp-save")?.addEventListener("click", async () => {
  const keys = ($("#cp-keys")?.value || "").split("\n").map((s) => s.trim()).filter(Boolean);
  const whitelist = ($("#cp-whitelist")?.value || "").split("\n").map((s) => s.trim()).filter(Boolean);
  await invoke("extras_save", { key: "control", value: { keys, whitelist } });
  toast("控制面已保存");
});
$("#btn-cli-reuse")?.addEventListener("click", async () => {
  try {
    const d = await invoke("cli_import");
    const id = "cli_" + Date.now();
    await invoke("save_account", {
      account: {
        id,
        client: "codebuddy",
        displayName: "CodeBuddy CLI",
        accountId: id,
        accessToken: d.token,
        refreshToken: null,
        expiresAt: null,
        credits: 0,
        creditUsage: 0,
        creditExpiry: null,
        enabled: true,
        lastUsedAt: null,
        region: "cn",
        source: "cli",
      },
    });
    toast("已复用 CLI 会话");
    loadAll();
  } catch (err) {
    toast("复用失败：" + err);
  }
});
document.addEventListener("click", async (e) => {
  const p = e.target.closest("[data-open-path]");
  if (p) return toast("文件位于项目目录：" + p.dataset.openPath);
  const ex = e.target.closest("[data-export-client]");
  if (!ex) return;
  const client = ex.dataset.exportClient;
  const base = state.access?.base || "http://127.0.0.1:7863/v1";
  const key = state.access?.key || "";
  try {
    const r = await invoke("export_client", { client, baseUrl: base, apiKey: key });
    const el = $("#export-msg");
    if (el) {
      el.textContent = r.path ? `已写入：${r.path}` : `通用配置：${JSON.stringify(r.snippet || {})}`;
      el.className = "msg ok";
    }
    toast("已导出 " + client);
  } catch (err) {
    const el = $("#export-msg");
    if (el) {
      el.textContent = "导出失败：" + err;
      el.className = "msg bad";
    }
  }
});
$("#btn-compress-save")?.addEventListener("click", async () => {
  await invoke("extras_save", {
    key: "compress",
    value: { enabled: !!$("#compress-on")?.checked, maxChars: Number($("#compress-max")?.value) || 12000 },
  });
  toast("压缩设置已保存");
});
$("#btn-add-official")?.addEventListener("click", async () => {
  const six = OFFICIAL.slice(0, 6);
  for (const [name, url] of six) {
    await invoke("save_upstream", {
      upstream: {
        id: name.toLowerCase().replace(/[^a-z0-9]+/g, "_"),
        name, baseUrl: url, apiKey: null, protocol: "openai", enabled: true, weight: 1,
      },
    });
  }
  toast("已添加 " + six.length + " 家官方 API");
  loadAll();
});
$("#btn-zcode-enqueue")?.addEventListener("click", async () => {
  const z = state.extras?.zcode || { window: "00:00-08:00", queue: [] };
  const queue = [...(z.queue || []), { ts: new Date().toISOString(), model: "zcode-offpeak" }];
  await invoke("extras_save", {
    key: "zcode",
    value: { window: $("#zcode-window")?.value || "00:00-08:00", queue },
  });
  await loadExtras();
  toast("已加入 off-peak 队列");
});
$("#btn-thinking-save")?.addEventListener("click", async () => {
  const per = {};
  $$("[data-thinking]").forEach((s) => {
    if (s.value && s.value !== "default") per[s.dataset.thinking] = s.value;
  });
  const level = $("#thinking-level")?.value || "default";
  await invoke("extras_save", { key: "thinking", value: { level, per } });
  const n = Object.keys(per).length;
  toast(`思考档位已保存：默认 ${level}${n ? `，${n} 个上游单独设置` : ""}`);
  await loadExtras();
});

$("#btn-disable-all").addEventListener("click", async () => { if (!confirm("确认禁用全部账号？")) return; for (const a of state.bundle.accounts) if (a.enabled) await invoke("save_account", { account: { ...a, enabled: false } }); loadAll(); });

/* 原生签到（不启动 exe） */
function signUid() { return $("#sign-uid")?.value || state.bundle.accounts?.[0]?.accountId; }
function signLog(v) { const el = $("#sign-log"); el.textContent = typeof v === "string" ? v : JSON.stringify(v, null, 2); }
async function nativeRun(cmd, label) {
  const uid = signUid();
  if (!uid) return toast("没有可用账号");
  signLog(label + " 执行中…");
  try {
    const r = await invoke(cmd, { uid });
    signLog(label + " 完成\n" + JSON.stringify(r, null, 2));
    await invoke("append_hook", { kind: "signin", detail: label + " " + uid });
    loadAll();
  } catch (err) { signLog(label + " 失败：" + err); }
}
$("#btn-sign-all").addEventListener("click", () => $("#btn-native-checkin").click());
$("#btn-native-checkin").addEventListener("click", () => nativeRun("native_checkin", "立即签到"));
$("#btn-native-growth").addEventListener("click", () => nativeRun("native_growth", "一键成长任务"));
$("#btn-native-keepalive").addEventListener("click", () => nativeRun("native_keepalive", "活跃保活"));
$("#btn-native-refresh").addEventListener("click", () => nativeRun("native_refresh", "刷新 Token"));
$("#btn-native-status").addEventListener("click", () => nativeRun("native_status", "查询状态"));
$("#btn-batch-checkin")?.addEventListener("click", async () => {
  if (!confirm("对全部启用账号执行签到？")) return;
  const accs = (state.bundle.accounts || []).filter((a) => a.enabled);
  const log = $("#sign-log");
  if (log) log.textContent = `批量签到 ${accs.length} 个账号…`;
  const lines = [];
  for (const a of accs) {
    const name = a.displayName || a.accountId || a.id;
    try {
      const r = await invoke("native_checkin", { uid: a.accountId || a.id });
      const msg = r?.claim?.msg || r?.status?.msg || "已执行";
      lines.push(`${name}: ${msg}`);
    } catch (err) {
      lines.push(`${name}: 失败 ${err}`);
    }
    if (log) log.textContent = lines.join("\n");
  }
  await invoke("append_hook", { kind: "signin", detail: `批量签到 ${accs.length} 个账号` });
  toast("批量签到完成");
});

/* 任务：切换账号 + 一键完成 */
$("#task-uid").addEventListener("change", async (e) => {
  state.panel.uid = e.target.value;
  try { const t = await invoke("panel_tasks", { uid: state.panel.uid }); state.panel.tasks = t.tasks || []; state.panelError = null; renderTasks(); }
  catch (err) { state.panelError = String(err); renderTasks(); }
});
$("#btn-task-all").addEventListener("click", async () => {
  if (!state.panel.uid) return toast("没有可执行账号");
  if (!confirm("将对该账号执行「一键完成可自动任务」，继续？")) return;
  const btn = $("#btn-task-all");
  btn.disabled = true;
  btn.textContent = "执行中…（限流会自动重试）";
  toast("执行中，最多等 30 秒…");
  try {
    await invoke("panel_task_auto_all", { uid: state.panel.uid });
    await invoke("append_hook", { kind: "task", detail: "一键完成 " + state.panel.uid });
    const t = await invoke("panel_tasks", { uid: state.panel.uid });
    state.panel.tasks = t.tasks || [];
    renderTasks();
    toast("已执行，任务状态已刷新");
  } catch (err) {
    const msg = String(err);
    if (msg.includes("RATE_LIMITED")) {
      // 限流说人话，不要把上游 JSON 甩给用户
      toast("被上游限流了：等 5~10 分钟再点。这不是本应用的限制");
    } else {
      toast("执行失败：" + msg.slice(0, 160));
    }
  }
  btn.disabled = false;
  btn.textContent = "一键完成可自动任务";
});

/* 积分任务：切换账号 / 重新查询 / 全部接受 / 一键领取 */
$("#growth-uid")?.addEventListener("change", (e) => loadGrowth(e.target.value));
$("#btn-growth-reload")?.addEventListener("click", () => loadGrowth());
$("#btn-growth-accept")?.addEventListener("click", async () => {
  const codes = (state.growth.tasks || [])
    .filter((t) => (t.accept_status || "") === "not_accepted")
    .map((t) => t.task_code);
  if (!codes.length) return toast("没有待接受的任务");
  try {
    const r = await invoke("growth_accept", { uid: state.growth.uid, codes });
    toast(`已接受 ${r.count} 项`);
    await loadGrowth();
  } catch (e) {
    toast("接受失败：" + e);
  }
});
$("#btn-growth-claim")?.addEventListener("click", async () => {
  const targets = (state.growth.tasks || []).filter(growthClaimable);
  if (!targets.length) return toast("没有可领取的任务");
  if (!confirm(`将领取 ${targets.length} 项任务奖励，继续？`)) return;
  const btn = $("#btn-growth-claim");
  btn.disabled = true;
  let ok = 0, fail = 0;
  for (const t of targets) {
    try {
      await invoke("growth_claim", { uid: state.growth.uid, code: t.task_code });
      ok += 1;
    } catch {
      fail += 1;
    }
  }
  btn.disabled = false;
  toast(`领取完成：成功 ${ok} 项${fail ? `，失败 ${fail} 项` : ""}`);
  await loadGrowth();
});
$('#nav .nav-item[data-view="growth"]')?.addEventListener("click", () => {
  if (!state.growth.tasks.length && !state.growth.loading) loadGrowth();
});
document.addEventListener("click", async (e) => {
  const o = e.target.closest("[data-open-url]");
  if (!o) return;
  try {
    await invoke("open_external", { url: o.dataset.openUrl });
    toast("已打开；在 WorkBuddy 里完成动作后回来点「重新查询」");
  } catch (err) {
    toast("打开失败：" + err);
  }
});
document.addEventListener("click", async (e) => {
  const cl = e.target.closest("[data-claim]");
  if (!cl) return;
  cl.disabled = true;
  try {
    await invoke("growth_claim", { uid: state.growth.uid, code: cl.dataset.claim });
    toast("已领取");
    await loadGrowth();
  } catch (err) {
    toast("领取失败：" + err);
    cl.disabled = false;
  }
});

/* 添加账号：浏览器登录 / 导入 JSON */
let loginPollTimer = null;
let loginState = null;

function closeAdd() {
  if (loginPollTimer) { clearInterval(loginPollTimer); loginPollTimer = null; }
  loginState = null;
  $("#modal-add").classList.remove("show");
}
function resetAdd() {
  if (loginPollTimer) { clearInterval(loginPollTimer); loginPollTimer = null; }
  loginState = null;
  $("#modal-add").classList.add("show");
  $$("#add-tabs .add-tab").forEach((b) => b.classList.toggle("on", b.dataset.tab === "login"));
  $("#add-tab-login").hidden = false;
  $("#add-tab-platform").hidden = true;
  $("#add-tab-import").hidden = true;
  $("#add-url").hidden = true;
  $("#add-url").textContent = "";
  $("#btn-open-url").hidden = true;
  $("#btn-copy-url").hidden = true;
  $("#btn-start-login").hidden = false;
  $("#btn-start-login").disabled = false;
  $("#add-login-msg").textContent = "";
  $("#add-import-msg").textContent = "";
}
$("#btn-add").addEventListener("click", resetAdd);
$("#btn-oauth")?.addEventListener("click", resetAdd);
$("#btn-import-backup")?.addEventListener("click", () => {
  resetAdd();
  document.querySelector('#add-tabs .add-tab[data-tab="import"]')?.click();
});
$("#btn-cli-attach")?.addEventListener("click", () => toast("需先安装 CodeBuddy CLI；接入后可在设置里自动轮换"));
$("#btn-list-refresh")?.addEventListener("click", loadAll);
$$(".segmented [data-realm]").forEach((b) => b.addEventListener("click", () => {
  $$(".segmented [data-realm]").forEach((x) => x.classList.toggle("on", x === b));
  state.realm = b.dataset.realm;
  renderAccounts();
}));
$("#modal-close").addEventListener("click", closeAdd);
$("#modal-add").addEventListener("click", (e) => { if (e.target.id === "modal-add") closeAdd(); });

$$("#add-tabs .add-tab").forEach((b) => b.addEventListener("click", () => {
  $$("#add-tabs .add-tab").forEach((x) => x.classList.toggle("on", x === b));
  $("#add-tab-login").hidden = b.dataset.tab !== "login";
  $("#add-tab-platform").hidden = b.dataset.tab !== "platform";
  $("#add-tab-import").hidden = b.dataset.tab !== "import";
}));

/* 其他平台：API Key 直连（Trae / Z.AI / BigModel / MiniMax / 豆包 …） */
const PLATFORM_PRESETS = [
  // WorkBuddy / CodeBuddy 有官方 OAuth，走「浏览器登录」页签，不放在这里
  ...UPSTREAM_PRESETS.filter(([n]) => !/^WorkBuddy/i.test(n)),
  ["自定义", ""],
];
function fillPlatformSelect() {
  const sel = $("#add-plat-select");
  if (!sel || sel.dataset.ready) return;
  sel.innerHTML = PLATFORM_PRESETS.map(([n], i) => `<option value="${i}">${esc(n)}</option>`).join("");
  sel.dataset.ready = "1";
  applyPlatformPreset();
}
function applyPlatformPreset() {
  const idx = Number($("#add-plat-select")?.value || 0);
  const p = PLATFORM_PRESETS[idx];
  if (!p) return;
  $("#add-plat-base").value = p[1] || "";
  if ($("#add-plat-model")) {
    $("#add-plat-model").placeholder = /火山方舟/.test(p[0])
      ? "填推理接入点 ID：ep-xxxxxxxx（不是 doubao-pro）"
      : "可选：留空则用该平台默认模型";
  }
  renderPlatTip();
  // 本地类上游：自动探测一次，别让用户以为填了就通
  const probeBox = $("#plat-probe");
  if (probeBox) {
    const isLocal = /本地|127\.0\.0\.1|localhost/.test(p[0] + " " + (p[1] || ""));
    if (isLocal) probePlatform();
    else probeBox.innerHTML = "";
  }
}

async function probePlatform() {
  const box = $("#plat-probe");
  const url = $("#add-plat-base")?.value.trim();
  if (!box || !url) return;
  box.innerHTML = `<span class="muted">检测中…</span>`;
  try {
    const r = await invoke("probe_endpoint", { url });
    box.innerHTML = r.ok
      ? `<span class="pill green">已连通</span> <span class="muted">${esc(r.url)} · HTTP ${r.status}，可以直接保存</span>`
      : `<span class="pill orange">连不上</span> <span class="muted">${esc(r.hint)}——先把该服务启动，或换别家平台</span>`;
  } catch (e) {
    box.innerHTML = `<span class="pill orange">检测失败</span> <span class="muted">${esc(String(e))}</span>`;
  }
}
$("#btn-plat-probe")?.addEventListener("click", probePlatform);

/* 扫描本机已登录客户端里的凭据 */
$("#btn-scan-creds")?.addEventListener("click", async () => {
  const box = $("#scan-result");
  if (!box) return;
  box.innerHTML = `<div class="empty">扫描中…</div>`;
  try {
    const r = await invoke("scan_local_creds");
    state.scanned = r.found || [];
    if (!state.scanned.length) {
      box.innerHTML = `<div class="empty">${esc(r.note)}</div>`;
      return;
    }
    box.innerHTML = `<div class="table-wrap" style="margin-bottom:12px"><table class="tbl">
      <thead><tr><th>来源</th><th>字段</th><th>凭据</th><th style="width:76px">操作</th></tr></thead>
      <tbody>${state.scanned
        .map(
          (f, i) => `<tr>
        <td>${toolLogo({ id: "", name: f.platform })}<b style="font-weight:600">${esc(f.platform)}</b><div class="sub" style="font-size:11px;color:var(--muted)">${esc(f.path)}</div></td>
        <td><code>${esc(f.field)}</code></td><td><code>${esc(f.masked)}</code></td>
        <td class="op"><button class="btn sm primary" data-use-cred="${i}">填入</button></td></tr>`,
        )
        .join("")}</tbody></table></div><p class="hint" style="padding:0">${esc(r.note)}</p>`;
  } catch (e) {
    box.innerHTML = `<div class="empty">扫描失败：${esc(String(e))}</div>`;
  }
});
document.addEventListener("click", (e) => {
  const p = e.target.closest("[data-use-platform]");
  if (p) {
    goto("create");
    const sel = $("#cr-upstream");
    if (sel) {
      sel.value = p.dataset.usePlatform;
      toast("已选中该上游，填提示词即可生成");
    }
    return;
  }
  const b = e.target.closest("[data-use-cred]");
  if (!b) return;
  const f = (state.scanned || [])[Number(b.dataset.useCred)];
  if (!f) return;
  $("#add-plat-key").value = f.raw;
  toast(`已填入 ${f.platform} 的凭据，确认上面的平台后点保存`);
});

$("#add-plat-select")?.addEventListener("change", applyPlatformPreset);
$("#btn-plat-paste")?.addEventListener("click", async () => {
  try {
    const t = await navigator.clipboard.readText();
    if (!t || !t.trim()) return toast("剪贴板是空的，先去平台后台复制 Key");
    $("#add-plat-key").value = t.trim();
    toast("已从剪贴板填入");
  } catch {
    toast("系统不让读剪贴板，手动 Ctrl+V 粘到输入框即可");
  }
});
$("#btn-save-plat")?.addEventListener("click", async () => {
  fillPlatformSelect();
  const idx = Number($("#add-plat-select").value || 0);
  const p = PLATFORM_PRESETS[idx] || ["自定义", ""];
  const name = p[0];
  const baseUrl = $("#add-plat-base").value.trim();
  const apiKey = $("#add-plat-key").value.trim();
  const protocol = $("#add-plat-proto").value;
  const msg = $("#add-plat-msg");
  if (!baseUrl) { msg.textContent = "先填 Base URL"; msg.className = "msg bad"; return; }
  if (!apiKey) { msg.textContent = "先填 API Key"; msg.className = "msg bad"; return; }
  const id = (name === "自定义" ? baseUrl.replace(/^https?:\/\//, "").split(".")[0] : name)
    .toLowerCase().replace(/[^a-z0-9]+/g, "_");
  try {
    await invoke("save_upstream", {
      upstream: {
        id, name: name === "自定义" ? id : name, baseUrl, apiKey, protocol,
        enabled: true, weight: 1,
        model: $("#add-plat-model")?.value.trim() || null,
      },
    });
    // 同步建一条账号条目，「账号管理」里立刻能看到，不用等下次启动
    await invoke("save_account", {
      account: {
        id: `up_${id}`,
        client: "apikey",
        displayName: name === "自定义" ? id : name,
        accountId: id,
        accessToken: null,
        refreshToken: null,
        expiresAt: null,
        credits: null,
        creditUsage: null,
        creditExpiry: null,
        enabled: true,
        lastUsedAt: null,
        region: null,
        source: "apikey",
        notes: baseUrl,
      },
    }).catch(() => {});
    msg.textContent = `已添加「${name}」：上游已启用，账号列表也加了一条`;
    msg.className = "msg ok";
    $("#add-plat-key").value = "";
    toast("已添加：" + name);
    await loadAll();
    goto("upstreams");
  } catch (err) {
    msg.textContent = "保存失败：" + err;
    msg.className = "msg bad";
  }
});
fillPlatformSelect();

$("#btn-start-login").addEventListener("click", async () => {
  const realm = document.querySelector('input[name="addRealm"]:checked')?.value || "cn";
  $("#btn-start-login").disabled = true;
  $("#add-login-msg").textContent = "正在创建授权会话…";
  $("#add-login-msg").className = "msg";
  try {
    const r = await invoke("panel_login_start", { realm });
    loginState = r.state;
    $("#add-url").hidden = false;
    $("#add-url").textContent = r.url || "";
    $("#btn-open-url").hidden = false;
    $("#btn-copy-url").hidden = false;
    $("#add-login-msg").textContent = "请在浏览器完成登录，完成后自动载入…";
    try {
      await invoke("open_external", { url: r.url });
    } catch {
      toast("浏览器没打开，点下面「打开浏览器」或「复制链接」手动粘贴");
    }
    loginPollTimer = setInterval(async () => {
      if (!loginState) return;
      try {
        const p = await invoke("panel_login_poll", { state: loginState });
        if (p.done) {
          clearInterval(loginPollTimer); loginPollTimer = null;
          $("#add-login-msg").textContent = `已添加 ${p.nickname || p.uid}${p.realm === "global" ? "（国际版）" : ""}`;
          $("#add-login-msg").className = "msg ok";
          toast("账号已载入");
          loadAll();
          setTimeout(closeAdd, 1200);
        }
      } catch (err) {
        clearInterval(loginPollTimer); loginPollTimer = null;
        $("#add-login-msg").textContent = "轮询失败：" + err;
        $("#add-login-msg").className = "msg bad";
      }
    }, 3000);
  } catch (err) {
    $("#btn-start-login").disabled = false;
    $("#add-login-msg").textContent = "获取授权链接失败：" + err;
    $("#add-login-msg").className = "msg bad";
  }
});

$("#btn-open-url").addEventListener("click", async () => {
  const url = $("#add-url").textContent;
  if (url) try { await invoke("open_external", { url }); } catch (err) { toast(String(err)); }
});
$("#btn-copy-url").addEventListener("click", async () => {
  try { await navigator.clipboard.writeText($("#add-url").textContent); toast("链接已复制"); } catch { toast("复制失败"); }
});

$("#add-import-file").addEventListener("change", async (e) => {
  const f = e.target.files?.[0];
  if (!f) return;
  $("#add-import-msg").textContent = "导入中…";
  $("#add-import-msg").className = "msg";
  try {
    const text = await f.text();
    const r = await invoke("panel_import_cockpit_text", { name: f.name, text });
    $("#add-import-msg").textContent = `导入完成：成功 ${r.imported ?? 0} 个${r.skipped ? `，跳过 ${r.skipped} 个` : ""}`;
    $("#add-import-msg").className = "msg ok";
    toast("导入完成");
    loadAll();
    setTimeout(closeAdd, 1200);
  } catch (err) {
    $("#add-import-msg").textContent = "导入失败：" + err;
    $("#add-import-msg").className = "msg bad";
  }
});

/* 导入导出 */
$("#file-import").addEventListener("change", async (e) => { const f = e.target.files?.[0]; if (f) $("#import-text").value = await f.text(); });
function ioMsg(m, ok = true) { const el = $("#io-msg"); el.textContent = m; el.className = "msg " + (ok ? "ok" : "bad"); }
$("#btn-import").addEventListener("click", async () => { let b; try { b = JSON.parse($("#import-text").value); } catch (err) { return ioMsg("JSON 解析失败：" + err.message, false); } try { const r = await invoke("import_accounts", { bundle: b }); ioMsg(`导入成功：新增 ${r.added}，更新 ${r.updated}，共 ${r.total}`); loadAll(); } catch (err) { ioMsg("导入失败：" + err, false); } });
async function doExport(download) { const b = await invoke("export_accounts", { includeTokens: $("#chk-tokens").checked }); const txt = JSON.stringify(b, null, 2); $("#export-text").value = txt; if (download) { const blob = new Blob([txt], { type: "application/json" }); const a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = `axiao-accounts-${new Date().toISOString().slice(0, 10)}.json`; a.click(); URL.revokeObjectURL(a.href); } }
$("#btn-export").addEventListener("click", () => doExport(false));
$("#btn-download").addEventListener("click", () => doExport(true));

/* 上游 */
$("#btn-add-upstream").addEventListener("click", () => openUpstreamModal(null));
$("#up-close").addEventListener("click", () => $("#modal-upstream").classList.remove("show"));
$("#up-cancel").addEventListener("click", () => $("#modal-upstream").classList.remove("show"));
$("#up-save").addEventListener("click", async () => {
  const editing = state.upstreamEdit;
  const prev = editing ? (state.upstreams || []).find((x) => x.id === editing) : null;
  const upstream = {
    id: $("#u-id").value.trim() || "up_" + Date.now(),
    name: $("#u-name").value.trim() || "新上游",
    baseUrl: $("#u-base").value.trim(),
    apiKey: $("#u-key").value.trim() || null,
    protocol: $("#u-proto").value,
    enabled: prev ? prev.enabled : true,
    weight: Number($("#u-weight").value) || 1,
    model: $("#u-model")?.value.trim() || null,
  };
  if (!upstream.baseUrl) return toast("Base URL 必填");
  try {
    await invoke("save_upstream", { upstream });
    // 影子账号：让「账号管理」也能看到这个平台
    await invoke("save_account", {
      account: {
        id: `up_${upstream.id}`,
        client: "apikey",
        displayName: upstream.name,
        accountId: upstream.id,
        accessToken: null,
        refreshToken: null,
        expiresAt: null,
        credits: null,
        creditUsage: null,
        creditExpiry: null,
        enabled: upstream.enabled,
        lastUsedAt: null,
        region: null,
        source: "apikey",
        notes: upstream.baseUrl,
      },
    }).catch(() => {});
    state.upstreamEdit = null;
    $("#u-id").disabled = false;
    $("#modal-upstream").classList.remove("show");
    toast(prev ? "已更新上游" : "已添加上游");
    loadAll();
  } catch (err) {
    toast("保存失败：" + err);
  }
});

/* 配置保存 / 放弃 */
$("#btn-cfg-save").addEventListener("click", async () => {
  $("#cfg-msg").textContent = "保存中…";
  $("#cfg-msg").className = "msg";
  try {
    const cfg = collectConfig();
    const saved = await invoke("local_config_save", { config: cfg });
    let n = 0;
    try {
      const r = await invoke("panel_save_config", { config: cfg });
      n = (r.restart_required || []).length;
    } catch {
      /* 面板未启动时仅写文件 */
    }
    $("#cfg-msg").textContent = n
      ? `已写入 config.json，其中 ${n} 项需重启生效`
      : "已写入 config.json（部分字段需重启）";
    $("#cfg-msg").className = "msg ok";
    await invoke("append_hook", { kind: "config", detail: "保存便携版 config.json" });
    await loadConfig();
  } catch (err) {
    $("#cfg-msg").textContent = "保存失败：" + err;
    $("#cfg-msg").className = "msg bad";
  }
});
$("#btn-cfg-reset").addEventListener("click", loadConfig);

/* 配置 */
$("#btn-test-keyring").addEventListener("click", async () => { try { await invoke("store_secret", { id: "test", secret: "hello-" + Date.now() }); const v = await invoke("load_secret", { id: "test" }); $("#config-log").textContent = "keyring OK: " + v; } catch (err) { $("#config-log").textContent = "keyring 失败: " + err; } });
$("#btn-read-auth").addEventListener("click", async () => { try { const v = await invoke("read_auth"); $("#config-log").textContent = JSON.stringify(v, null, 2); } catch (err) { $("#config-log").textContent = "读取失败: " + err; } });
$("#btn-add-hook").addEventListener("click", async () => { await invoke("append_hook", { kind: "test", detail: "手动测试钩子" }); loadAll(); });

/* 定时 */
$("#btn-save-schedule").addEventListener("click", () => { localStorage.setItem("axiao_schedule", JSON.stringify({ checkin: $("#s-checkin").value, growth: $("#s-growth").value, travel: $("#s-travel").value, activity: $("#s-activity").value, keepalive: $("#s-keepalive").value, balance: $("#s-balance").value })); $("#schedule-msg").textContent = "已保存（本地）"; $("#schedule-msg").className = "msg ok"; });
(function restoreSchedule() { try { const s = JSON.parse(localStorage.getItem("axiao_schedule") || "null"); if (s) { $("#s-checkin").value = s.checkin; $("#s-growth").value = s.growth; $("#s-travel").value = s.travel; $("#s-activity").value = s.activity; $("#s-keepalive").value = s.keepalive; $("#s-balance").value = s.balance; } } catch {} })();

renderComponents();
renderCoverage();
renderUpstreamPresets();
renderExtras();
renderAccessInfo();
renderAccounts();
if (hasTauri) {
  loadAll();
  setInterval(async () => {
    try {
      const r = await invoke("zcode_tick");
      if (r.ran) {
        toast(`ZCode 队列执行 ${r.ran} 个`);
        loadExtras();
      }
    } catch { /* ignore */ }
  }, 60000);
} else {
  toast("浏览器预览：请用 npm run dev 启动 Tauri");
}

/* 图标兜底：任何 logo 加载失败都不能出现"碎图 + alt 文字" */
document.addEventListener(
  "error",
  (e) => {
    const img = e.target;
    if (!(img instanceof HTMLImageElement) || !img.classList.contains("logo-img")) return;
    const label = (img.getAttribute("alt") || img.getAttribute("title") || "").trim();
    if (!label) {
      // 没有名字可显示就干脆隐藏，避免碎图
      img.style.display = "none";
      return;
    }
    const span = document.createElement("span");
    span.className = "logo-fallback";
    span.textContent = label.charAt(0).toUpperCase();
    span.title = label;
    img.replaceWith(span);
  },
  true,
);
