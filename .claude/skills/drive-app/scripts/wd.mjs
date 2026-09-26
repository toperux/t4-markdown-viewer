// WebDriver counterpart of cdp.mjs, for Linux (WebKitGTK through tauri-driver).
// node wd.mjs start <app> [args…] | eval "<expr>" | click "<css selector>" [Shift|Control|Alt]
//   | key <Key>[+<Key>…] (e.g. key ArrowRight, key Control+o) | drag "x,y x,y …"
//   | shot <out.png> | window [n] | raw <METHOD> <path> [json] | stop
// The session id is kept in a file, so separate calls share one session.
import { readFileSync, writeFileSync, rmSync } from "node:fs";
const base = `http://127.0.0.1:${process.env.WD_PORT ?? 4444}`;
const idFile = `${process.env.TMPDIR ?? "/tmp"}/t4-wd-session`;
const [cmd, arg, ...rest] = process.argv.slice(2);
const fail = (msg) => {
  console.log(msg);
  process.exit(1);
};
const wd = async (method, path, body) => {
  const r = await fetch(base + path, {
    method,
    headers: { "content-type": "application/json" },
    body: body && JSON.stringify(body),
  }).catch(() => fail(`tauri-driver is not listening on ${base}`));
  const { value } = await r.json();
  // "no such element" comes with an empty message; the selector says which.
  if (value?.error) fail(`${value.error}: ${value.message || arg}`);
  return value;
};
const s = (path = "") => {
  let id;
  try {
    id = readFileSync(idFile, "utf8");
  } catch {
    fail("no session: run start first");
  }
  return `/session/${id}${path}`;
};
// W3C key codes for the names the app cares about; anything else is sent as is.
const KEYS = {
  Control: "\uE009", Shift: "\uE008", Alt: "\uE00A", Meta: "\uE03D",
  Enter: "\uE007", Escape: "\uE00C", Tab: "\uE004", Backspace: "\uE003",
  Delete: "\uE017", Home: "\uE011", End: "\uE010", PageUp: "\uE00E", PageDown: "\uE00F",
  ArrowLeft: "\uE012", ArrowUp: "\uE013", ArrowRight: "\uE014", ArrowDown: "\uE015",
  ...Object.fromEntries([...Array(12)].map((_, i) => [`F${i + 1}`, String.fromCharCode(0xe031 + i)])),
};

if (cmd === "start") {
  const v = await wd("POST", "/session", {
    capabilities: { alwaysMatch: { "tauri:options": { application: arg, args: rest } } },
  });
  writeFileSync(idFile, v.sessionId);
  console.log("session", v.sessionId);
} else if (cmd === "eval") {
  // Inlined, not eval()'d, so the page's CSP has no say. Errors come back as values.
  const script = `const done = arguments[arguments.length - 1];
    Promise.resolve().then(() => (${arg})).then(done, (e) => done({ thrown: String(e) }));`;
  console.log(JSON.stringify(await wd("POST", s("/execute/async"), { script, args: [] })));
} else if (cmd === "click") {
  // A real click (user gesture). Refused when something covers the element:
  // then drag "x,y" clicks at a point instead. click "<sel>" Shift holds a
  // modifier through it: Shift+click is a new window, Control+click a new tab.
  const el = await wd("POST", s("/element"), { using: "css selector", value: arg });
  if (!rest[0]) await wd("POST", s(`/element/${Object.values(el)[0]}/click`), {});
  else {
    // One action per tick, side by side: modifier down, move, press, release, modifier up.
    const mod = KEYS[rest[0]] ?? rest[0];
    const pause = { type: "pause" };
    await wd("POST", s("/actions"), {
      actions: [
        { type: "key", id: "kb", actions: [{ type: "keyDown", value: mod }, pause, pause, { type: "keyUp", value: mod }] },
        {
          type: "pointer",
          id: "mouse",
          parameters: { pointerType: "mouse" },
          actions: [
            { type: "pointerMove", origin: el, x: 0, y: 0 },
            { type: "pointerDown", button: 0 },
            { type: "pointerUp", button: 0 },
            pause,
          ],
        },
      ],
    });
  }
  console.log("clicked", arg, rest[0] ?? "");
} else if (cmd === "key") {
  // Modifiers down in order, then up in reverse: Control+o is Ctrl+O.
  const keys = arg.split("+").map((k) => KEYS[k] ?? k);
  const actions = [
    ...keys.map((value) => ({ type: "keyDown", value })),
    ...keys.toReversed().map((value) => ({ type: "keyUp", value })),
  ];
  await wd("POST", s("/actions"), { actions: [{ type: "key", id: "kb", actions }] });
  console.log("pressed", arg);
} else if (cmd === "drag") {
  // Press at the first point, move through the rest, release at the last.
  // One point is a plain click there.
  const pts = arg.split(" ").map((p) => p.split(",").map(Number));
  const move = ([x, y]) => ({ type: "pointerMove", origin: "viewport", x: Math.round(x), y: Math.round(y) });
  const actions = [move(pts[0]), { type: "pointerDown", button: 0 }, ...pts.slice(1).map(move), { type: "pointerUp", button: 0 }];
  await wd("POST", s("/actions"), {
    actions: [{ type: "pointer", id: "mouse", parameters: { pointerType: "mouse" }, actions }],
  });
  console.log("dragged", arg);
} else if (cmd === "shot") {
  writeFileSync(arg, Buffer.from(await wd("GET", s("/screenshot")), "base64"));
  console.log("saved", arg);
} else if (cmd === "window") {
  // No argument: list the handles. A number switches to that one; which
  // window it is, ask it: eval "appWindow.label"
  const handles = await wd("GET", s("/window/handles"));
  if (arg === undefined) console.log(JSON.stringify(handles));
  else {
    await wd("POST", s("/window"), { handle: handles[Number(arg)] });
    console.log("switched to", handles[Number(arg)]);
  }
} else if (cmd === "raw") {
  // Any other endpoint, relative to the session: raw GET /url
  console.log(JSON.stringify(await wd(arg, s(rest[0]), rest[1] && JSON.parse(rest[1]))));
} else if (cmd === "stop") {
  await wd("DELETE", s());
  rmSync(idFile);
  console.log("stopped");
} else {
  console.log("unknown command:", cmd);
  process.exit(2);
}
