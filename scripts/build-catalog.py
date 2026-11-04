#!/usr/bin/env python3
"""从 freellmapi 源码 + 数据库提取「免费模型目录」，生成给本应用用的 catalog.json。

为什么要这个脚本而不是手抄：412 个模型、35 家厂商的配额数字会变，
手抄必然过期。重跑一次脚本就能刷新，成本几秒。

数据来源（都在 freellmapi 仓库里）：
  1. server/data/freeapi.db  —— models / media_models / embedding_models 三张表，
     带 RPM/RPD/TPM/月度预算/上下文/是否支持视觉与工具/付费单价
  2. server/src/providers/*.ts —— 每家厂商的 platform id、显示名、baseUrl、协议

用法：
    python scripts/build-catalog.py --src "D:/.../中心汇总/freellmapi"
    python scripts/build-catalog.py --src ... --out src-tauri/src/catalog.json
"""
import argparse
import json
import os
import re
import sqlite3
import sys
from datetime import date
from pathlib import Path

# 常见 provider 文件里的常量形式：const XXX_BASE_URL = 'https://...'
CONST_RE = re.compile(r"(?:export\s+)?const\s+([A-Z][A-Z0-9_]*)\s*=\s*['\"](https?://[^'\"]+)['\"]")
# 两种写法都要抓：
#   super({ platform: 'x', ... })              ← 自定义适配器内部
#   new OpenAICompatProvider({ platform: 'x' }) ← index.ts 里内联注册的
CFG_RE = re.compile(
    r"(?:super|new\s+\w+Provider)\(\s*\{(?P<body>.*?)\}\s*\)", re.S
)
FIELD_RE = {
    "platform": re.compile(r"platform:\s*['\"]([^'\"]+)['\"]"),
    "name": re.compile(r"name:\s*['\"]([^'\"]+)['\"]"),
    "baseUrl": re.compile(r"baseUrl:\s*(?:['\"]([^'\"]+)['\"]|([A-Z][A-Z0-9_]*))"),
}

# 官方直营（相对可信）与第三方聚合站的区分，直接用平台 id 白名单。
OFFICIAL = {
    "google", "mistral", "cohere", "nvidia", "cloudflare", "groq", "cerebras",
    "zhipu", "modelscope", "moondream", "speechify", "exa", "firecrawl",
    "linkup", "tavily", "brave", "parallel", "sail", "typhoon",
}

# 有 23 家的 name 会退化成平台 id（自定义适配器不写 name 字段）。
# 能自动首字母大写的就不列了，这里只补真正需要改写的品牌名。
NAME_FIX = {
    "google": "Google AI Studio",
    "cohere": "Cohere",
    "cloudflare": "Cloudflare Workers AI",
    "aihorde": "AI Horde",
    "aclide": "Aclide",
    "moondream": "Moondream",
    "modelscope": "ModelScope",
    "llm7": "LLM7",
    "llmtr": "LLMTR",
    "unorouter": "UnoRouter",
    "bazaarlink": "BazaarLink",
    "router9": "Router9",
    "dreamprompting": "DreamPrompting",
}


def pretty_name(pid: str, raw: str) -> str:
    if pid in NAME_FIX:
        return NAME_FIX[pid]
    # 只有"和 id 一字不差"才需要美化；ElectronHub / xKiro 这种本来就写好的不能动
    if raw == pid:
        return pid[:1].upper() + pid[1:]
    return raw

# 协议：Gemini 用自家格式，其余绝大多数是 OpenAI 兼容
PROTOCOL = {"google": "gemini", "cohere": "cohere"}


def parse_providers(providers_dir: Path, index_file: Path):
    """返回 {platform: {name, baseUrl, file, note}}"""
    out = {}

    # 先建全局常量表（baseUrl 常写成 const XXX_BASE_URL）
    consts = {}
    for f in list(providers_dir.glob("*.ts")):
        for m in CONST_RE.finditer(f.read_text(encoding="utf-8", errors="replace")):
            consts[m.group(1)] = m.group(2)
    for m in CONST_RE.finditer(index_file.read_text(encoding="utf-8", errors="replace")):
        consts[m.group(1)] = m.group(2)

    def absorb(text, fname):
        for cfg in CFG_RE.finditer(text):
            body = cfg.group("body")
            p = FIELD_RE["platform"].search(body)
            if not p:
                continue
            plat = p.group(1)
            n = FIELD_RE["name"].search(body)
            b = FIELD_RE["baseUrl"].search(body)
            base = ""
            if b:
                base = b.group(1) or consts.get(b.group(2), "")
            out[plat] = {
                "name": (n.group(1) if n else plat),
                "baseUrl": base,
                "file": fname,
            }

    for f in providers_dir.glob("*.ts"):
        absorb(f.read_text(encoding="utf-8", errors="replace"), f.name)
    absorb(index_file.read_text(encoding="utf-8", errors="replace"), "index.ts")

    # 第二遍：自定义适配器常用 `readonly platform = 'x'` + 文件内 URL 常量，
    # 不走 super({...})，上面的正则抓不到。按文件兜底。
    for f in providers_dir.glob("*.ts"):
        text = f.read_text(encoding="utf-8", errors="replace")
        m = re.search(r"readonly\s+platform(?::\s*\w+)?\s*=\s*['\"](\w+)['\"]", text)
        if not m:
            continue
        plat = m.group(1)
        if out.get(plat, {}).get("baseUrl"):
            continue
        urls = re.findall(r"const\s+([A-Z][A-Z0-9_]*)\s*=\s*['\"](https?://[^'\"]+)['\"]", text)
        if not urls:
            # 有些类把地址写成实例字段：private readonly baseUrl = 'https://...'
            holder = re.findall(r"readonly\s+baseUrl\s*=\s*['\"](https?://[^'\"]+)['\"]", text)
            urls = [("BASE_URL", u) for u in holder]
        base = ""
        for nm, u in urls:
            if "BASE" in nm or "API" in nm:
                base = u
                break
        if not base and urls:
            base = urls[0][1]
        # Cloudflare 的地址里带 account id，没法写死，给占位符
        if plat == "cloudflare":
            base = "https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/v1"
        if base:
            out.setdefault(plat, {"name": plat, "file": f.name})
            out[plat]["baseUrl"] = base
    return out, consts


