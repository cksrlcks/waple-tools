// 우클릭·드롭다운·메뉴 막대가 같이 쓰는 데스크톱 메뉴 모양(반투명 유리). 값은 docs/superpowers/specs/2026-10-27-desktop-ui-redesign-design.md 4.1
// 확대·미끄러짐 애니메이션 없이 짧은 페이드만 쓴다

/** 메뉴·하위 메뉴 상자 */
export const menuPopup = "z-50 max-h-(--available-height) min-w-36 origin-(--transform-origin) overflow-x-hidden overflow-y-auto rounded-[10px] bg-menu p-[5px] text-popover-foreground shadow-menu backdrop-blur-[24px] backdrop-saturate-[1.8] outline-none data-open:animate-in data-open:fade-in-0 data-open:duration-75 data-closed:animate-out data-closed:fade-out-0 data-closed:duration-100";

/** 항목. 강조(마우스·키보드)는 파랑 바탕에 흰 글자, 위험 항목은 빨강 바탕. 아이콘 없는 항목은 inset으로 아이콘 칸만큼 들여 글자 시작선을 맞춘다 */
export const menuItem = "relative flex h-6 cursor-default items-center gap-2 rounded-[5px] pr-2.5 pl-2 text-sm outline-hidden select-none data-highlighted:bg-primary data-highlighted:text-primary-foreground data-highlighted:**:text-primary-foreground/85 data-inset:pl-8 data-[variant=destructive]:text-destructive data-[variant=destructive]:*:[svg]:text-destructive data-[variant=destructive]:data-highlighted:bg-destructive data-[variant=destructive]:data-highlighted:text-white data-[variant=destructive]:data-highlighted:**:text-white/85 data-disabled:pointer-events-none data-disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";

/** 체크·라디오 항목. 표시는 오른쪽 끝(right-2)이라 그만큼 오른쪽을 비운다 */
export const menuCheckItem = `${menuItem} pr-8`;

/** 하위 메뉴를 여는 항목. 하위 메뉴가 열려 있는 동안 강조 유지 */
export const menuSubTrigger = `${menuItem} data-popup-open:bg-primary data-popup-open:text-primary-foreground data-popup-open:**:text-primary-foreground/85`;

export const menuSeparator = "mx-2.5 my-[5px] h-px bg-menu-separator";

/** 단축키·하위 메뉴 화살표. 강조 시 흰색은 menuItem의 **: 규칙이 처리 */
export const menuShortcut = "ml-auto text-xs tracking-widest text-muted-foreground mac:tracking-normal";

export const menuLabel = "px-2 py-1 text-xs text-muted-foreground data-inset:pl-8";
