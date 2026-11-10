export const previewKeys = {
  thumbnails: ["thumbnail"] as const,
  thumbnail: (path: string) => [...previewKeys.thumbnails, path] as const,
  tools: ["tool-preview"] as const,
  tool: (toolId: string, path: string | null, opts: string) =>
    [...previewKeys.tools, toolId, path, opts] as const,
}
