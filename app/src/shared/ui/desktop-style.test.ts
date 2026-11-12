/// <reference types="node" />
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (p: string) => readFileSync(new URL(p, import.meta.url), "utf8");
const css = read("../../app/styles/index.css");
const block = (sel: string) => css.slice(css.indexOf(`${sel} {`), css.indexOf("\n}", css.indexOf(`${sel} {`)));

const TOKENS = ["--menu", "--menu-separator", "--dialog-footer", "--dialog-footer-border", "--primary-hover", "--primary-active",
  "--destructive-hover", "--outline-hover", "--primary-edge", "--primary-edge-bottom", "--outline-edge", "--outline-edge-bottom",
  "--destructive-edge", "--destructive-edge-bottom"];

describe("desktop design tokens", () => {
  it.each(TOKENS)("%s is defined for light and dark", (t) => {
    expect(block(":root")).toContain(`${t}:`);
    expect(block(".dark")).toContain(`${t}:`);
  });
  it.each(["--shadow-menu", "--shadow-dialog", "--shadow-button", "--shadow-button-subtle"])("%s is a theme shadow", (t) => {
    expect(block("@theme inline")).toContain(`${t}:`);
  });
});

const MENU_FILES = ["./context-menu.tsx", "./dropdown-menu.tsx", "./menubar.tsx"];
const ANIMATED = ["./context-menu.tsx", "./dropdown-menu.tsx", "./menubar.tsx", "./dialog.tsx", "./tooltip.tsx", "./select.tsx"];

describe("menus", () => {
  const styles = (() => {
    try {
      return read("./menu-styles.ts");
    } catch {
      return "";
    }
  })();
  it.each(MENU_FILES)("%s uses shared menu styles", (f) => expect(read(f)).toContain('from "./menu-styles"'));
  it("popup is translucent glass", () => {
    expect(styles).toMatch(/menuPopup = "[^"]*bg-menu/);
    expect(styles).toMatch(/menuPopup = "[^"]*backdrop-blur/);
    expect(styles).toMatch(/menuPopup = "[^"]*shadow-menu/);
  });
  it("highlight is neutral, destructive highlight is destructive", () => {
    expect(styles).toMatch(/menuItem = "[^"]*data-highlighted:bg-foreground\/8/);
    expect(styles).toMatch(/menuItem = "[^"]*data-\[variant=destructive\]:data-highlighted:bg-destructive/);
  });
});

describe("no web-style motion", () => {
  it.each(ANIMATED)("%s has no zoom or slide animation", (f) => {
    expect(read(f)).not.toMatch(/zoom-(in|out)-|slide-in-from-/);
  });
});

describe("dialog", () => {
  const d = read("./dialog.tsx");
  it("close button is opt-in", () => expect(d).toMatch(/function DialogContent\(\{[^}]*showCloseButton = false/));
  it("box uses dialog shadow, no ring", () => {
    expect(d).toContain("shadow-dialog");
    expect(d).not.toMatch(/data-slot="dialog-content"[\s\S]*?ring-1/);
  });
  it("footer is a tight gray band, always a row", () => {
    expect(d).toMatch(/dialog-footer[\s\S]*bg-dialog-footer/);
    expect(d).not.toContain("flex-col-reverse");
  });
});

describe("button", () => {
  const b = read("./button.tsx");
  it("no web press nudge", () => expect(b).not.toContain("translate-y-px"));
  it("default/outline/destructive have edge borders", () => {
    expect(b).toMatch(/default: "[^"]*border-primary-edge[^"]*border-b-primary-edge-bottom/);
    expect(b).toMatch(/outline:\s*"[^"]*border-outline-edge[^"]*border-b-outline-edge-bottom/);
    expect(b).toMatch(/destructive:\s*"[^"]*bg-destructive [^"]*text-white/);
  });
  it("default size is 32px", () => expect(b).toMatch(/default:\s*"h-8 /));
});

describe("button scope", () => {
  const b = read("./button.tsx");
  const base = b.slice(b.indexOf("cva(") + 4, b.indexOf("{", b.indexOf("cva(")));
  it("ghost/link/secondary keep their shape (base radius and weight unchanged)", () => {
    expect(base).toContain("rounded-lg");
    expect(base).toContain("font-medium");
  });
  it.each(["default", "outline", "destructive"])("%s variant sets the desktop radius and weight", (v) => {
    expect(b).toMatch(new RegExp(`${v}:\\s*"[^"]*rounded-md font-normal`));
  });
});

describe("button sizes keep ghost/secondary/link as before", () => {
  const b = read("./button.tsx");
  const compound = b.slice(b.indexOf("compoundVariants"));
  it("has compound variants for the untouched variants", () => {
    expect(compound).toMatch(/variant: \["ghost", "secondary", "link"\]/);
  });
  it("sm stays 28px and default stays 32px for them", () => {
    expect(compound).toMatch(/size: "sm",\s*class: "h-7/);
    expect(compound).toMatch(/size: "default",\s*class: "h-8/);
  });
});
