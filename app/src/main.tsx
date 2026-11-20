import React from "react"
import { getCurrentWindow } from "@tauri-apps/api/window"
import ReactDOM from "react-dom/client"

import { App } from "@/app/app"
import { PanelApp } from "@/app/panel-app"
import { playSplash } from "@/app/play-splash"

import "./app/styles/index.css"

// 옵션 패널 별도 창(라벨 `panel`)은 같은 index.html로 패널 앱만 그린다
let isPanel = false
try {
  isPanel = getCurrentWindow().label === "panel"
} catch {
  // Tauri 밖(브라우저 미리보기): 메인 앱
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{isPanel ? <PanelApp /> : <App />}</React.StrictMode>,
)

// 시작 화면: 앱을 처음 켤 때만 로고 애니메이션, 새로고침이면 바로 없앤다 (패널 창은 없음)
const splash = document.getElementById("splash")
const hideSplash = () => {
  splash?.classList.add("opacity-0")
  splash?.addEventListener("transitionend", () => splash.remove())
}
let firstRun = true
try {
  // 세션 기록은 새로고침해도 남고, 앱을 다시 켜면 비어 있다
  firstRun = !isPanel && !sessionStorage.getItem("splash-played")
  if (!isPanel) sessionStorage.setItem("splash-played", "1")
} catch {
  // 저장소를 못 쓰면 매번 처음 켠 것으로 본다
  firstRun = !isPanel
}
if (firstRun) void playSplash().then(hideSplash, hideSplash)
else splash?.remove()
