import { create } from "zustand";

interface DataVersionState {
  /** 视频库数据版本（扫描/树刷新/增删改后递增，触发列表与统计页刷新） */
  libraryVersion: number;
  /** 观看日志版本（播放开始/结束、日志操作后递增，触发日志与统计页刷新） */
  logsVersion: number;
  bumpLibrary: () => void;
  bumpLogs: () => void;
}

/** 全局数据刷新联动：任何数据变更后递增版本号，页面 useEffect 依赖版本号实现即时刷新 */
export const useDataVersionStore = create<DataVersionState>((set) => ({
  libraryVersion: 0,
  logsVersion: 0,
  bumpLibrary: () => set((s) => ({ libraryVersion: s.libraryVersion + 1 })),
  bumpLogs: () => set((s) => ({ logsVersion: s.logsVersion + 1 })),
}));
