import { create } from "zustand";
import { settingsApi } from "../services/api";
import { message } from "antd";

interface SettingsState {
  settings: Record<string, string>;
  loaded: boolean;
  load: () => Promise<void>;
  set: (key: string, value: string) => Promise<void>;
  get: (key: string) => string | undefined;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: {},
  loaded: false,

  async load() {
    try {
      const s = await settingsApi.getAll();
      set({ settings: s, loaded: true });
    } catch (e) {
      message.error(`加载设置失败: ${e}`);
    }
  },

  async set(key, value) {
    try {
      await settingsApi.set(key, value);
      set((s) => ({ settings: { ...s.settings, [key]: value } }));
    } catch (e) {
      message.error(`保存设置失败: ${e}`);
      throw e;
    }
  },

  get: (key) => get().settings[key],
}));
