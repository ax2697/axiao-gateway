#!/usr/bin/env python3
"""打源码发布包。

从**当前工作副本**打（不是某个旧拷贝），排除构建产物与依赖缓存，
但**必须保留**两样东西，否则拿到源码的人编译不出来：

- `src-tauri/resource.o`：预编译的资源（manifest + 图标）。本机 windres 坏了，
  构建要靠它；`.gitignore` 里 `*.o` 被忽略，打包时不能跟着忽略。
- `src-tauri/src/catalog.json`：免费模型目录，`include_str!` 编译期嵌入。

用法：
    python scripts/make-source-zip.py
    python scripts/make-source-zip.py --out D:/somewhere/x.zip
"""
import argparse
import os
import time
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

EXCLUDE_DIRS = {
    "node_modules", "target", "dist", "release", "gen",
    ".git", "__pycache__", ".vs", ".idea", ".venv", ".pytest_cache",
    # 过程产物 / 本机状态
    "AI工作区", ".axiao",
}

# 按扩展名排除，但下面有白名单
EXCLUDE_EXT = {".exe", ".dll", ".pdb", ".rlib", ".zip", ".7z", ".rar", ".log"}

# 必须留下：预编译资源对象（不然没 manifest，exe 双击闪退）
KEEP_NAMES = {"resource.o"}


def desktop() -> Path:
    """真实桌面路径。

    不能假设是 `~/Desktop` —— 这台机器用 360 把桌面迁到了 D 盘，
    而 `Path.home()/Desktop` 指向的 C 盘目录几乎是空的。包丢在那里
    用户根本看不到（实际发生过：用户说"桌面都是旧版"，因为新包
    压根没出现在他看得见的桌面上）。
    """
    import os as _os
    try:
        import winreg
        with winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders",
        ) as k:
            v, _ = winreg.QueryValueEx(k, "Desktop")
            p = Path(_os.path.expandvars(v))
            if p.exists():
                return p
    except Exception:
        pass
    return Path.home() / "Desktop"



def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=None)
    args = ap.parse_args()

    # 版本号从 Cargo.toml 读，别手写（手写必然忘记改）
    ver = "0.0.0"
    cargo = ROOT / "src-tauri" / "Cargo.toml"
    if cargo.exists():
        for line in cargo.read_text(encoding="utf-8").splitlines():
            if line.startswith("version"):
                ver = line.split("=")[1].strip().strip('"')

    out = Path(args.out) if args.out else (
        desktop() / f"阿晓网关助手-v{ver}-source.zip"
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    if out.exists():
        out.unlink()

    count = 0
    skipped = []
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for root, dirs, files in os.walk(ROOT):
            dirs[:] = [d for d in dirs if d not in EXCLUDE_DIRS]
            for f in files:
                ext = os.path.splitext(f)[1].lower()
                if ext in EXCLUDE_EXT and f not in KEEP_NAMES:
                    continue
                full = Path(root) / f
                rel = Path("阿晓网关助手") / full.relative_to(ROOT)
                z.write(full, str(rel).replace("\\", "/"))
                count += 1
            # 记录被跳过的目录名，便于核对
            if not dirs and not files:
                skipped.append(str(Path(root).relative_to(ROOT)))

    size = out.stat().st_size / 1024 / 1024
    print(f"OK  文件 {count} 个  {size:.2f} MB")
    print(f"    {out}")

    # 校验两个"编译必需品"确实在包里
    with zipfile.ZipFile(out) as z:
        names = z.namelist()
        must = [
            "阿晓网关助手/src-tauri/resource.o",
            "阿晓网关助手/src-tauri/src/catalog.json",
            "阿晓网关助手/src-tauri/src/main.rs",
            "阿晓网关助手/src/index.html",
            "阿晓网关助手/src/app.js",
            "阿晓网关助手/CHANGELOG.md",
        ]
        missing = [m for m in must if m not in names]
        if missing:
            print("!! 缺少关键文件，源码包不完整：")
            for m in missing:
                print(f"   - {m}")
            return 1
        print("    关键文件齐全（resource.o / catalog.json / 前端 / 变更日志）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
