# dist（构建产物，不提交）

```bash
npm install
npx tauri build --no-bundle
# 产物：src-tauri/target/release/axiao-gateway.exe
```

发布包建议包含：

```
阿晓网关助手-v0.1.0-win64.zip
├─ axiao-gateway.exe
├─ config.example.json
├─ 使用说明.txt
└─ docs/
```

> 不要把 `auths/`、`config.json`、任何 token 打进发布包。
