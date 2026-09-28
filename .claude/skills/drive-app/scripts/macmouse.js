// Real OS mouse input on macOS, the counterpart of click.ps1 and `xdotool click`:
// the plugin's WebDriver input is synthetic JS, which no drag or gesture sees.
//   osascript -l JavaScript macmouse.js click x,y [mods] | dblclick x,y
//     | drag "x,y x,y …" [mods]
// Screen points from the main display's top-left. mods: cmd, shift, alt, ctrl,
// joined with +. Needs Accessibility for the terminal; bring the app to the
// front first, or the events go to whatever window is under the point.
ObjC.import("CoreGraphics");

function run([verb, pts, mods = ""]) {
  const FLAGS = {
    cmd: $.kCGEventFlagMaskCommand,
    shift: $.kCGEventFlagMaskShift,
    alt: $.kCGEventFlagMaskAlternate,
    ctrl: $.kCGEventFlagMaskControl,
  };
  // An HID-state source, posted at the session tap: with no source, posted at
  // the HID tap, events moved the cursor but never reached the webview (measured).
  const src = $.CGEventSourceCreate($.kCGEventSourceStateHIDSystemState);
  // Checked before anything is posted; a throw makes osascript exit non-zero.
  if (!["click", "dblclick", "drag"].includes(verb)) throw new Error(`unknown verb: ${verb}`);
  const flags = mods.split("+").filter(Boolean).reduce((f, m) => {
    if (!Object.hasOwn(FLAGS, m)) throw new Error(`unknown modifier: ${m}`);
    return f | FLAGS[m];
  }, 0);
  const points = pts.split(" ").map((p) => p.split(",").map(Number));
  // Flags on every event of the gesture, and a click count WebKit reads as
  // `detail`: 2 on the second pair is what makes a dblclick.
  const post = (type, [x, y], clicks = 1) => {
    const e = $.CGEventCreateMouseEvent(src, type, $.CGPointMake(x, y), $.kCGMouseButtonLeft);
    $.CGEventSetFlags(e, flags);
    $.CGEventSetIntegerValueField(e, $.kCGMouseEventClickState, clicks);
    $.CGEventPost($.kCGSessionEventTap, e);
    delay(0.02);
  };
  const click = (p, n) => {
    post($.kCGEventLeftMouseDown, p, n);
    post($.kCGEventLeftMouseUp, p, n);
  };

  post($.kCGEventMouseMoved, points[0]);
  if (verb === "click") click(points[0], 1);
  else if (verb === "dblclick") {
    click(points[0], 1);
    click(points[0], 2);
  } else {
    // Moves with the button held are "dragged" events; a plain move would read
    // buttons === 0 in the page, and the app cancels its drag on that.
    post($.kCGEventLeftMouseDown, points[0]);
    for (let i = 1; i < points.length; i++) {
      const [[x0, y0], [x1, y1]] = [points[i - 1], points[i]];
      for (let s = 1; s <= 10; s++)
        post($.kCGEventLeftMouseDragged, [x0 + ((x1 - x0) * s) / 10, y0 + ((y1 - y0) * s) / 10]);
    }
    post($.kCGEventLeftMouseUp, points[points.length - 1]);
  }
  return `${verb} ${pts} ${mods}`.trim();
}
