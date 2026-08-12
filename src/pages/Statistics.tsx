import { useEffect, useState } from "react";
import {
  Button,
  Col,
  DatePicker,
  Empty,
  Row,
  Segmented,
  Space,
  Spin,
  Table,
  Tag as AntTag,
  message,
} from "antd";
import { DownloadOutlined } from "@ant-design/icons";
import ReactECharts from "echarts-for-react";
import { logApi, statsApi, videoApi } from "../services/api";
import { useWorkspaceStore } from "../stores/useWorkspaceStore";
import type { HourCell, LeaderboardItem, OpenLogWithVideo, TagStat, TrendPoint, VideoInfo } from "../types";
import { formatDuration, formatWatchTime } from "../utils/format";
import VideoPlayer from "../components/VideoPlayer";

type TrendRange = "day" | "week" | "month";

export default function Statistics() {
  const [range, setRange] = useState<TrendRange>("day");
  const [trend, setTrend] = useState<TrendPoint[]>([]);
  const [tagStats, setTagStats] = useState<TagStat[]>([]);
  const [heatmap, setHeatmap] = useState<HourCell[]>([]);
  const [leaderboard, setLeaderboard] = useState<LeaderboardItem[]>([]);
  const [lbCategory, setLbCategory] = useState("open");
  const [logs, setLogs] = useState<OpenLogWithVideo[]>([]);
  const [logsTotal, setLogsTotal] = useState(0);
  const [logPage, setLogPage] = useState(1);
  const [logRange, setLogRange] = useState<[string | null, string | null]>([null, null]);
  const [playing, setPlaying] = useState<VideoInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const currentWorkspaceId = useWorkspaceStore((s) => s.currentWorkspaceId);
  const workspaceName = useWorkspaceStore(
    (s) => s.workspaces.find((w) => w.id === s.currentWorkspaceId)?.name,
  );

  useEffect(() => {
    setLoading(true);
    Promise.all([
      statsApi.trend(currentWorkspaceId, range),
      statsApi.tagStats(currentWorkspaceId),
      statsApi.hourlyHeatmap(currentWorkspaceId),
    ])
      .then(([t, ts, hm]) => {
        setTrend(t);
        setTagStats(ts);
        setHeatmap(hm);
      })
      .catch((e) => message.error(`加载统计失败: ${e}`))
      .finally(() => setLoading(false));
  }, [range, currentWorkspaceId]);

  useEffect(() => {
    statsApi.leaderboard(currentWorkspaceId, lbCategory, 10).then(setLeaderboard).catch(() => {});
  }, [lbCategory, currentWorkspaceId]);

  useEffect(() => {
    setLogPage(1);
  }, [currentWorkspaceId]);

  useEffect(() => {
    logApi
      .list({ startDate: logRange[0], endDate: logRange[1] }, currentWorkspaceId, logPage, 20)
      .then((r) => {
        setLogs(r.items);
        setLogsTotal(r.total);
      })
      .catch(() => {});
  }, [logPage, logRange, currentWorkspaceId]);

  const trendOption = {
    tooltip: { trigger: "axis" },
    legend: { data: ["观看时长", "观看次数"] },
    grid: { left: 60, right: 60, top: 40, bottom: 30 },
    xAxis: { type: "category", data: trend.map((t) => t.label) },
    yAxis: [
      { type: "value", name: "时长(分钟)" },
      { type: "value", name: "次数" },
    ],
    series: [
      {
        name: "观看时长",
        type: "line",
        smooth: true,
        data: trend.map((t) => Math.round(t.watchSeconds / 60)),
        itemStyle: { color: "#4a7dff" },
        areaStyle: { opacity: 0.15 },
      },
      {
        name: "观看次数",
        type: "line",
        smooth: true,
        yAxisIndex: 1,
        data: trend.map((t) => t.openCount),
        itemStyle: { color: "#ff6b6b" },
      },
    ],
  };

  const tagPieOption = {
    tooltip: { trigger: "item", formatter: "{b}: {c} ({d}%)" },
    series: [
      {
        type: "pie",
        radius: ["35%", "65%"],
        data: tagStats.map((t) => ({ name: t.tagName, value: t.videoCount, itemStyle: { color: t.color } })),
        label: { formatter: "{b} {c}" },
      },
    ],
  };

  const weekdayNames = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
  const heatData = (() => {
    const map = new Map<string, HourCell>();
    for (const c of heatmap) map.set(`${c.weekday}-${c.hour}`, c);
    const cells: { value: [number, number, number]; seconds: number }[] = [];
    for (let w = 0; w < 7; w++) {
      for (let h = 0; h < 24; h++) {
        const c = map.get(`${w}-${h}`);
        cells.push({ value: [h, w, c?.count ?? 0], seconds: c?.seconds ?? 0 });
      }
    }
    return cells;
  })();

  const heatOption = {
    tooltip: {
      formatter: (p: { value: [number, number, number]; data: { seconds: number } }) =>
        `${weekdayNames[p.value[1]]} ${String(p.value[0]).padStart(2, "0")}:00 — ${p.value[2]} 次播放，${formatWatchTime(p.data.seconds)}`,
    },
    grid: { left: 60, right: 20, top: 20, bottom: 40 },
    xAxis: {
      type: "category",
      data: Array.from({ length: 24 }, (_, h) => `${String(h).padStart(2, "0")}`),
      splitArea: { show: true },
    },
    yAxis: {
      type: "category",
      data: weekdayNames,
      splitArea: { show: true },
    },
    visualMap: {
      min: 0,
      max: Math.max(1, ...heatData.map((d) => d.value[2])),
      calculable: true,
      orient: "horizontal",
      left: "center",
      bottom: 0,
      inRange: { color: ["#e8f0ff", "#4a7dff", "#1d39c4"] },
    },
    series: [
      {
        name: "观看热力",
        type: "heatmap",
        data: heatData,
        label: { show: false },
        emphasis: { itemStyle: { shadowBlur: 10, shadowColor: "rgba(0,0,0,0.5)" } },
      },
    ],
  };

  const exportCsv = async () => {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const filePath = await save({
        defaultPath: `观看日志_${new Date().toISOString().slice(0, 10)}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!filePath) return;
      await logApi.export({ startDate: logRange[0], endDate: logRange[1] }, currentWorkspaceId, filePath);
      message.success(`已导出到 ${filePath}`);
    } catch (e) {
      message.error(`导出失败: ${e}`);
    }
  };

  const playLog = async (log: OpenLogWithVideo) => {
    try {
      const v = await videoApi.detail(log.videoId);
      setPlaying(v);
    } catch {
      /* ignore */
    }
  };

  const lbColumns = [
    { title: "#", width: 44, render: (_: unknown, __: unknown, i: number) => i + 1 },
    { title: "视频", dataIndex: "fileName", ellipsis: true },
    { title: "次数", dataIndex: "openCount", width: 70, render: (n: number) => <AntTag color="blue">{n}</AntTag> },
    { title: "时长", dataIndex: "duration", width: 90, render: (d: number | null) => formatDuration(d) },
  ];

  const logColumns = [
    { title: "视频", dataIndex: "fileName", ellipsis: true },
    { title: "打开时间", dataIndex: "openTime", width: 170 },
    { title: "关闭时间", dataIndex: "closeTime", width: 170, render: (v: string | null) => v || "-" },
    { title: "观看时长", dataIndex: "duration", width: 110, render: (d: number | null) => formatWatchTime(d) },
    {
      title: "状态",
      dataIndex: "status",
      width: 90,
      render: (s: string) => (
        <AntTag color={s === "closed" ? "green" : s === "crashed" ? "orange" : "blue"}>
          {s === "closed" ? "正常" : s === "crashed" ? "异常" : "进行中"}
        </AntTag>
      ),
    },
  ];

  if (loading) return <div style={{ padding: 40, textAlign: "center" }}><Spin /></div>;

  return (
    <div className="page-container">
      <div className="page-title">
        统计分析{workspaceName ? ` · ${workspaceName}` : ""}
      </div>

      <Row gutter={[14, 14]}>
        <Col xs={24} lg={14}>
          <div className="stat-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
              <span className="label">观看趋势</span>
              <Segmented
                size="small"
                value={range}
                onChange={(v) => setRange(v as TrendRange)}
                options={[
                  { label: "日", value: "day" },
                  { label: "周", value: "week" },
                  { label: "月", value: "month" },
                ]}
              />
            </div>
            {trend.length > 0 ? (
              <ReactECharts option={trendOption} style={{ height: 320 }} />
            ) : (
              <Empty description="暂无观看数据" />
            )}
          </div>
        </Col>
        <Col xs={24} lg={10}>
          <div className="stat-card">
            <span className="label">标签分布</span>
            {tagStats.length > 0 ? (
              <ReactECharts option={tagPieOption} style={{ height: 320 }} />
            ) : (
              <Empty description="暂无标签" style={{ marginTop: 100 }} />
            )}
          </div>
        </Col>
      </Row>

      <Row gutter={[14, 14]} style={{ marginTop: 14 }}>
        <Col xs={24} lg={14}>
          <div className="stat-card">
            <span className="label">一周观看习惯热力图（7×24）</span>
            {heatmap.length > 0 ? (
              <ReactECharts option={heatOption} style={{ height: 260 }} />
            ) : (
              <Empty description="暂无观看数据" style={{ marginTop: 60 }} />
            )}
          </div>
        </Col>
        <Col xs={24} lg={10}>
          <div className="stat-card">
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
              <span className="label">排行榜 TOP10</span>
              <Segmented
                size="small"
                value={lbCategory}
                onChange={(v) => setLbCategory(v as string)}
                options={[
                  { label: "最多打开", value: "open" },
                  { label: "最长观看", value: "duration" },
                  { label: "最近活跃", value: "recent" },
                ]}
              />
            </div>
            <Table
              size="small"
              rowKey="videoId"
              pagination={false}
              dataSource={leaderboard}
              columns={lbColumns}
              scroll={{ y: 260 }}
            />
          </div>
        </Col>
      </Row>

      <div className="stat-card" style={{ marginTop: 14 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
          <span className="label">观看日志</span>
          <Space>
            <DatePicker.RangePicker
              onChange={(_, dateStrings) =>
                setLogRange([dateStrings[0] || null, dateStrings[1] || null])
              }
              size="small"
            />
            <Button size="small" icon={<DownloadOutlined />} onClick={exportCsv}>
              导出 CSV
            </Button>
          </Space>
        </div>
        <Table
          size="small"
          rowKey="id"
          dataSource={logs}
          columns={logColumns}
          pagination={{
            current: logPage,
            pageSize: 20,
            total: logsTotal,
            onChange: setLogPage,
            showTotal: (t) => `共 ${t} 条`,
          }}
          onRow={(r) => ({ onDoubleClick: () => playLog(r) })}
        />
      </div>

      {playing && <VideoPlayer video={playing} onClose={() => setPlaying(null)} />}
    </div>
  );
}
