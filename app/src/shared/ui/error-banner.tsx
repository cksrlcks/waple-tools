import { cn } from "@/shared/lib/utils"

type Props = {
  message: string
  className?: string
}

export function ErrorBanner({ message, className }: Props) {
  return (
    <div
      role="alert"
      className={cn(
        "border-destructive/30 bg-destructive/10 text-destructive border-b px-3 py-1.5 text-sm",
        className,
      )}
    >
      {message}
    </div>
  )
}
