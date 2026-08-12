import { useEffect, useState } from "react";
import { Outlet, useLocation, useNavigate } from "react-router-dom";
import {
  Layout,
  Menu,
  Button,
  Progress,
  Tag as AntTag,
  Space,
  Tooltip,
  Modal,
  Input,
  Popconfirm,
  Empty,
} from "antd";
import {
  DashboardOutlined,
  FolderOpenOutlined,
  BarChartOutlined,
  TagsOutlined,
  SettingOutlined,
  PlayCircleOutlined,
  FolderAddOutlined,
  CloseCircleOutlined,
  DeleteOutlined,
} from "@ant-design/icons";
import { open } from "@tauri-apps/plugin-dialog";
import { useScanStore } from "../../stores/useScanStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { useLibraryStore } from "../../stores/useLibraryStore";

const { Sider, Header, Content } = Layout;

const menuItems = [
  { key: "/dashboard", icon: <DashboardOutlined />, label: "仪表盘" },
  { key: "/library", icon: <FolderOpenOutlined />, label: "视频库" },
  { key: "/statistics", icon: <BarChartOutlined />, label: "统计分析" },
  { key: "/tags", icon: <TagsOutlined />, label: "标签管理" },
  { key: "/settings", icon: <SettingOutlined />, label: "设置" },
];

