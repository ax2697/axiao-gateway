/**
 * 阿晓网关 · Cloudflare Worker 版（10 上游聚合）
 * 部署：wrangler deploy
 * 环境变量：GATEWAY_KEY、WB_KEY、TRAE_KEY、QODER_KEY、ZAI_KEY、BIGMODEL_KEY、
 *          MINIMAX_KEY、DOUBAO_KEY、CODEARTS_KEY、DSH_KEY、OPENAI_KEY
 */
const UPSTREAMS = [
  { name: "workbuddy", base: "https://copilot.tencent.com/v1", keyEnv: "WB_KEY" },
  { name: "trae", base: "https://api.trae.ai/v1", keyEnv: "TRAE_KEY" },
  { name: "qoder", base: "https://api.qoder.com/v1", keyEnv: "QODER_KEY" },
  { name: "zai", base: "https://api.z.ai/api/coding/paas/v4", keyEnv: "ZAI_KEY" },
  { name: "bigmodel", base: "https://open.bigmodel.cn/api/paas/v4", keyEnv: "BIGMODEL_KEY" },
  { name: "minimax", base: "https://api.minimaxi.com/v1", keyEnv: "MINIMAX_KEY" },
  { name: "doubao", base: "https://ark.cn-beijing.volces.com/api/v3", keyEnv: "DOUBAO_KEY" },
  { name: "codearts", base: "https://codearts.cn-north-4.myhuaweicloud.com/v1", keyEnv: "CODEARTS_KEY" },
  { name: "dsh", base: "http://127.0.0.1:8317/v1", keyEnv: "DSH_KEY" },
  { name: "openai", base: "https://api.openai.com/v1", keyEnv: "OPENAI_KEY" },
];

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (url.pathname === "/v1/models") {
      return Response.json({
        object: "list",
        data: UPSTREAMS.map((u) => ({ id: `${u.name}/*`, object: "model", owned_by: u.name })),
      });
    }
    if (url.pathname !== "/v1/chat/completions") {
      return new Response("not found", { status: 404 });
    }
    const auth = (req.headers.get("authorization") || "").replace(/^Bearer\s+/i, "");
    if (env.GATEWAY_KEY && auth !== env.GATEWAY_KEY) {
      return new Response("unauthorized", { status: 401 });
    }
    const body = await req.json();
    const raw = String(body.model || "");
    const [name, ...rest] = raw.split("/");
    const up = UPSTREAMS.find((u) => u.name === name) || UPSTREAMS[0];
    const model = up.name === name && rest.length ? rest.join("/") : raw;
    const key = env[up.keyEnv] || "";
    const r = await fetch(`${up.base}/chat/completions`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        ...(key ? { authorization: `Bearer ${key}` } : {}),
      },
      body: JSON.stringify({ ...body, model }),
    });
    return new Response(r.body, {
      status: r.status,
      headers: { "content-type": "application/json; charset=utf-8" },
    });
  },
};
