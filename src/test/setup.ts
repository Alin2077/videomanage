import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";

// globals: false 模式下需手动清理渲染 DOM
afterEach(() => {
  cleanup();
});

// antd v5 在 jsdom 环境所需 polyfill
if (typeof window !== "undefined") {
  // matchMedia
  if (!window.matchMedia) {
    (window as unknown as { matchMedia: unknown }).matchMedia = (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    });
  }
}

// ResizeObserver
if (typeof globalThis !== "undefined" && !("ResizeObserver" in globalThis)) {
  (globalThis as unknown as { ResizeObserver: unknown }).ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

// getComputedStyle 部分字段（antd 使用）
if (typeof window !== "undefined" && !window.getComputedStyle) {
  (window as unknown as { getComputedStyle: unknown }).getComputedStyle = () => ({
    getPropertyValue: () => "",
  });
}
