/* Global startup error trap.
 *
 * Loaded as a same-origin script (not inline) so the app can run under a strict
 * `script-src 'self'` Content Security Policy. It must stay a classic, non-defer
 * script tag in <head> so it executes before the module bundle and can catch a
 * failure during module evaluation or React mount — otherwise that failure is a
 * silent blank white window.
 *
 * If React has already mounted (the #otm-boot placeholder is gone) this is a no-op.
 */
(function () {
  function showFatal(message, stack) {
    var root = document.getElementById("root") || document.body;
    if (!document.getElementById("otm-boot")) return;
    root.innerHTML = "";
    var wrap = document.createElement("div");
    wrap.setAttribute(
      "style",
      "font-family:ui-monospace,Menlo,monospace;background:#0b1620;" +
        "color:#e7f1f8;height:100%;width:100%;box-sizing:border-box;" +
        "padding:20px;overflow:auto"
    );
    var head = document.createElement("div");
    head.setAttribute("style", "color:#ff8a8a;font-weight:600;margin-bottom:8px");
    head.textContent = "OpenTokenMonitor failed to start";
    var detail = document.createElement("div");
    detail.setAttribute(
      "style",
      "color:#9fb3c2;font-size:12px;white-space:pre-wrap;word-break:break-word"
    );
    detail.textContent =
      (message || "Unknown error") + (stack ? "\n\n" + stack : "");
    wrap.appendChild(head);
    wrap.appendChild(detail);
    root.appendChild(wrap);
  }

  window.addEventListener("error", function (e) {
    showFatal(e.message, e.error && e.error.stack);
  });
  window.addEventListener("unhandledrejection", function (e) {
    var reason = e.reason;
    showFatal(
      reason && reason.message ? reason.message : String(reason),
      reason && reason.stack
    );
  });
})();
