import { useEffect } from "react";
import { Outlet, useLocation, useNavigate } from "react-router-dom";
import { Layout, Menu, Button, Progress, Tag as AntTag, Space, Tooltip } from "antd";
import {
  DashboardOutlined,
  FolderOpenOutlined,
  BarChartOutlined,
  TagsOutlined,
  SettingOutlined,
  PlayCircleOutlined,
  FolderAddOutlined,
  CloseCircleOutlined,
} from "@ant-design/icons";
import { open } from "@tauri-apps/plugin-dialog";
import { useScanStore } from "../../stores/useScanStore";
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
  const { progress, startScan, cancelScan, refreshProgress } = useScanStore();
  const setSelectedFolder = useLibraryStore((s) => s.setSelectedFolder);

  useEffect(() => {
    if (progress.isScanning) {
      const timer = setInterval(refreshProgress, 600);
      return () => clearInterval(timer);
    }
  }, [progress.isScanning, refreshProgress]);

  const pickFolder = async () => {
    const dir = await open({ directory: true, multiple: false, title: "选择视频库根目录" });
    if (typeof dir === "string") {
      setSelectedFolder(null);
      const result = await startScan(dir);
      if (result) {
        // 扫描完成后刷新
        refreshProgress();
      }
    }
  };

  const selectedKey =
    menuItems.find((m) => location.pathname.startsWith(m.key))?.key || "/dashboard";

  return (
    <Layout style={{ height: "100%" }}>
      <Sider width={200} theme="light" style={{ borderRight: "1px solid #f0f0f0" }}>
        <div style={{ padding: "16px 16px 8px", fontSize: 16, fontWeight: 700 }}>
          🎬 视频管理
        </div>
        <Menu
          mode="inline"
          selectedKeys={[selectedKey]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
          style={{ borderInlineEnd: "none" }}
        />
        <div style={{ padding: 12 }}>
          <Button
            type="primary"
            block
            icon={<FolderAddOutlined />}
            onClick={pickFolder}
            disabled={progress.isScanning}
          >
            {progress.isScanning ? "扫描中..." : "选择文件夹并扫描"}
          </Button>
        </div>
      </Sider>
      <Layout>
        <Header
          style={{
            background: "var(--ant-color-bg-container, #fff)",
            borderBottom: "1px solid #f0f0f0",
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
            <span style={{ fontWeight: 500 }}>本地视频资产库</span>
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
    </Layout>
  );
}
