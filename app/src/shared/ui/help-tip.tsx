import { IconHelpCircle } from "@tabler/icons-react"

import { cn } from "@/shared/lib/utils"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/shared/ui/tooltip"

type Props = {
  /** 풍선에 보여 줄 설명 */
  text: string
  className?: string
}

/** 라벨 옆 원형 물음표: 마우스를 올리거나 포커스하면 풍선으로 설명 */
export function HelpTip({ text, className }: Props) {
  return (
    <Tooltip>
      <TooltipTrigger
        type="button"
        aria-label={text}
        className={cn(
          "text-muted-foreground/70 hover:text-foreground focus-visible:ring-primary/40 inline-flex shrink-0 rounded-full outline-none focus-visible:ring-2",
          className,
        )}
      >
        <IconHelpCircle className="size-3.5" stroke={1.75} />
      </TooltipTrigger>
      <TooltipContent side="top" className="max-w-64 leading-snug">
        {text}
      </TooltipContent>
    </Tooltip>
  )
}
