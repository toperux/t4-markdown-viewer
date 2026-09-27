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
    const { n, config, id, source, width } = e.data;
    // The width mermaid lays out in — gantt fills it — as the page's was.
    document.body.style.width = `${width}px`;
    try {
      mermaid.initialize(config);
      const { svg, diagramType } = await mermaid.render(id, source);
      parent.postMessage({ n, svg, diagramType }, "*");
    } catch (err) {
      parent.postMessage({ n, error: String(err?.message ?? err) }, "*");
    }
  });
  parent.postMessage({ ready: true }, "*");
}
