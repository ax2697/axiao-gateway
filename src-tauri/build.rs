fn main() {
    // 资源（manifest + 图标）有两条路，二选一，不能同时走，
    // 否则链接器报 "duplicate resource: type ICON / type MANIFEST"。
    //
    // 路线 A（默认，推荐）：让 tauri-build 自己调 windres 生成资源库 libresource.a，
    //   里面已经含 Common Controls v6 manifest 和全部图标。
    //   前提是本机 windres 可用（MinGW 的 bin 要在 PATH 里，见 docs/BUILD-NOTES.md 坑 5）。
    //
    // 路线 B（windres 坏掉时的兜底）：构建时同时设
    //   TAURI_SKIP_WINRES=1（让 tauri-build 跳过资源编译）
    //   WB_LINK_RESOURCE=1（让这里把预编译好的 src-tauri/resource.o 链进去）
    //
    // 为什么用显式开关而不是"检测到就链"：检测不出来。tauri-build 生成的是
    // libresource.a，本脚本运行时无法预知它会不会被跳过，只能由调用方声明。
    //
    // 两条都不走会怎样：exe 没有 .rsrc 段 → 没嵌 manifest → Windows 按
    // comctl32 v5 加载 → TaskDialogIndirect 找不到 → 进程以 0xC0000139 退出，
    // 表现是"双击没反应"。详见 docs/BUILD-NOTES.md 坑 9。
    // 一旦跳过 windres，资源就必须自己补上，所以这两个条件联动，
    // 不需要调用方记两个变量。
    let skip_winres = std::env::var_os("TAURI_SKIP_WINRES").is_some();
    let force = std::env::var_os("WB_LINK_RESOURCE").is_some();
    if skip_winres || force {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let res = std::path::Path::new(&manifest_dir).join("resource.o");
        if res.exists() {
            println!("cargo:rustc-link-arg={}", res.display());
            println!("cargo:rerun-if-changed={}", res.display());
        } else {
            println!(
                "cargo:warning=跳过 windres 但找不到 src-tauri/resource.o，exe 会缺 manifest 并可能双击闪退"
            );
        }
    }
    println!("cargo:rerun-if-env-changed=WB_LINK_RESOURCE");
    println!("cargo:rerun-if-env-changed=TAURI_SKIP_WINRES");

    tauri_build::build()
}
