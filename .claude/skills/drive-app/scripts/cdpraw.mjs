// node cdpraw.mjs <port> <method> [jsonParams] — send one raw CDP command to the first tauri page.
const [port, method, params = "{}"] = process.argv.slice(2);
const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
console.log(list.map((t) => `${t.type} ${t.url}`).join("\n"));
const page = list.find((t) => t.type === "page");
if (!page) process.exit(1);
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r));
ws.send(JSON.stringify({ id: 1, method, params: JSON.parse(params) }));
const t = setTimeout(() => { console.log("no reply (2s)"); process.exit(0); }, 2000);
ws.addEventListener("message", (e) => { console.log(e.data.slice(0, 500)); clearTimeout(t); process.exit(0); });
