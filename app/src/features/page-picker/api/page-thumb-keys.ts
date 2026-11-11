export const pageThumbKeys = {
  all: ["page-thumbnails"] as const,
  of: (toolId: string, path: string) =>
    [...pageThumbKeys.all, toolId, path] as const,
}
