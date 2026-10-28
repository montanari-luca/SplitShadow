import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { applyTheme } from "./theme";
import "./index.css";

applyTheme();
document.body.classList.toggle("solid", getCurrentWindow().label !== "palette");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
