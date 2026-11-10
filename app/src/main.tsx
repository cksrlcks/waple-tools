import React from "react"
import ReactDOM from "react-dom/client"

import { App } from "@/app/app"
import { playSplash } from "@/app/play-splash"

import "./app/styles/index.css"

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)

// 시작 화면: 앱을 처음 켤 때만 로고 애니메이션, 새로고침이면 바로 없앤다
const splash = document.getElementById("splash")
const hideSplash = () => {
  splash?.classList.add("opacity-0")
  splash?.addEventListener("transitionend", () => splash.remove())
}
let firstRun = true
try {
  // 세션 기록은 새로고침해도 남고, 앱을 다시 켜면 비어 있다
  firstRun = !sessionStorage.getItem("splash-played")
  sessionStorage.setItem("splash-played", "1")
} catch {
  // 저장소를 못 쓰면 매번 처음 켠 것으로 본다
}
if (firstRun) void playSplash().then(hideSplash, hideSplash)
else splash?.remove()
