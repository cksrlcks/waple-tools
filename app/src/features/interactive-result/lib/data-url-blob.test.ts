import { describe, expect, it } from "vitest"

import { dataUrlBlob } from "./data-url-blob"

describe("dataUrlBlob", () => {
  it("base64 data URL을 형식·바이트 그대로 Blob으로", async () => {
    const blob = dataUrlBlob(`data:image/png;base64,${btoa("\x89PNG")}`)
    expect(blob.type).toBe("image/png")
    expect([...new Uint8Array(await blob.arrayBuffer())]).toEqual([
      0x89, 0x50, 0x4e, 0x47,
    ])
  })
  it("형식이 없으면 image/png", () => {
    expect(dataUrlBlob(`data:;base64,${btoa("x")}`).type).toBe("image/png")
  })
})
