import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import App from "./App";
import ErrorBoundary from "@/components/states/ErrorBoundary";

// Last-resort fatal renderer. Used when React itself cannot be created (the
// ErrorBoundary inside the tree cannot catch that). Builds the DOM with
// textContent so an error string can never inject markup.
function renderFatal(container: HTMLElement, error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  const stack = error instanceof Error ? error.stack ?? "" : "";
  container.innerHTML = "";

  const wrap = document.createElement("div");
  wrap.setAttribute(
    "style",
    "font-family:ui-monospace,Menlo,monospace;background:#0b1620;color:#e7f1f8;" +
      "height:100%;width:100%;box-sizing:border-box;padding:20px;overflow:auto",
  );

  const head = document.createElement("div");
  head.setAttribute("style", "color:#ff8a8a;font-weight:600;margin-bottom:8px");
  head.textContent = "OpenTokenMonitor failed to start";

  const detail = document.createElement("div");
  detail.setAttribute(
    "style",
    "color:#9fb3c2;font-size:12px;white-space:pre-wrap;word-break:break-word",
  );
  detail.textContent = message + (stack ? `\n\n${stack}` : "");

  wrap.appendChild(head);
  wrap.appendChild(detail);
  container.appendChild(wrap);
}

const container = document.getElementById("root");

if (!container) {
  document.body.textContent = "OpenTokenMonitor: #root element missing.";
} else {
  try {
    ReactDOM.createRoot(container).render(
      <React.StrictMode>
        <ErrorBoundary>
          <App />
        </ErrorBoundary>
      </React.StrictMode>,
    );
  } catch (error) {
    renderFatal(container, error);
  }
}
