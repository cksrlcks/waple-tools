import { Radio as RadioPrimitive } from "@base-ui/react/radio"
import { RadioGroup as RadioGroupPrimitive } from "@base-ui/react/radio-group"
import { cn } from "cn"

type Props = {
  options: { value: string; label: string }[]
  value: string
  onChange: (value: string) => void
  "aria-label"?: string
  className?: string
}

/** 몇 개 안 되는 선택지를 한 줄 버튼으로 (화살표 키로 이동) */
function Segmented({ options, value, onChange, className, ...props }: Props) {
  return (
    <RadioGroupPrimitive
      data-slot="segmented"
      value={value}
      onValueChange={(v) => onChange(String(v))}
      className={cn("bg-muted inline-flex shrink-0 rounded-lg p-0.5", className)}
      {...props}
    >
      {options.map((o) => (
        <RadioPrimitive.Root
          key={o.value}
          value={o.value}
          // Radio.Root는 <span>이라 inline-flex를 줘야 높이·가운데 정렬이 먹는다
          className="inline-flex items-center justify-center text-muted-foreground hover:text-foreground focus-visible:ring-primary/40 data-checked:bg-card data-checked:text-foreground data-checked:shadow-button-subtle h-6.5 rounded-md px-3 text-sm whitespace-nowrap transition-colors outline-none focus-visible:ring-2"
        >
          {o.label}
        </RadioPrimitive.Root>
      ))}
    </RadioGroupPrimitive>
  )
}

export { Segmented }
