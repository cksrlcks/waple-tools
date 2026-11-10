import {
  IconAppWindow,
  IconBraces,
  IconCopy,
  IconCrop,
  IconDroplet,
  IconGif,
  IconLayoutGrid,
  IconPhotoDown,
  IconTool,
  IconVectorBezier,
  type Icon,
} from "@tabler/icons-react"

const ICONS: Record<string, Icon> = {
  "image.optimize": IconPhotoDown,
  "image.edit": IconCrop,
  "image.watermark": IconDroplet,
  "image.svg": IconVectorBezier,
  "image.gif_optimize": IconGif,
  "image.sprite": IconLayoutGrid,
  "image.base64": IconBraces,
  "image.icon": IconAppWindow,
  "dev.copy": IconCopy,
}

/** 도구 목록·화면 머리에 쓰는 아이콘 (모르는 도구는 공구 아이콘) */
export const toolIcon = (id: string): Icon => ICONS[id] ?? IconTool
