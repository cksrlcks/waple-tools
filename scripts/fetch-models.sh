#!/usr/bin/env bash
# AI 도구(업스케일·배경 제거) 모델을 각 크레이트의 model/ 폴더에 받는다. 이미 있고 sha256이 맞으면 건너뛴다.
# 모델은 DLL 안에 include_bytes!로 들어간다 (git에는 넣지 않는다).
#   사용) scripts/fetch-models.sh                 # 둘 다
#         scripts/fetch-models.sh image.upscale   # 도구 하나만
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
only="${1:-}"

fetch() { # <도구 id> <파일> <url> <sha256>
  local id="$1" file="$2" url="$3" sum="$4"
  [ -z "$only" ] || [ "$only" = "$id" ] || return 0
  if [ -f "$file" ] && echo "$sum  $file" | sha256sum -c --status; then
    echo "있음: $file"
    return 0
  fi
  mkdir -p "$(dirname "$file")"
  echo "받는 중: $url"
  curl -fsSL --retry 3 -o "$file.part" "$url"
  if ! echo "$sum  $file.part" | sha256sum -c --status; then
    echo "sha256 불일치: $url"
    rm -f "$file.part"
    exit 1
  fi
  mv "$file.part" "$file"
}

# Real-ESRGAN x4plus (BSD-3-Clause)
fetch image.upscale "$root/crates/tools/image-upscale/model/model.onnx" \
  "https://huggingface.co/bukuroo/RealESRGAN-ONNX/resolve/main/real-esrgan-x4plus-128.onnx" \
  6a6f4a3d58553d40fdd443d9e5f4b2deb9b52bef1ec2947700fc2167ac876c7d

# BiRefNet_lite (MIT)
fetch image.bg_remove "$root/crates/tools/image-bg-remove/model/model.onnx" \
  "https://huggingface.co/onnx-community/BiRefNet_lite-ONNX/resolve/main/onnx/model.onnx" \
  5600024376f572a557870a5eb0afb1e5961636bef4e1e22132025467d0f03333
