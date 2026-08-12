import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Divider,
  Form,
  Input,
  Segmented,
  Space,
  Switch,
  Tag as AntTag,
  message,
} from "antd";
import {
  DatabaseOutlined,
  DownloadOutlined,
  ExperimentOutlined,
  PlayCircleOutlined,
  SearchOutlined,
  UploadOutlined,
  GlobalOutlined,
} from "@ant-design/icons";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { settingsApi } from "../services/api";
import { useSettingsStore } from "../stores/useSettingsStore";
import { useWorkspaceStore } from "../stores/useWorkspaceStore";

export default function Settings() {
  const { settings, set, load } = useSettingsStore();
  const workspaces = useWorkspaceStore((s) => s.workspaces);
  const [form] = Form.useForm();
  const [tools, setTools] = useState<{ ffprobe: string | null; ffmpeg: string | null } | null>(null);

  useEffect(() => {
    form.setFieldsValue({
      player_path: settings.player_path || "",
      ffprobe_path: settings.ffprobe_path || "",
      ffmpeg_path: settings.ffmpeg_path || "",
      ignore_folders: settings.ignore_folders || ".git, node_modules, $RECYCLE.BIN, System Volume Information",
      compute_hash: settings.compute_hash === "1",
    });
  }, [settings, form]);

  const saveField = async (key: string, value: string) => {
    try {
      await set(key, value);
    } catch {
      /* 错误已提示 */
    }
  };

  const detect = async () => {
    const result = await settingsApi.detectMediaTools();
    setTools(result);
    if (result.ffprobe || result.ffmpeg) {
      if (result.ffprobe) await saveField("ffprobe_path", result.ffprobe);
      if (result.ffmpeg) await saveField("ffmpeg_path", result.ffmpeg);
      message.success("已探测并保存媒体工具路径");
      load();
    } else {
      message.warning("未检测到 ffmpeg/ffprobe。请安装 ffmpeg 并加入 PATH，或手动指定路径");
    }
  };

  const pickPath = async (key: string) => {
    const file = await open({
      multiple: false,
      title: "选择可执行文件",
      filters: [{ name: "程序", extensions: ["exe", "bat", "cmd"] }],
    });
    if (typeof file === "string") {
      await saveField(key, file);
      load();
    }
  };

  const exportBackup = async () => {
    try {
      const filePath = await save({
        defaultPath: `视频库备份_${new Date().toISOString().slice(0, 10)}.vfm-backup`,
        filters: [{ name: "视频库备份", extensions: ["vfm-backup"] }],
      });
      if (!filePath) return;
      await settingsApi.exportBackup(filePath);
      message.success(`备份已导出到 ${filePath}`);
    } catch (e) {
      message.error(`导出失败: ${e}`);
    }
  };

  const importBackup = async () => {
    try {
      const filePath = await open({
        multiple: false,
        title: "选择备份文件",
        filters: [{ name: "视频库备份", extensions: ["vfm-backup"] }],
      });
      if (!filePath) return;
      await settingsApi.importBackup(filePath);
      message.success("备份已导入，重启应用后生效");
    } catch (e) {
      message.error(`导入失败: ${e}`);
    }
  };

  const downloadFfmpeg = (url: string) => {
    openUrl(url).catch(() => message.error("无法打开浏览器，请手动访问 " + url));
  };

  return (
    <div className="page-container">
      <div className="page-title">设置</div>

      <Card size="small" title={<Space><PlayCircleOutlined /> 外观与行为</Space>} style={{ maxWidth: 720, marginBottom: 14 }}>
        <Space direction="vertical" size={12} style={{ width: "100%" }}>
          <Space>
            <span>主题：</span>
            <Segmented
              value={settings.theme === "dark" ? "dark" : "light"}
              onChange={(v) => saveField("theme", v as string)}
              options={[
                { label: "☀️ 亮色", value: "light" },
                { label: "🌙 暗色", value: "dark" },
              ]}
            />
          </Space>
          <Space>
            <span>点击关闭按钮时最小化到系统托盘：</span>
            <Switch
              checked={settings.close_to_tray === "1"}
              onChange={(v) => saveField("close_to_tray", v ? "1" : "0")}
            />
            <span style={{ color: "#999", fontSize: 12 }}>
              开启后关闭窗口将驻留右下角托盘，可从托盘菜单退出
            </span>
          </Space>
          <Space>
            <span>播放器快进/快退步长（秒）：</span>
            <Input
              type="number"
              min={0.5}
              step={0.5}
              style={{ width: 100 }}
              defaultValue={settings.seek_step || "5"}
              onBlur={(e) => {
                const v = parseFloat(e.target.value);
                saveField("seek_step", Number.isFinite(v) && v > 0 ? String(v) : "5");
              }}
            />
            <span style={{ color: "#999", fontSize: 12 }}>
              播放时按 ← / → 键按此步长快退/快进（如 0.5 / 1 / 2 秒）
            </span>
          </Space>
        </Space>
      </Card>

      <Card
        size="small"
        title={<Space><ExperimentOutlined /> 媒体工具（ffmpeg / ffprobe）</Space>}
        extra={
          <Space>
            <Button size="small" icon={<GlobalOutlined />} onClick={() => downloadFfmpeg("https://www.gyan.dev/ffmpeg/builds/")}>
              下载 ffmpeg (Windows)
            </Button>
            <Button size="small" type="primary" icon={<SearchOutlined />} onClick={detect}>
              自动检测
            </Button>
          </Space>
        }
        style={{ maxWidth: 720, marginBottom: 14 }}
      >
        <Alert
          type="info"
          showIcon
          style={{ marginBottom: 12 }}
          message={
            <>
              安装 ffmpeg（包含 ffprobe）后，扫描时自动提取分辨率/时长/编码/帧率并生成封面。未安装时仅记录文件信息，功能不受影响。其他平台下载：{" "}
              <a onClick={() => downloadFfmpeg("https://ffmpeg.org/download.html")}>ffmpeg.org</a>
            </>
          }
        />
        <Form form={form} layout="vertical">
          <Space direction="vertical" style={{ width: "100%" }} size={8}>
            <Space>
              <span style={{ width: 90 }}>ffprobe 路径</span>
              <Form.Item name="ffprobe_path" noStyle>
                <Input
                  style={{ width: 380 }}
                  placeholder="留空则自动查找 PATH"
                  onBlur={(e) => saveField("ffprobe_path", e.target.value)}
                />
              </Form.Item>
              <Button size="small" onClick={() => pickPath("ffprobe_path")}>浏览…</Button>
            </Space>
            <Space>
              <span style={{ width: 90 }}>ffmpeg 路径</span>
              <Form.Item name="ffmpeg_path" noStyle>
                <Input
                  style={{ width: 380 }}
                  placeholder="留空则自动查找 PATH"
                  onBlur={(e) => saveField("ffmpeg_path", e.target.value)}
                />
              </Form.Item>
              <Button size="small" onClick={() => pickPath("ffmpeg_path")}>浏览…</Button>
            </Space>
          </Space>
        </Form>
        {(tools?.ffprobe || tools?.ffmpeg) && (
          <div style={{ marginTop: 8 }}>
            {tools?.ffprobe && <AntTag color="green">✓ ffprobe: {tools.ffprobe}</AntTag>}
            {tools?.ffmpeg && <AntTag color="green" style={{ marginLeft: 8 }}>✓ ffmpeg: {tools.ffmpeg}</AntTag>}
          </div>
        )}
      </Card>

      <Card size="small" title={<Space><PlayCircleOutlined /> 播放</Space>} style={{ maxWidth: 720, marginBottom: 14 }}>
        <Form form={form} layout="vertical">
          <Space>
            <span>外部播放器路径（可选）</span>
            <Form.Item name="player_path" noStyle>
              <Input
                style={{ width: 380 }}
                placeholder="留空则使用系统默认播放器"
                onBlur={(e) => saveField("player_path", e.target.value)}
              />
            </Form.Item>
            <Button size="small" onClick={() => pickPath("player_path")}>浏览…</Button>
          </Space>
        </Form>
      </Card>

      <Card size="small" title={<Space><DatabaseOutlined /> 扫描与数据</Space>} style={{ maxWidth: 720, marginBottom: 14 }}>
        <Space direction="vertical" size={12} style={{ width: "100%" }}>
          <Space>
            <span style={{ width: 90 }}>忽略文件夹</span>
            <Input
              style={{ width: 440 }}
              defaultValue={settings.ignore_folders || ".git, node_modules, $RECYCLE.BIN, System Volume Information"}
              onBlur={(e) => saveField("ignore_folders", e.target.value)}
              placeholder="逗号分隔，如：.git, node_modules"
            />
          </Space>
          <Space>
            <span style={{ width: 90 }}>计算文件哈希</span>
            <Switch
              checked={settings.compute_hash === "1"}
              onChange={(v) => saveField("compute_hash", v ? "1" : "0")}
            />
            <span style={{ color: "#999", fontSize: 12 }}>用于重复视频检测，扫描速度会变慢</span>
          </Space>
          <Divider style={{ margin: "4px 0" }} />
          <div>
            <div style={{ marginBottom: 6 }}>工作区列表（在左侧边栏新增/切换/删除）</div>
            {workspaces.length === 0 ? (
              <span style={{ color: "#999", fontSize: 12 }}>暂无工作区</span>
            ) : (
              <Space direction="vertical" size={4}>
                {workspaces.map((w) => (
                  <Space key={w.id} size={8}>
                    <AntTag color="blue">{w.name}</AntTag>
                    <span style={{ color: "#888", fontSize: 12 }}>{w.path}</span>
                    <span style={{ color: "#bbb", fontSize: 12 }}>{w.videoCount} 个视频</span>
                  </Space>
                ))}
              </Space>
            )}
          </div>
          <Space>
            <Button icon={<DownloadOutlined />} onClick={exportBackup}>导出备份 (.vfm-backup)</Button>
            <Button icon={<UploadOutlined />} onClick={importBackup}>导入备份</Button>
            <span style={{ color: "#999", fontSize: 12 }}>备份包含数据库与封面缓存，导入后需重启生效</span>
          </Space>
        </Space>
      </Card>
    </div>
  );
}
