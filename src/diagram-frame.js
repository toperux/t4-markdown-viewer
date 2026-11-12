/*
 * Runs inside the diagram frame: a sandboxed iframe with scripts but no origin
 * of its own, so nothing mermaid draws or runs here can reach the app's page
 * or Tauri. The page sends one diagram at a time and gets back mermaid's SVG
 * string, or why it failed.
 */
if (typeof mermaid === "undefined") parent.postMessage({ failed: true }, "*");
else {
  addEventListener("message", async (e) => {
    if (e.source !== parent || !e.data) return;
    // The app's own fonts, as bytes: this frame has no origin and can't load
    // its CSS. How many loaded goes back, for the checks.
    if (Array.isArray(e.data.fonts)) {
      const faces = e.data.fonts.flatMap(({ weight, style, data }) => {
        if (!(data instanceof ArrayBuffer)) return [];
        const face = new FontFace("Libron", data, { weight: String(weight), style: String(style) });
        document.fonts.add(face);
        return [face.load().then(() => 1, () => 0)];
      });
      const loaded = (await Promise.all(faces)).reduce((a, b) => a + b, 0);
      parent.postMessage({ fontsReady: true, loaded }, "*");
      return;
    }
    const { n, config, id, source, width } = e.data;
    // The width mermaid lays out in — gantt fills it — as the page's was.
    document.body.style.width = `${width}px`;
    try {
      mermaid.initialize(config);
      const { svg: drawn, diagramType } = await mermaid.render(id, source);
      // WebKit leaves an `em` in a label's position unscaled in a zoomed
      // document, so the label slides off its box (measured). The same length
      // in px, taken here where nothing is zoomed, zooms with the rest. Parsed
      // as HTML, as the page does, and attached so the font sizes are computed.
      const box = document.createElement("div");
      document.body.append(box);
      box.innerHTML = drawn;
      for (const el of box.querySelectorAll("text, tspan"))
        for (const name of ["x", "y", "dx", "dy"])
          if (/^\s*-?[\d.]+em\s*$/.test(el.getAttribute(name)))
            el.setAttribute(name, parseFloat(el.getAttribute(name)) * parseFloat(getComputedStyle(el).fontSize));
      const svg = box.innerHTML;
      box.remove();
      // A gantt's `click … href` links are not in its drawing: mermaid wires
      // them to the page it drew in, which is gone by the time the app has the
      // drawing. Its own map of them — URLs already made safe — goes back too.
      // A failure to read it costs the links, not the drawing.
      let links = [];
      if (diagramType === "gantt" && /\bclick\b/.test(source))
        try {
          links = [...(await mermaid.mermaidAPI.getDiagramFromText(source)).db.getLinks()];
        } catch {}
      parent.postMessage({ n, svg, diagramType, links }, "*");
    } catch (err) {
      parent.postMessage({ n, error: String(err?.message ?? err) }, "*");
    }
  });
  parent.postMessage({ ready: true }, "*");
}
