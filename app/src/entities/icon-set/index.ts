export type { Collection, IconItem, IconSet, InstalledSet } from "./model/types"
export {
  checkIconUpdates,
  iconCollections,
  installIconSet,
  listIconSets,
  readIconSet,
  removeIconSet,
} from "./api/icon-set-api"
export { iconSetKeys } from "./api/icon-set-keys"
export { useInstalledSets } from "./api/use-installed-sets"
export { useIconSet } from "./api/use-icon-set"
export { RECOMMENDED } from "./config/recommended"
export type { Recommended } from "./config/recommended"
export { styleOf, stylesOf } from "./lib/style-of"
export { iconSize, strokeOf, toSvg } from "./lib/render-svg"
export type { RenderOpts } from "./lib/render-svg"
export { toClassSnippet, toDataUri, toJsx } from "./lib/copy-formats"
