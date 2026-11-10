import React from "react"
import ReactDOM from "react-dom/client"

import { App } from "@/app/app"

import "./app/styles/index.css"

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)

// 시작 로고: 잠깐 보여준 뒤 흐리게 사라진다
const splash = document.getElementById("splash")
setTimeout(() => {
  splash?.classList.add("opacity-0")
  splash?.addEventListener("transitionend", () => splash.remove())
}, 800)
