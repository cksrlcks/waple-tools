// 아이콘 라이브러리 도구의 무료 세트 목록 스냅샷을 갱신한다. 개발할 때만 실행한다 (앱·DLL은 실행하지 않음).
// 사용: node scripts/update-icon-collections.mjs
import { gunzipSync } from "node:zlib"
import { writeFileSync } from "node:fs"

const FREE = ["MIT", "Apache-2.0", "ISC", "CC0-1.0"]
const ID = /^[a-z0-9]+(-[a-z0-9]+)*$/
const OUT = new URL("../crates/tools/web-icon-library/data/collections.json", import.meta.url)

const meta = await (await fetch("https://registry.npmjs.org/@iconify/collections/latest")).json()
const tgz = Buffer.from(await (await fetch(meta.dist.tarball)).arrayBuffer())
const tar = gunzipSync(tgz)

// 최소 tar 읽기: 512바이트 헤더(이름 0..100, 크기 124..136 8진수), 데이터는 512 단위로 채워짐
let json
for (let off = 0; off + 512 <= tar.length; ) {
  const name = tar.toString("utf8", off, off + 100).replace(/\0.*$/, "")
  if (!name) break
  const size = parseInt(tar.toString("utf8", off + 124, off + 136).replace(/\0.*$/, "").trim(), 8)
  if (name === "package/collections.json") {
    json = JSON.parse(tar.toString("utf8", off + 512, off + 512 + size))
    break
  }
  off += 512 + Math.ceil(size / 512) * 512
}
if (!json) throw new Error("package/collections.json 없음")

const list = Object.entries(json)
  .filter(([id, c]) => ID.test(id) && !c.hidden && FREE.includes(c.license?.spdx))
  .map(([id, c]) => ({
    id,
    name: c.name,
    total: c.total ?? 0,
    category: c.category ?? null,
    license: c.license.spdx,
    palette: !!c.palette,
  }))
  .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))

writeFileSync(OUT, JSON.stringify(list, null, 1) + "\n")
console.log(`@iconify/collections ${meta.version}: ${list.length}개`)
