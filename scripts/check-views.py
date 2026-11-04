#!/usr/bin/env python3
"""页面完整性静态检查。

用户报过好几次「页面是空的」。这类问题的根因几乎都是可枚举的：

1. 侧栏 `data-view="x"` 指向的 `#view-x` 容器不存在 → 点了白屏
2. JS 里 `$("#foo")` 拼成了一个 HTML 里没有的 id → 静默拿到 null，
   后续 `.textContent = ...` 报错，整块渲染中断（比白屏更难查）
3. HTML 里重复 id → `getElementById` 只返回第一个，监听器绑错元素
4. PAGE_META / GUIDE 里有条目但没对应视图 → 导航能点，内容是空的

这些不用跑浏览器就能查出来，而且比"截图看一遍"可靠得多——截图只能看到
当前那一页，这个脚本一次覆盖全部 30+ 个视图。

用法：
    python scripts/check-views.py
退出码：0 干净；1 发现问题
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HTML = ROOT / "src" / "index.html"
JS = ROOT / "src" / "app.js"


def read(p: Path) -> str:
    if not p.exists():
        print(f"[FAIL] 找不到 {p}")
        sys.exit(1)
    return p.read_text(encoding="utf-8")


def main():
    html = read(HTML)
    js = read(JS)

    problems = []

    # ---------- 收集 HTML 里的 id ----------
    ids = re.findall(r'\bid="([^"]+)"', html)
    id_set = set(ids)

    # 重复 id
    seen = set()
    dup = []
    for i in ids:
        if i in seen:
            dup.append(i)
        seen.add(i)
    if dup:
        problems.append(f"HTML 里有重复 id：{sorted(set(dup))}")

    # ---------- 视图容器 ----------
    views = set(re.findall(r'class="view[^"]*"\s+id="view-([a-z0-9_]+)"', html))
    navs = set(re.findall(r'data-view="([a-z0-9_]+)"', html))

    missing_views = sorted(navs - views)
    if missing_views:
        problems.append(f"侧栏指向的视图容器不存在（点了会白屏）：{missing_views}")

    orphan_views = sorted(views - navs)
    if orphan_views:
        problems.append(f"有视图但侧栏没有入口（用户找不到）：{orphan_views}")

    # ---------- PAGE_META ----------
    meta_block = re.search(r"const PAGE_META = \{(.*?)\n\};", js, re.S)
    if meta_block:
        meta_keys = set(re.findall(r"^\s{2}([a-z0-9_]+):", meta_block.group(1), re.M))
        bad = sorted(meta_keys - views)
        if bad:
            problems.append(f"PAGE_META 里这些 key 没有对应视图：{bad}")

    # ---------- GUIDE 条目 ----------
    guide_ids = set(re.findall(r'id: "([a-z0-9_]+)", title: "', js))
    bad_guide = sorted(guide_ids - views)
    if bad_guide:
        problems.append(f"教程条目没有对应视图（点了「怎么用」跳不过去）：{bad_guide}")

    # ---------- JS 里引用的 id 是否都存在 ----------
    # 只查 `$("#literal")` 和 `$$("#literal ...")` 这种字面量，
    # 动态拼接（`$("#" + x)`）跳不过，会在下面单独说明
    refs = set(re.findall(r'\$\$?\("#([a-zA-Z0-9_-]+)', js))
    # 排除两个已知的非缺失情况：
    #   - 以 "-" 结尾：是 `$("#view-" + name)` 这种前缀，正则只抓到了前半截
    #   - 在 JS 里被 `xxx.id = "yyy"` 动态创建的元素
    dyn_ids = set(re.findall(r'\.id\s*=\s*"([^"]+)"', js))
    # 还有一种：JS 里拼 HTML 字符串时写了 id="xxx"，插进 DOM 后再查（合法）
    dyn_ids |= set(re.findall(r'id="([a-zA-Z0-9_-]+)"', js))
    missing_refs = sorted(x for x in (refs - id_set) if not x.endswith("-") and x not in dyn_ids)
    if missing_refs:
        # 排除那些是 CSS 选择器前缀（比如 #nav .nav-item 里的 nav 其实是存在的）
        problems.append(
            f"JS 引用了 HTML 里不存在的 id（静默拿到 null，对应渲染会中断）：{missing_refs}"
        )

    # 动态拼接的引用数量（提示性，不算错误）
    dyn = len(re.findall(r'\$\$?\("#"\s*\+', js))

    # ---------- 报告 ----------
    print(f"视图 {len(views)} 个 / HTML id {len(id_set)} 个 / JS 引用 {len(refs)} 个")
    if dyn:
        print(f"另有 {dyn} 处动态拼接的 id 引用，静态检查覆盖不到（需要时人工看）")

    if problems:
        print("\n!!! 发现 %d 类问题 !!!" % len(problems))
        for p in problems:
            print(f"  - {p}")
        return 1

    print("\n检查通过：视图 / 导航 / PAGE_META / 教程 / id 引用 全部对得上")
    return 0


if __name__ == "__main__":
    sys.exit(main())
