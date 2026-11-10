/// <reference types="vite/client" />

// lottie-web의 가벼운 SVG 재생기에는 타입이 없어 본 패키지 타입을 쓴다
declare module "lottie-web/build/player/lottie_light" {
  import lottie from "lottie-web"
  export default lottie
}
