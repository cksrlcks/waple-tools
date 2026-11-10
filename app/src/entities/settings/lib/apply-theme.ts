import { resolveTheme, type ThemeSetting } from "@/shared/lib/resolve-theme"

/** `<html class="dark">`를 설정에 맞춘다. "system"이면 OS 설정 변경도 따라가며, 해제 함수를 돌려준다. */
export function applyTheme(theme: ThemeSetting): () => void {
  const media = window.matchMedia("(prefers-color-scheme: dark)")
  const apply = () =>
    document.documentElement.classList.toggle(
      "dark",
      resolveTheme(theme, media.matches) === "dark",
    )
  apply()
  if (theme !== "system") return () => {}
  media.addEventListener("change", apply)
  return () => media.removeEventListener("change", apply)
}
