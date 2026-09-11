// node cdp.mjs <port> eval "<expr>" | shot <out.png> | click "<css selector>"
import { writeFileSync } from "node:fs";
const [port, cmd, arg] = process.argv.slice(2);
const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const page = pages.find((p) => p.type === "page" && p.url.includes("tauri"));
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r, { once: true }));
let id = 0;
const send = (method, params) =>
  new Promise((resolve) => {
    const my = ++id;
    ws.addEventListener("message", function on(e) {
      const msg = JSON.parse(e.data);
      if (msg.id !== my) return;
      ws.removeEventListener("message", on);
      resolve(msg.result ?? msg.error);
    });
    ws.send(JSON.stringify({ id: my, method, params }));
  });
const evaluate = async (expression) => {
  const r = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  return r.exceptionDetails ?? r.result?.value ?? r;
};
if (cmd === "eval") {
  console.log(JSON.stringify(await evaluate(arg)));
} else if (cmd === "click") {
  // A real click (user gesture), at the element's centre.
  const at = await evaluate(`(() => { const r = document.querySelector(${JSON.stringify(arg)})?.getBoundingClientRect();
    return r && r.width ? { x: r.x + r.width / 2, y: r.y + r.height / 2 } : null; })()`);
  if (!at) console.log("not found or not visible:", arg);
  else {
    for (const type of ["mousePressed", "mouseReleased"])
      await send("Input.dispatchMouseEvent", { type, x: at.x, y: at.y, button: "left", clickCount: 1 });
    console.log("clicked", arg, JSON.stringify(at));
  }
} else {
  const r = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(arg, Buffer.from(r.data, "base64"));
  console.log("saved", arg);
}
ws.close();
