import { create } from "zustand";
import { scanApi } from "../services/api";
import type { ScanProgress, ScanResult } from "../types";
import { message } from "antd";

interface ScanState {
  progress: ScanProgress;
  startScan: (path: string, name?: string) => Promise<ScanResult | null>;
  cancelScan: () => Promise<void>;
  refreshProgress: () => Promise<void>;
  poll: () => void;
  stopPoll: () => void;
  _timer: ReturnType<typeof setInterval> | null;
}

const initialProgress: ScanProgress = {
  isScanning: false,
  progress: 0,
  currentPath: "",
  scannedFiles: 0,
  totalFiles: 0,
  added: 0,
  updated: 0,
  unchanged: 0,
  errors: [],
};

export const useScanStore = create<ScanState>((set, get) => ({
  progress: initialProgress,
  _timer: null,

  async refreshProgress() {
    try {
      const p = await scanApi.getScanProgress();
      set({ progress: p });
    } catch {
      /* ignore */
    }
  },

  poll() {
    get().stopPoll();
    const timer = setInterval(() => get().refreshProgress(), 500);
    set({ _timer: timer });
  },

  stopPoll() {
    const t = get()._timer;
    if (t) {
      clearInterval(t);
      set({ _timer: null });
    }
  },

  async startScan(path, name) {
    get().stopPoll();
    set({ progress: { ...initialProgress, isScanning: true } });
    get().poll();
    try {
      const result = await scanApi.scanRootFolder(path, name || "");
      await get().refreshProgress();
      get().stopPoll();
      if (result.errors.length > 0) {
        message.warning(`扫描完成：新增 ${result.added}，更新 ${result.updated}，${result.errors.length} 个文件提取失败`);
      } else {
        message.success(`扫描完成：新增 ${result.added}，更新 ${result.updated}，未变化 ${result.unchanged}`);
      }
      return result;
    } catch (e) {
      get().stopPoll();
      message.error(`扫描失败: ${e}`);
      return null;
    }
  },

  async cancelScan() {
    try {
      await scanApi.cancelScan();
      message.info("已请求取消扫描");
    } catch (e) {
      message.error(`取消失败: ${e}`);
    }
  },
}));
