export const mediaSeekKeys = {
  all: ["media-probe"] as const,
  probe: (toolId: string, path: string) =>
    [...mediaSeekKeys.all, toolId, path] as const,
}
