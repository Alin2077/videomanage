import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, waitFor } from "@testing-library/react";
import VideoPlayer from "./VideoPlayer";
import { videoApi } from "../services/api";
import type { VideoInfo } from "../types";

// ---- mock 依赖 ----
vi.mock("../services/api", () => ({
  videoApi: {
    logOpen: vi.fn().mockResolvedValue(42),
    logClose: vi.fn().mockResolvedValue(undefined),
  },
  toAssetUrl: vi.fn((p: string) => `asset://${p}`),
}));

// artplayer 实例化 mock（避免真实 DOM/媒体加载）
vi.mock("artplayer", () => ({
  default: class {
    container: unknown;
    currentTime = 0;
    duration = 60;
    toggle() {}
    destroy() {}
    constructor(opts: { container: unknown }) {
      this.container = opts.container;
    }
  },
}));

const mockVideo: VideoInfo = {
  id: 1,
  folderId: 1,
  fileName: "test.mp4",
  filePath: "C:\\test.mp4",
  fileSize: 1024,
  duration: 60,
  width: 1920,
  height: 1080,
  codec: "h264",
  fps: 30,
  sampleRate: null,
  coverPath: null,
  customTitle: null,
  notes: null,
  openCount: 0,
  fileHash: null,
  createdAt: "2026-01-01 00:00:00",
  modifiedAt: "2026-01-01 00:00:00",
  scannedAt: "2026-01-01 00:00:00",
  tags: [],
};

describe("VideoPlayer 播放次数", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("父组件重渲染（onClose 引用变化）时 logOpen 只记录一次，避免播放次数暴涨", async () => {
    const { rerender, unmount } = render(<VideoPlayer video={mockVideo} onClose={() => {}} />);
    // 挂载时记录一次打开
    expect(videoApi.logOpen).toHaveBeenCalledTimes(1);

    // 模拟父组件多次重渲染（每次传入新的 onClose 箭头函数）
    for (let i = 0; i < 10; i++) {
      rerender(<VideoPlayer video={mockVideo} onClose={() => {}} />);
    }
    // 打开日志仍只有 1 次（修复前会随每次渲染 +1 → 无限循环）
    expect(videoApi.logOpen).toHaveBeenCalledTimes(1);

    unmount();
    // 卸载后等待打开日志完成，补记一次关闭日志
    await waitFor(() => {
      expect(videoApi.logClose).toHaveBeenCalledTimes(1);
    });
  });
});
