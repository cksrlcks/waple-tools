import type { CSSProperties } from "react"
import { Toaster as Sonner, type ToasterProps } from "sonner"

/** 토스트 자리. 색은 앱 토큰을 따른다 (sonner 기본 색 대신) */
export function Toaster(props: ToasterProps) {
  return (
    <Sonner
      position="bottom-right"
      style={
        {
          "--normal-bg": "var(--popover)",
          "--normal-text": "var(--popover-foreground)",
          "--normal-border": "var(--border)",
          "--border-radius": "var(--radius)",
        } as CSSProperties
      }
      {...props}
    />
  )
}
