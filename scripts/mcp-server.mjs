#!/usr/bin/env node
/**
 * 阿晓网关 MCP 服务端（stdio，无依赖）
 * 环境变量：AXIAO_BASE（默认 http://127.0.0.1:7863/v1）、AXIAO_KEY
 */
const BASE = (process.env.AXIAO_BASE || "http://127.0.0.1:7863/v1").replace(/\/$/, "");
const KEY = process.env.AXIAO_KEY || "";

const send = (obj) => process.stdout.write(JSON.stringify(obj) + "\n");

const TOOLS = [
  {
    name: "chat",
    description: "调用阿晓网关（OpenAI 兼容）完成一次对话",
    inputSchema: {
      type: "object",
      properties: {
        model: { type: "string" },
        messages: { type: "array", items: { type: "object" } },
      },
      required: ["model", "messages"],
    },
  },
  {
    name: "models",
    description: "列出网关可用模型",
    inputSchema: { type: "object", properties: {} },
  },
];

async function callTool(name, args) {
  if (name === "models") {
    const r = await fetch(`${BASE}/models`, {
      headers: KEY ? { authorization: `Bearer ${KEY}` } : {},
    });
    return await r.json();
  }
  const r = await fetch(`${BASE}/chat/completions`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      ...(KEY ? { authorization: `Bearer ${KEY}` } : {}),
    },
    body: JSON.stringify(args || {}),
  });
  return await r.json();
}

process.stdin.setEncoding("utf8");
let buf = "";
process.stdin.on("data", async (chunk) => {
  buf += chunk;
  let idx;
  while ((idx = buf.indexOf("\n")) >= 0) {
    const line = buf.slice(0, idx).trim();
    buf = buf.slice(idx + 1);
    if (!line) continue;
    let msg;
    try {
      msg = JSON.parse(line);
    } catch {
      continue;
    }
    try {
      if (msg.method === "initialize") {
        send({
          jsonrpc: "2.0",
          id: msg.id,
          result: {
            protocolVersion: "2024-11-05",
            capabilities: { tools: {} },
            serverInfo: { name: "axiao-gateway", version: "0.9.5" },
          },
        });
      } else if (msg.method === "tools/list") {
        send({ jsonrpc: "2.0", id: msg.id, result: { tools: TOOLS } });
      } else if (msg.method === "tools/call") {
        const result = await callTool(msg.params?.name, msg.params?.arguments);
        send({
          jsonrpc: "2.0",
          id: msg.id,
          result: { content: [{ type: "text", text: JSON.stringify(result) }] },
        });
      } else if (msg.id !== undefined) {
        send({ jsonrpc: "2.0", id: msg.id, error: { code: -32601, message: "method not found" } });
      }
    } catch (e) {
      send({ jsonrpc: "2.0", id: msg.id, error: { code: -32000, message: String(e) } });
    }
  }
});
