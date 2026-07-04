import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { isTauri } from "./platform";
import "./themes.css";
import "./styles.css";

document.body.classList.add(isTauri ? "platform-tauri" : "platform-web");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
