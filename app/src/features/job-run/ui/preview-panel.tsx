import type { Plan } from "@/entities/tool"
import { cn } from "@/shared/lib/utils"

type Props = {
  plan: Plan
  className?: string
}

export function PreviewPanel({ plan, className }: Props) {
  const dirs = [...new Set(plan.files.map((f) => f.destDir))]
  return (
    <section
      className={cn(
        "bg-muted/40 flex flex-col gap-1.5 rounded-md border p-3 text-xs",
        className,
      )}
    >
      <p className="text-sm">
        대상 {plan.files.length}개 · 건너뜀 {plan.skipped.length}개
      </p>
      {dirs.length > 0 && (
        <p className="text-muted-foreground">
          결과 위치: {dirs.slice(0, 3).join(", ")}
          {dirs.length > 3 && ` 외 ${dirs.length - 3}곳`}
        </p>
      )}
      {plan.skipped.length > 0 && (
        <ul className="text-muted-foreground">
          {plan.skipped.map((s) => (
            <li key={s.path} className="truncate" title={s.path}>
              {s.path}: {s.reason}
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}
