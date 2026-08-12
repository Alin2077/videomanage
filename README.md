# 视频文件管理系统（Video File Manager）

轻量级、跨平台桌面端视频文件管理系统。基于 **Tauri 2.x + Rust + React + TypeScript + SQLite**，安装包约 5MB、内存占用低、数据全部本地存储。

## 功能一览

| 模块 | 说明 |
| --- | --- |
| 📁 文件扫描与树状结构 | 原生文件夹选择、递归扫描、基于 mtime 的增量更新、懒加载文件夹树、扫描进度/取消 |
| 🎬 视频播放与日志 | 内嵌播放器（HTML5）或外部播放器；自动记录打开/关闭时间与观看时长；异常关闭自动修复；CSV 导出 |
| 📊 统计图表与排行榜 | 仪表盘总览、日/周/月观看趋势（双 Y 轴）、最多打开/最长观看/最近活跃 TOP10、标签分布饼图、7×24 观看热力图 |
| 🏷️ 标签管理 | 标签组 + 标签 CRUD、颜色属性、视频多标签、AND 组合筛选、批量打标签 |
| 📄 文件管理 | 元信息编辑（备注/自定义标题）、批量删除、全局搜索、列表/网格双视图 |
| 🛡️ 数据安全 | 数据库本地化存储（SQLite WAL）、一键备份/恢复（.vfm-backup 包） |
| ⚙️ 其他 | 暗色/亮色主题、忽略文件夹配置、可配置 ffmpeg/ffprobe 路径、外部播放器路径 |

## 技术栈

- **桌面框架**: Tauri 2.x
- **后端**: Rust（rusqlite + walkdir + serde + chrono + tar）
- **前端**: React 18 + TypeScript + Vite + Ant Design 5 + Zustand + ECharts + react-router
- **存储**: SQLite（数据库位于 `%APPDATA%/video-manager/videos.db`，封面缓存于 `%APPDATA%/video-manager/covers/`）

## 开发环境要求

- Node.js ≥ 18
- Rust（stable）
- Windows 需 WebView2 运行时（Win10/11 一般自带）
- **（可选）ffmpeg/ffprobe**：安装并加入 PATH 后，扫描时自动提取分辨率/时长/编码/帧率并生成视频封面；未安装时仅索引文件信息，不影响其他功能

## 快速开始

```bash
npm install
npm run tauri dev        # 开发模式
```

## 构建安装包

```bash
npm run tauri build      # 产物位于 src-tauri/target/release/bundle/
```

## 目录结构

```
src/                     # 前端
├── components/          # 通用组件（布局/文件夹树/视频卡片/标签选择/播放器）
├── pages/               # 页面（仪表盘/视频库/统计分析/标签管理/设置）
├── stores/              # Zustand 状态
├── services/api.ts      # Tauri 命令封装
├── types/               # TypeScript 类型
├── hooks/ utils/        # 自定义 Hook 与工具函数
src-tauri/               # Rust 后端
├── src/
│   ├── main.rs lib.rs   # 入口与命令注册
│   ├── db.rs            # SQLite 初始化/迁移/备份应用
│   ├── scanner.rs       # 递归扫描 + 增量更新 + 进度/取消
│   ├── metadata.rs      # ffprobe/ffmpeg 元数据提取与封面生成
│   ├── models.rs        # 数据模型
│   └── commands/        # Tauri 命令（扫描/视频/标签/日志/统计/设置）
└── tauri.conf.json
```

## 使用提示

1. 首次使用：点击侧边栏「选择文件夹并扫描」指定视频库根目录。
2. 双击视频 → 内嵌播放并自动记录观看日志；右键 → 更多操作。
3. 在「设置」页可配置 ffmpeg 路径、外部播放器、忽略文件夹、主题等。
