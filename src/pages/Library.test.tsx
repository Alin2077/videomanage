import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import Library from "./Library";

// ---- mock 依赖 ----
vi.mock("../services/api", () => ({
  videoApi: {
    list: vi.fn(),
    openWithPlayer: vi.fn(),
    updateMeta: vi.fn(),
    batchDelete: vi.fn(),
    search: vi.fn(),
  },
  tagApi: {
    batchAdd: vi.fn(),
    setVideoTags: vi.fn(),
  },
  toAssetUrl: vi.fn((p: string) => `asset://${p}`),
}));

// 静默 antd message（避免测试输出噪音）
vi.mock("antd", async (importOriginal) => {
  const actual = await importOriginal<typeof import("antd")>();
  return {
    ...actual,
    message: {
      ...actual.message,
      error: vi.fn(),
      success: vi.fn(),
      warning: vi.fn(),
    },
  };
});

import { videoApi } from "../services/api";
import { useWorkspaceStore } from "../stores/useWorkspaceStore";
import type { VideoInfo } from "../types";

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

describe("Library 页面", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // 重置工作区 store 到已知状态
    useWorkspaceStore.setState({
      workspaces: [{ id: 1, name: "测试", path: "C:\\test", createdAt: "", videoCount: 1, folderCount: 1 }],
      currentWorkspaceId: 1,
      loaded: true,
    });
    (videoApi.list as ReturnType<typeof vi.fn>).mockResolvedValue({
      total: 1,
      items: [mockVideo],
    });
  });

  it("有工作区且有视频时正常渲染列表，不发生无限循环", async () => {
    render(<Library />);
    // 等待数据加载完成
    await waitFor(() => {
      expect(screen.getByText("test.mp4")).toBeInTheDocument();
    });
    // 数据应只加载一次（无无限循环：若循环，list 调用次数会持续增长）
    expect(videoApi.list).toHaveBeenCalledTimes(1);
  });

  it("无工作区时显示空态提示", async () => {
    useWorkspaceStore.setState({ workspaces: [], currentWorkspaceId: null });
    render(<Library />);
    await waitFor(() => {
      expect(screen.getByText("请先在左侧「新增文件夹并扫描」创建工作区")).toBeInTheDocument();
    });
    expect(videoApi.list).not.toHaveBeenCalled();
  });
});
