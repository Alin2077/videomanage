import { create } from "zustand";
import { tagApi } from "../services/api";
import type { TagGroupWithTags, TagInput, TagGroupInput } from "../types";
import { message } from "antd";

interface TagState {
  tagGroups: TagGroupWithTags[];
  loaded: boolean;
  loadTags: () => Promise<void>;
  createTag: (tag: TagInput) => Promise<void>;
  updateTag: (tag: TagInput) => Promise<void>;
  deleteTag: (tagId: number) => Promise<void>;
  createGroup: (group: TagGroupInput) => Promise<void>;
  deleteGroup: (groupId: number) => Promise<void>;
}

export const useTagStore = create<TagState>((set, get) => ({
  tagGroups: [],
  loaded: false,

  async loadTags() {
    try {
      const groups = await tagApi.getTree();
      set({ tagGroups: groups, loaded: true });
    } catch (e) {
      message.error(`加载标签失败: ${e}`);
    }
  },

  async createTag(tag) {
    await tagApi.upsert(tag);
    await get().loadTags();
  },

  async updateTag(tag) {
    await tagApi.upsert(tag);
    await get().loadTags();
  },

  async deleteTag(tagId) {
    await tagApi.remove(tagId);
    await get().loadTags();
  },

  async createGroup(group) {
    await tagApi.upsertGroup(group);
    await get().loadTags();
  },

  async deleteGroup(groupId) {
    await tagApi.removeGroup(groupId);
    await get().loadTags();
  },
}));
