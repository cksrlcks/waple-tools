export type ThemeSetting = "system" | "light" | "dark"

export function resolveTheme(
  theme: ThemeSetting,
  prefersDark: boolean,
): "light" | "dark" {
  if (theme === "system") return prefersDark ? "dark" : "light"
  return theme
}
