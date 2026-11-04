#!/usr/bin/env python3
"""打「成品包」——发给普通用户的那个 zip。

跟源码包的区别，很多人会搞混：
- **成品包**（这个）：双击就能跑，5 个文件，不用装任何东西
- 源码包（另一个脚本）：137 个文件，要装 Rust 自己编译

以前只打了源码包，桌面上就一个 `*-source.zip`，用户看到的是
一堆 .rs 文件，根本不知道从哪下手。成品包才是他们要的。

用法：
    python scripts/make-dist-zip.py
"""
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DIST = ROOT / "dist"


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
    if not DIST.exists():
        print("!! 没有 dist/ 目录，先跑构建")
        return 1

    ver = "0.0.0"
    cargo = ROOT / "src-tauri" / "Cargo.toml"
    if cargo.exists():
        for line in cargo.read_text(encoding="utf-8").splitlines():
            if line.startswith("version"):
                ver = line.split("=")[1].strip().strip('"')

    files = sorted(p for p in DIST.iterdir() if p.is_file())
    if not files:
        print("!! dist/ 是空的")
        return 1

    # 必须有 exe，否则这个包是废的
    if not any(p.name.endswith(".exe") for p in files):
        print("!! dist/ 里没有 exe，包是废的")
        return 1

    out = desktop() / f"【用这个】阿晓网关助手-v{ver}.zip"
    if out.exists():
        out.unlink()

    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for p in files:
            z.write(p, f"阿晓网关助手/{p.name}")

    size = out.stat().st_size / 1024 / 1024
    print(f"OK  成品包 {len(files)} 个文件  {size:.2f} MB")
    print(f"    {out}")
    for p in files:
        print(f"      {p.name:<24} {p.stat().st_size/1024:.0f} KB")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
