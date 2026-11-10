import { Switch as SwitchPrimitive } from "@base-ui/react/switch"

import { cn } from "@/shared/lib/utils"

/** 켜기·끄기 스위치 (28×16) */
function Switch({ className, ...props }: SwitchPrimitive.Root.Props) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        "bg-input focus-visible:ring-primary/40 data-checked:bg-primary relative inline-flex h-4 w-7 shrink-0 items-center rounded-full p-0.5 transition-colors outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className="bg-background shadow-item block size-3 rounded-full transition-transform data-checked:translate-x-3"
      />
    </SwitchPrimitive.Root>
  )
}

export { Switch }
