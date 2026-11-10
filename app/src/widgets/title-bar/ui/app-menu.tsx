import { useState } from "react"

import {
  THEME_OPTIONS,
  useSaveSettings,
  useSettings,
} from "@/entities/settings"
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
import { windowCall } from "../lib/window-call"
import { InfoDialog } from "./info-dialog"

type Props = {
  onOpenSettings: () => void
  /** 작업 중에는 설정을 열지 않는다 */
  locked?: boolean
}

const trigger = "h-7 rounded-md px-2.5 text-sm font-normal"

/** 프로그램 메뉴: 파일 · 보기 · 도움말 */
export function AppMenu({ onOpenSettings, locked = false }: Props) {
  const [dialog, setDialog] = useState<"about" | "update" | null>(null)
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
            <MenubarItem onClick={() => windowCall("close")}>종료</MenubarItem>
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
            <MenubarItem onClick={() => setDialog("update")}>
              업데이트 확인…
            </MenubarItem>
            <MenubarItem onClick={() => setDialog("about")}>
              LocalTools 정보
            </MenubarItem>
          </MenubarContent>
        </MenubarMenu>
      </Menubar>

      <InfoDialog
        open={dialog === "update"}
        onClose={() => setDialog(null)}
        title="업데이트 확인"
      >
        자동 업데이트는 준비 중입니다. 지금 버전은 {pkg.version}입니다.
      </InfoDialog>
      <InfoDialog
        open={dialog === "about"}
        onClose={() => setDialog(null)}
        title="LocalTools 정보"
      >
        <p>버전 {pkg.version}</p>
        <p className="mt-1">
          모든 파일은 이 컴퓨터 안에서만 처리합니다. 원본은 지우지 않고 original
          폴더로 옮깁니다.
        </p>
      </InfoDialog>
    </>
  )
}