export default function MainLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const { progress, cancelScan, refreshProgress } = useScanStore();
  const { workspaces, currentWorkspaceId, load, addWorkspace, removeWorkspace, switchWorkspace } =
    useWorkspaceStore();
  const setSelectedFolder = useLibraryStore((s) => s.setSelectedFolder);

  const [namingOpen, setNamingOpen] = useState(false);
  const [pickedPath, setPickedPath] = useState("");
  const [wsName, setWsName] = useState("");

  // 首次加载工作区
  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 扫描进度轮询
  useEffect(() => {
    if (progress.isScanning) {
      const timer = setInterval(refreshProgress, 600);
      return () => clearInterval(timer);
    }
  }, [progress.isScanning, refreshProgress]);

  const pickFolder = async () => {
    const dir = await open({ directory: true, multiple: false, title: "选择视频库文件夹" });
    if (typeof dir === "string") {
      setPickedPath(dir);
      const defaultName =
        dir.split(/[\\/]/).filter(Boolean).pop() || "新工作区";
      setWsName(defaultName);
      setNamingOpen(true);
    }
  };

  const confirmAdd = async () => {
    setNamingOpen(false);
    setSelectedFolder(null);
    await addWorkspace(pickedPath, wsName);
  };

  const currentWorkspace = workspaces.find((w) => w.id === currentWorkspaceId);

  const selectedKey =
    menuItems.find((m) => location.pathname.startsWith(m.key))?.key || "/dashboard";

  return (
    <Layout style={{ height: "100%" }}>
      <Sider
        width={230}
        theme="light"
        style={{ borderRight: "1px solid var(--ant-color-border-secondary, #f0f0f0)" }}
      >
        <div style={{ padding: "16px 16px 8px", fontSize: 16, fontWeight: 700 }}>
          🎬 视频管理
        </div>

        {/* 工作区区块 */}
        <div style={{ padding: "0 8px 8px" }}>
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              padding: "4px 8px",
              fontSize: 12,
              color: "var(--ant-color-text-tertiary, #999)",
            }}
          >
            <span>工作区</span>
            <Button
              size="small"
              type="text"
              icon={<FolderAddOutlined />}
              onClick={pickFolder}
              disabled={progress.isScanning}
              title="新增文件夹并扫描"
            />
          </div>

          {workspaces.length === 0 ? (
            <div style={{ padding: "8px 10px" }}>
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description="尚无工作区"
                style={{ marginBottom: 4 }}
              />
              <Button type="primary" block icon={<FolderAddOutlined />} onClick={pickFolder}>
                新增文件夹并扫描
              </Button>
            </div>
          ) : (
            <div style={{ maxHeight: 240, overflow: "auto" }}>
              {workspaces.map((w) => (
                <div
                  key={w.id}
                  onClick={() => {
                    switchWorkspace(w.id);
                    setSelectedFolder(null);
                  }}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "6px 8px",
                    borderRadius: 6,
                    cursor: "pointer",
                    marginBottom: 2,
                    background:
                      w.id === currentWorkspaceId
                        ? "var(--ant-color-primary-bg, #e6f4ff)"
                        : "transparent",
                  }}
                >
                  <Tooltip title={w.path} placement="right">
                    <span
                      style={{
                        flex: 1,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                        fontSize: 13,
                        fontWeight: w.id === currentWorkspaceId ? 600 : 400,
                        color: "var(--ant-color-text, #333)",
                      }}
                    >
                      {w.name}
                      <span style={{ color: "#999", fontSize: 11, marginLeft: 6 }}>
                        {w.videoCount}
                      </span>
                    </span>
                  </Tooltip>
                  <Popconfirm
                    title="删除该工作区？"
                    description="将删除其视频记录（不影响磁盘文件）。"
                    okText="删除"
                    okType="danger"
                    onConfirm={() => removeWorkspace(w.id)}
                  >
                    <Button
                      size="small"
                      type="text"
                      danger
                      icon={<DeleteOutlined />}
                      onClick={(e) => e.stopPropagation()}
                    />
                  </Popconfirm>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* 当前工作区路径 */}
        {currentWorkspace && (
          <Tooltip title={currentWorkspace.path} placement="right">
            <div
              style={{
                margin: "0 12px 10px",
                padding: "6px 10px",
                borderRadius: 6,
                fontSize: 12,
                background: "var(--ant-color-fill-secondary, #f5f5f5)",
                color: "var(--ant-color-text-secondary, #666)",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              📁 {currentWorkspace.path}
            </div>
          </Tooltip>
        )}

        <Menu
          mode="inline"
          selectedKeys={[selectedKey]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
          style={{ borderInlineEnd: "none" }}
        />
      </Sider>
      <Layout>
        <Header
          style={{
            background: "var(--ant-color-bg-container, #fff)",
            borderBottom: "1px solid var(--ant-color-border-secondary, #f0f0f0)",
            height: 48,
            lineHeight: "48px",
            padding: "0 16px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <Space size="middle">
            <PlayCircleOutlined style={{ fontSize: 18, color: "#4a7dff" }} />
            <span style={{ fontWeight: 500 }}>
              本地视频资产库{currentWorkspace ? ` · ${currentWorkspace.name}` : ""}
            </span>
          </Space>
          {progress.isScanning ? (
            <Space size="middle" style={{ width: "46%" }}>
              <Progress
                percent={Math.round(progress.progress)}
                size="small"
                style={{ flex: 1, margin: 0 }}
              />
              <Tooltip title={progress.currentPath} placement="bottom">
                <span style={{ fontSize: 12, color: "#888", maxWidth: 260, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {progress.scannedFiles}/{progress.totalFiles}
                </span>
              </Tooltip>
              <Button size="small" icon={<CloseCircleOutlined />} onClick={cancelScan}>
                取消
              </Button>
            </Space>
          ) : (
            <Space>
              {progress.added > 0 && <AntTag color="green">新增 {progress.added}</AntTag>}
              {progress.updated > 0 && <AntTag color="blue">更新 {progress.updated}</AntTag>}
            </Space>
          )}
        </Header>
        <Content style={{ overflow: "auto" }}>
          <Outlet />
        </Content>
      </Layout>

      {/* 新增工作区命名 */}
      <Modal
        title="新增文件夹并扫描"
        open={namingOpen}
        onCancel={() => setNamingOpen(false)}
        onOk={confirmAdd}
        okText="扫描"
        destroyOnClose
      >
        <Space direction="vertical" style={{ width: "100%" }}>
          <div style={{ fontSize: 13, color: "#888" }}>
            文件夹：<span style={{ color: "var(--ant-color-text, #333)" }}>{pickedPath}</span>
          </div>
          <Input
            placeholder="工作区名称"
            value={wsName}
            onChange={(e) => setWsName(e.target.value)}
            onPressEnter={confirmAdd}
          />
        </Space>
      </Modal>
    </Layout>
  );
}
