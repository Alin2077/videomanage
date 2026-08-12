import { create } from "zustand";
import type { VideoInfo } from "../types";

export type ViewMode = "list" | "grid" | "detail";

interface LibraryState {
  selectedFolderId: number | null;
  viewMode: ViewMode;
  searchKeyword: string;
  filterTags: number[]; // AND 逻辑筛选
  selectedVideoIds: number[];
  currentVideos: VideoInfo[];
  total: number;
  page: number;
  pageSize: number;
  sortBy: "name" | "size" | "duration" | "openCount" | "modifiedAt";
  sortOrder: "asc" | "desc";
  setSelectedFolder: (id: number | null) => void;
  setViewMode: (mode: ViewMode) => void;
  setSearch: (keyword: string) => void;
  toggleFilterTag: (tagId: number) => void;
  clearFilterTags: () => void;
  setSelectedVideoIds: (ids: number[]) => void;
  toggleVideoSelect: (id: number) => void;
  clearSelection: () => void;
  setCurrentVideos: (videos: VideoInfo[], total: number) => void;
  setPage: (page: number) => void;
  setPageSize: (size: number) => void;
  setSort: (by: LibraryState["sortBy"], order: LibraryState["sortOrder"]) => void;
}

export const useLibraryStore = create<LibraryState>((set) => ({
  selectedFolderId: null,
  viewMode: (localStorage.getItem("vm_view_mode") as ViewMode) || "list",
  searchKeyword: "",
  filterTags: [],
  selectedVideoIds: [],
  currentVideos: [],
  total: 0,
  page: 1,
  pageSize: 50,
  sortBy: "modifiedAt",
  sortOrder: "desc",
  setSelectedFolder: (id) => set({ selectedFolderId: id, page: 1, selectedVideoIds: [] }),
  setViewMode: (mode) => {
    localStorage.setItem("vm_view_mode", mode);
    set({ viewMode: mode });
  },
  setSearch: (keyword) => set({ searchKeyword: keyword, page: 1 }),
  toggleFilterTag: (tagId) =>
    set((s) => {
      const has = s.filterTags.includes(tagId);
      return { filterTags: has ? s.filterTags.filter((t) => t !== tagId) : [...s.filterTags, tagId], page: 1 };
    }),
  clearFilterTags: () => set({ filterTags: [], page: 1 }),
  setSelectedVideoIds: (ids) => set({ selectedVideoIds: ids }),
  toggleVideoSelect: (id) =>
    set((s) => ({
      selectedVideoIds: s.selectedVideoIds.includes(id)
        ? s.selectedVideoIds.filter((x) => x !== id)
        : [...s.selectedVideoIds, id],
    })),
  clearSelection: () => set({ selectedVideoIds: [] }),
  setCurrentVideos: (videos, total) => set({ currentVideos: videos, total }),
  setPage: (page) => set({ page }),
  setPageSize: (size) => set({ pageSize: size, page: 1 }),
  setSort: (by, order) => set({ sortBy: by, sortOrder: order, page: 1 }),
}));
