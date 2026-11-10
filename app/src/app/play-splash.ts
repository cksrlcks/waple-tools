/**
 * 시작 화면 로고 애니메이션 (Lottie). 끝까지 재생한 뒤 돌아온다.
 * 재생기와 애니메이션은 처음 켤 때만 따로 불러온다 (메인 번들에 넣지 않음).
 */
export async function playSplash(): Promise<void> {
  const box = document.getElementById("splash-anim")
  const text = document.getElementById("splash-text")
  if (!box) return
  const [{ default: lottie }, { default: data }] = await Promise.all([
    import("lottie-web/build/player/lottie_light"),
    import("@/shared/assets/splash.json"),
  ])
  box.style.display = "block"
  if (text) text.style.display = "block"
  const anim = lottie.loadAnimation({
    container: box,
    renderer: "svg",
    loop: false,
    autoplay: true,
    animationData: data,
  })
  // 재생이 어떤 이유로 끝나지 않아도 4초 뒤에는 넘어간다
  await new Promise<void>((done) => {
    anim.addEventListener("complete", () => done())
    setTimeout(done, 4000)
  })
  // 완성된 로고를 잠깐 보여 준다
  await new Promise((done) => setTimeout(done, 300))
}
