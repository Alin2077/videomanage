import { create } from "zustand";
import { message } from "antd";
import { workspaceApi } from "../services/api";
import { useScanStore } from "./useScanStore";
import type { Workspace } from "../types";

interface WorkspaceState {
  workspaces: Workspace[];
  currentWorkspaceId: number | null;
  loaded: boolean;
  load: () => Promise<void>;
  /** 新增工作区并开始扫描，返回新工作区 id（成功时自动切换过去） */
  addWorkspace: (path: string, name?: string) => Promise<number | null>;
  removeWorkspace: (id: number) => Promise<void>;
  switchWorkspace: (id: number) => void;
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  workspaces: [],
  currentWorkspaceId: null,
  loaded: false,

  async load() {
    try {
      const workspaces = await workspaceApi.list();
      // 保持当前选中（若被删除则回退到第一个）
      let current = get().currentWorkspaceId;
      if (!workspaces.some((w) => w.id === current)) {
        current = workspaces.length > 0 ? workspaces[0].id : null;
      }
      set({ workspaces, currentWorkspaceId: current, loaded: true });
    } catch (e) {
      message.error(`加载工作区失败: ${e}`);
    }
  },

  async addWorkspace(path, name) {
    const result = await useScanStore.getState().startScan(path, name || "");
    if (result) {
      set({ currentWorkspaceId: result.workspaceId });
      await get().load();
      return result.workspaceId;
    }
    return null;
  },

  async removeWorkspace(id) {
    try {
      await workspaceApi.remove(id);
      await get().load();
    } catch (e) {
      message.error(`删除工作区失败: ${e}`);
    }
  },

  switchWorkspace(id) {
    set({ currentWorkspaceId: id });
  },
}));
