import { HomePage } from "@/pages/home"

import { Providers } from "./providers"
import { ThemeSync } from "./theme-sync"

export function App() {
  return (
    <Providers>
      <ThemeSync />
      <HomePage />
    </Providers>
  )
}
