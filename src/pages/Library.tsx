import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Alert,
  Button,
  Dropdown,
  Empty,
  Input,
  Modal,
  Pagination,
  Popover,
  Segmented,
  Space,
  Spin,
  Table,
  Tag as AntTag,
  message,
} from "antd";
import {
  AppstoreOutlined,
  BarsOutlined,
  DeleteOutlined,
  EditOutlined,
  MoreOutlined,
  PlayCircleOutlined,
  TagsOutlined,
} from "@ant-design/icons";
import { tagApi, videoApi } from "../services/api";
import { useLibraryStore } from "../stores/useLibraryStore";
import { useScanStore } from "../stores/useScanStore";
import { useTagStore } from "../stores/useTagStore";
import { useWorkspaceStore } from "../stores/useWorkspaceStore";
import type { VideoInfo } from "../types";
import { formatDuration, formatSize, formatResolution } from "../utils/format";
import { useDebounce } from "../hooks/useDebounce";
import FolderTree from "../components/FolderTree";
import VideoCard from "../components/VideoCard";
import TagSelect from "../components/TagSelect";
import VideoPlayer from "../components/VideoPlayer";

export default function Library() {
  const store = useLibraryStore();
  const {
    selectedFolderId,
    viewMode,
    searchKeyword,
    filterTags,
    selectedVideoIds,
    currentVideos,
    total,
    page,
    pageSize,
    sortBy,
    sortOrder,
  } = store;

  const [loading, setLoading] = useState(false);
  const [playing, setPlaying] = useState<VideoInfo | null>(null);
  const [tagModalVideo, setTagModalVideo] = useState<VideoInfo | null>(null);
  const [editModalVideo, setEditModalVideo] = useState<VideoInfo | null>(null);
  const [editTitle, setEditTitle] = useState("");
  const [editNotes, setEditNotes] = useState("");
  const [batchTagOpen, setBatchTagOpen] = useState(false);
  const [batchTagIds, setBatchTagIds] = useState<number[]>([]);
  const [refreshKey] = useState(0);
  const scanProgress = useScanStore((s) => s.progress);
  const allTags = useTagStore((s) => s.tagGroups.flatMap((g) => g.tags));
  const currentWorkspaceId = useWorkspaceStore((s) => s.currentWorkspaceId);

  const debouncedSearch = useDebounce(searchKeyword, 350);

  const query = useMemo(
    () => ({
      workspaceId: currentWorkspaceId,
      folderId: selectedFolderId,
      keyword: debouncedSearch || null,
      tagIds: filterTags.length > 0 ? filterTags : null,
      page,
      pageSize,
      sortBy,
      sortOrder,
    }),
    [currentWorkspaceId, selectedFolderId, debouncedSearch, filterTags, page, pageSize, sortBy, sortOrder],
  );

  const load = useCallback(async () => {
    if (currentWorkspaceId === null) {
      store.setCurrentVideos([], 0);
      setLoading(false);
      return;
    }
    setLoading(true);
    try {
      const result = await videoApi.list(query);
      store.setCurrentVideos(result.items, result.total);
    } catch (e) {
      message.error(`加载视频失败: ${e}`);
    } finally {
      setLoading(false);
    }
  }, [query, currentWorkspaceId]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    load();
  }, [load, scanProgress.added, scanProgress.updated, scanProgress.unchanged, refreshKey]);

  const play = (v: VideoInfo) => setPlaying(v);

  const openExternal = async (v: VideoInfo) => {
    try {
      await videoApi.openWithPlayer(v.id, null);
      message.success(`已调用系统播放器打开「${v.fileName}」`);
    } catch (e) {
      message.error(`打开失败: ${e}`);
    }
  };

  const editMeta = async () => {
    if (!editModalVideo) return;
    try {
      await videoApi.updateMeta(editModalVideo.id, {
        customTitle: editTitle || null,
        notes: editNotes || null,
      });
      message.success("已保存");
      setEditModalVideo(null);
      load();
    } catch (e) {
      message.error(`保存失败: ${e}`);
    }
  };

  const batchTag = async () => {
    if (selectedVideoIds.length === 0 || batchTagIds.length === 0) return;
    try {
      await tagApi.batchAdd(selectedVideoIds, batchTagIds);
      message.success(`已为 ${selectedVideoIds.length} 个视频打标签`);
      setBatchTagOpen(false);
      setBatchTagIds([]);
      store.clearSelection();
      load();
    } catch (e) {
      message.error(`打标签失败: ${e}`);
    }
  };

  const batchDelete = () => {
    if (selectedVideoIds.length === 0) return;
    Modal.confirm({
      title: "删除视频记录",
      content: `确定删除选中的 ${selectedVideoIds.length} 条视频记录吗？（仅删除系统记录，不影响物理文件）`,
      okText: "删除",
      okType: "danger",
      onOk: async () => {
        try {
          await videoApi.batchDelete(selectedVideoIds);
          message.success("删除完成");
          store.clearSelection();
          load();
        } catch (e) {
          message.error(`删除失败: ${e}`);
        }
      },
    });
  };

  const contextMenuItems = (v: VideoInfo) => [
    { key: "play", label: "内嵌播放", icon: <PlayCircleOutlined />, onClick: () => play(v) },
    { key: "external", label: "系统播放器打开", icon: <PlayCircleOutlined />, onClick: () => openExternal(v) },
    { key: "tags", label: "管理标签", icon: <TagsOutlined />, onClick: () => setTagModalVideo(v) },
    { key: "edit", label: "编辑备注/标题", icon: <EditOutlined />, onClick: () => { setEditModalVideo(v); setEditTitle(v.customTitle || ""); setEditNotes(v.notes || ""); } },
    { type: "divider" as const },
    { key: "delete", label: "删除记录", icon: <DeleteOutlined />, danger: true, onClick: () => { store.setSelectedVideoIds([v.id]); batchDelete(); } },
  ];

  const columns = [
    {
      title: "文件名",
      dataIndex: "fileName",
      ellipsis: true,
      sorter: true,
      render: (_: unknown, v: VideoInfo) => (
        <Space>
          <span style={{ fontWeight: 500 }}>{v.customTitle || v.fileName}</span>
          {v.tags.slice(0, 2).map((t) => (
            <AntTag key={t.id} color={t.color} style={{ marginInlineEnd: 0 }}>{t.name}</AntTag>
          ))}
        </Space>
      ),
    },
    {
      title: "时长",
      dataIndex: "duration",
      width: 90,
      sorter: true,
      render: (d: number | null) => formatDuration(d),
    },
    {
      title: "分辨率",
      width: 110,
      render: (_: unknown, v: VideoInfo) => formatResolution(v.width, v.height),
    },
    {
      title: "编码",
      dataIndex: "codec",
      width: 90,
      render: (c: string | null) => c || "-",
    },
    {
      title: "大小",
      dataIndex: "fileSize",
      width: 100,
      sorter: true,
      render: (s: number) => formatSize(s),
    },
    {
      title: "观看",
      dataIndex: "openCount",
      width: 80,
      sorter: true,
      render: (n: number) => <AntTag color={n > 0 ? "blue" : "default"}>{n}</AntTag>,
    },
    {
      title: "修改时间",
      dataIndex: "modifiedAt",
      width: 160,
      sorter: true,
      render: (s: string) => s,
    },
    {
      title: "",
      width: 60,
      render: (_: unknown, v: VideoInfo) => (
        <Dropdown menu={{ items: contextMenuItems(v) }} trigger={["click"]}>
          <Button type="text" size="small" icon={<MoreOutlined />} />
        </Dropdown>
      ),
    },
  ];

  const sortMap: Record<string, { by: "name" | "size" | "duration" | "openCount" | "modifiedAt"; order: "asc" | "desc" }> = {
    fileName: { by: "name", order: "asc" },
    duration: { by: "duration", order: "desc" },
    fileSize: { by: "size", order: "desc" },
    openCount: { by: "openCount", order: "desc" },
    modifiedAt: { by: "modifiedAt", order: "desc" },
  };

  return (
    <div style={{ display: "flex", height: "100%" }}>
      {/* 左：文件夹树 */}
      <div style={{ width: 260, borderRight: "1px solid #f0f0f0", overflow: "auto" }}>
        <FolderTree
          selectedId={selectedFolderId}
          onSelect={store.setSelectedFolder}
          refreshKey={refreshKey}
          workspaceId={currentWorkspaceId}
        />
      </div>

      {/* 右：内容区 */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
        {/* 工具栏 */}
        <div style={{ padding: "10px 14px", borderBottom: "1px solid #f0f0f0", display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
          <Input.Search
            placeholder="搜索文件名 / 路径 / 备注 / 标签"
            allowClear
            value={searchKeyword}
            onChange={(e) => store.setSearch(e.target.value)}
            style={{ width: 300 }}
          />
          <Popover
            title="按标签筛选（AND）"
            trigger="click"
            content={
              <div style={{ width: 280 }}>
                <TagSelect
                  value={filterTags}
                  onChange={(ids) => {
                    store.clearFilterTags();
                    ids.forEach((id) => store.toggleFilterTag(id));
                  }}
                />
                <Button size="small" onClick={store.clearFilterTags} style={{ marginTop: 8 }}>
                  清除筛选
                </Button>
              </div>
            }
          >
            <Button icon={<TagsOutlined />}>
              标签筛选{filterTags.length > 0 ? ` (${filterTags.length})` : ""}
            </Button>
          </Popover>
          <Segmented
            value={viewMode}
            onChange={(v) => store.setViewMode(v as "list" | "grid")}
            options={[
              { value: "list", icon: <BarsOutlined /> },
              { value: "grid", icon: <AppstoreOutlined /> },
            ]}
          />
          <div style={{ flex: 1 }} />
          <span style={{ color: "#888", fontSize: 13 }}>共 {total} 个视频</span>
          {selectedVideoIds.length > 0 && (
            <Space>
              <Button size="small" type="primary" icon={<TagsOutlined />} onClick={() => setBatchTagOpen(true)}>
                打标签 ({selectedVideoIds.length})
              </Button>
              <Button size="small" danger icon={<DeleteOutlined />} onClick={batchDelete}>
                删除
              </Button>
            </Space>
          )}
        </div>

        {scanProgress.errors.length > 0 && !scanProgress.isScanning && (
          <Alert
            type="warning"
            showIcon
            closable
            message={`上次扫描有 ${scanProgress.errors.length} 个文件提取失败（未安装 ffmpeg 时属正常）`}
            style={{ margin: 8 }}
          />
        )}

        {/* 列表 / 网格 */}
        <div style={{ flex: 1, overflow: "auto" }}>
          {loading ? (
            <div style={{ textAlign: "center", padding: 60 }}><Spin /></div>
          ) : currentVideos.length === 0 ? (
            <Empty
              style={{ marginTop: 80 }}
              description={
                currentWorkspaceId === null
                  ? "请先在左侧「新增文件夹并扫描」创建工作区"
                  : "该工作区暂无视频，可点击侧边栏右上角重新扫描"
              }
            />
          ) : viewMode === "grid" ? (
            <div className="video-grid">
              {currentVideos.map((v) => (
                <VideoCard
                  key={v.id}
                  video={v}
                  selected={selectedVideoIds.includes(v.id)}
                  onClick={(video) => store.toggleVideoSelect(video.id)}
                  onPlay={play}
                />
              ))}
            </div>
          ) : (
            <Table
              rowKey="id"
              size="small"
              dataSource={currentVideos}
              columns={columns}
              pagination={false}
              rowSelection={{
                selectedRowKeys: selectedVideoIds,
                onChange: (keys) => store.setSelectedVideoIds(keys as number[]),
              }}
              onRow={(v) => ({
                onDoubleClick: () => play(v),
              })}
              onChange={(_p, _f, sorter) => {
                if (Array.isArray(sorter)) return;
                const field = sorter.field as string;
                const map = sortMap[field];
                if (map) {
                  store.setSort(map.by, sorter.order === "ascend" ? "asc" : "desc");
                }
              }}
              scroll={{ x: 900 }}
            />
          )}
        </div>

        {/* 分页 */}
        <div style={{ padding: 10, borderTop: "1px solid #f0f0f0", display: "flex", justifyContent: "flex-end" }}>
          <Pagination
            current={page}
            pageSize={pageSize}
            total={total}
            showSizeChanger
            pageSizeOptions={[20, 50, 100]}
            onChange={(p, ps) => {
              if (ps !== pageSize) store.setPageSize(ps);
              store.setPage(p);
            }}
            showTotal={(t) => `共 ${t} 条`}
          />
        </div>
      </div>

      {/* 播放器 */}
      {playing && <VideoPlayer video={playing} onClose={() => setPlaying(null)} />}

      {/* 单视频打标签 */}
      <Modal
        title={`管理标签 - ${tagModalVideo?.fileName ?? ""}`}
        open={!!tagModalVideo}
        onCancel={() => setTagModalVideo(null)}
        onOk={async () => {
          if (!tagModalVideo) return;
          try {
            await tagApi.setVideoTags(tagModalVideo.id, tagModalVideo.tags.map((t) => t.id));
            message.success("标签已更新");
            setTagModalVideo(null);
            load();
          } catch (e) {
            message.error(`更新失败: ${e}`);
          }
        }}
        destroyOnClose
      >
        {tagModalVideo && (
          <TagSelect
            value={tagModalVideo.tags.map((t) => t.id)}
            onChange={(ids) => {
              const current = tagModalVideo;
              setTagModalVideo({
                ...current,
                tags: ids.map((id) => allTags.find((t) => t.id === id) || { id, name: `#${id}`, color: "#999", groupId: null }),
              });
            }}
          />
        )}
      </Modal>

      {/* 批量打标签 */}
      <Modal
        title={`为 ${selectedVideoIds.length} 个视频打标签`}
        open={batchTagOpen}
        onCancel={() => { setBatchTagOpen(false); setBatchTagIds([]); }}
        onOk={batchTag}
        okText="应用标签"
        destroyOnClose
      >
        <TagSelect value={batchTagIds} onChange={setBatchTagIds} />
      </Modal>

      {/* 编辑备注 */}
      <Modal
        title="编辑视频信息"
        open={!!editModalVideo}
        onCancel={() => setEditModalVideo(null)}
        onOk={editMeta}
        okText="保存"
        destroyOnClose
      >
        <Space direction="vertical" style={{ width: "100%" }}>
          <Input
            placeholder="自定义标题（不修改物理文件）"
            value={editTitle}
            onChange={(e) => setEditTitle(e.target.value)}
          />
          <Input.TextArea
            placeholder="备注"
            rows={4}
            value={editNotes}
            onChange={(e) => setEditNotes(e.target.value)}
          />
        </Space>
      </Modal>
    </div>
  );
}
