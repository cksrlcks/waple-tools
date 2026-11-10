import { useState } from "react"

import {
  THEME_OPTIONS,
  useSaveSettings,
  useSettings,
} from "@/entities/settings"
import { checkForUpdate } from "@/features/app-update"
import logoFull from "@/shared/assets/logo-full.svg"
import {
  Menubar,
  MenubarContent,
  MenubarItem,
  MenubarMenu,
  MenubarRadioGroup,
  MenubarRadioItem,
  MenubarSeparator,
  MenubarTrigger,
} from "@/shared/ui/menubar"

import pkg from "../../../../package.json"
import { quitApp } from "../api/quit-app"
import { InfoDialog } from "./info-dialog"

type Props = {
  onOpenSettings: () => void
  /** 작업 중에는 설정을 열지 않는다 */
  locked?: boolean
}

const trigger = "h-7 rounded-md px-2.5 text-sm font-normal"

/** 프로그램 메뉴: 파일 · 보기 · 도움말 */
export function AppMenu({ onOpenSettings, locked = false }: Props) {
  const [dialog, setDialog] = useState<"about" | null>(null)
  const settings = useSettings()
  const save = useSaveSettings()
  const theme = settings.data?.theme ?? "system"

  return (
    <>
      <Menubar className="h-7 gap-0 rounded-none border-0 bg-transparent p-0 shadow-none">
        <MenubarMenu>
          <MenubarTrigger className={trigger}>파일</MenubarTrigger>
          <MenubarContent className="min-w-48">
            <MenubarItem disabled={locked} onClick={onOpenSettings}>
              설정…
            </MenubarItem>
            <MenubarSeparator />
            <MenubarItem onClick={quitApp}>종료</MenubarItem>
          </MenubarContent>
        </MenubarMenu>

        <MenubarMenu>
          <MenubarTrigger className={trigger}>보기</MenubarTrigger>
          <MenubarContent className="min-w-48">
            <MenubarRadioGroup
              value={theme}
              onValueChange={(v) =>
                settings.data &&
                save.mutate({
                  ...settings.data,
                  theme: v as typeof theme,
                })
              }
            >
              {THEME_OPTIONS.map((o) => (
                <MenubarRadioItem key={o.value} value={o.value}>
                  {o.label}
                </MenubarRadioItem>
              ))}
            </MenubarRadioGroup>
          </MenubarContent>
        </MenubarMenu>

        <MenubarMenu>
          <MenubarTrigger className={trigger}>도움말</MenubarTrigger>
          <MenubarContent className="min-w-48">
            <MenubarItem onClick={() => void checkForUpdate(true)}>
              업데이트 확인…
            </MenubarItem>
            <MenubarItem onClick={() => setDialog("about")}>
              waple tools 정보
            </MenubarItem>
          </MenubarContent>
        </MenubarMenu>
      </Menubar>

      <InfoDialog
        open={dialog === "about"}
        onClose={() => setDialog(null)}
        title="waple tools 정보"
      >
        <img
          src={logoFull}
          alt="waple tools"
          className="mt-3 mb-3 h-6 dark:invert"
        />
        <p className="mb-3">버전 {pkg.version}</p>
        <p>모든 파일은 이 컴퓨터 안에서만 처리합니다.</p>
      </InfoDialog>
    </>
  )
}