def load_db(db: Path):
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    con.row_factory = sqlite3.Row
    return con


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", required=True, help="freellmapi 仓库根目录")
    ap.add_argument("--out", default="src-tauri/src/catalog.json")
    args = ap.parse_args()

    root = Path(args.src)
    db_path = root / "server" / "data" / "freeapi.db"
    pdir = root / "server" / "src" / "providers"
    idx = pdir / "index.ts"
    for p in (db_path, pdir, idx):
        if not p.exists():
            print(f"[FAIL] 缺少 {p}")
            return 1

    meta, consts = parse_providers(pdir, idx)
    con = load_db(db_path)
    cur = con.cursor()

    # ---- 厂商级汇总 ----
    rows = cur.execute("""
        SELECT platform,
               COUNT(*) n,
               SUM(CASE WHEN enabled=1 THEN 1 ELSE 0 END) enabled_n,
               SUM(CASE WHEN supports_vision=1 THEN 1 ELSE 0 END) vision,
               SUM(CASE WHEN supports_tools=1 THEN 1 ELSE 0 END) tools,
               MAX(rpm_limit) rpm,
               MAX(rpd_limit) rpd,
               MAX(tpm_limit) tpm,
               MAX(tpd_limit) tpd,
               MAX(context_window) ctx,
               MIN(paid_input_per_m) pin,
               MIN(paid_output_per_m) pout
        FROM models GROUP BY platform
    """).fetchall()

    providers = []
    missing_meta = []
    for r in rows:
        plat = r["platform"]
        m = meta.get(plat, {})
        if not m.get("baseUrl"):
            missing_meta.append(plat)
        # 月度预算：取该平台出现最多的那个字符串（同一家基本一致）
        budgets = [x[0] for x in cur.execute(
            "SELECT monthly_token_budget FROM models WHERE platform=? AND monthly_token_budget<>'' ",
            (plat,)
        ) if x[0]]
        budget = max(set(budgets), key=budgets.count) if budgets else ""
        providers.append({
            "id": plat,
            "name": pretty_name(plat, m.get("name") or plat),
            "baseUrl": m.get("baseUrl", ""),
            "protocol": PROTOCOL.get(plat, "openai"),
            "official": plat in OFFICIAL,
            "models": r["n"],
            "enabled": r["enabled_n"] or 0,
            "vision": r["vision"] or 0,
            "tools": r["tools"] or 0,
            "rpm": r["rpm"],
            "rpd": r["rpd"],
            "tpm": r["tpm"],
            "tpd": r["tpd"],
            "context": r["ctx"],
            "budget": budget,
            "paidInPerM": r["pin"],
            "paidOutPerM": r["pout"],
        })

    providers.sort(key=lambda x: (-x["models"], x["id"]))

    # ---- 模型明细（紧凑数组，控制体积）----
    # [platform, model_id, display_name, intel_rank, speed_rank, ctx, vision, tools, rpm, rpd, budget]
    models = []
    for r in cur.execute("""
        SELECT platform, model_id, display_name, intelligence_rank, speed_rank,
               context_window, supports_vision, supports_tools,
               rpm_limit, rpd_limit, monthly_token_budget
        FROM models WHERE enabled=1
        ORDER BY platform, intelligence_rank
    """):
        models.append([
            r["platform"], r["model_id"], r["display_name"],
            r["intelligence_rank"], r["speed_rank"],
            r["context_window"], r["supports_vision"], r["supports_tools"],
            r["rpm_limit"], r["rpd_limit"], r["monthly_token_budget"] or "",
        ])

    media = [[r["platform"], r["model_id"], r["display_name"], r["modality"], r["quota_label"]]
             for r in cur.execute(
                 "SELECT platform, model_id, display_name, modality, quota_label FROM media_models WHERE enabled=1")]

    embeddings = [[r["platform"], r["model_id"], r["display_name"], r["dimensions"], r["max_input_tokens"]]
                  for r in cur.execute(
                      "SELECT platform, model_id, display_name, dimensions, max_input_tokens FROM embedding_models WHERE enabled=1")]

    out = {
        "generatedAt": date.today().isoformat(),
        "source": "freellmapi（github）server/data/freeapi.db + providers/*.ts",
        "providerCount": len(providers),
        "modelCount": len(models),
        "providers": providers,
        "models": models,
        "media": media,
        "embeddings": embeddings,
    }

    outp = Path(args.out)
    outp.parent.mkdir(parents=True, exist_ok=True)
    outp.write_text(json.dumps(out, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")

    size = outp.stat().st_size
    print(f"厂商 {len(providers)} 家 / 模型 {len(models)} 个 / 媒体 {len(media)} / 向量 {len(embeddings)}")
    print(f"输出 {outp}  {size/1024:.1f} KB")
    if missing_meta:
        print(f"\n⚠ 有 {len(missing_meta)} 家没解析到 baseUrl，需要手工补：")
        for p in sorted(missing_meta):
            print(f"   - {p}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
