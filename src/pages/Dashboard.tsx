import { useEffect, useState } from "react";
import { Col, Empty, Row, Spin, Table, Tag as AntTag } from "antd";
import { statsApi, videoApi } from "../services/api";
import type { DashboardStats, LeaderboardItem, TrendPoint, VideoInfo } from "../types";
import { formatSize, formatWatchTime, formatDuration } from "../utils/format";
import ReactECharts from "echarts-for-react";
import VideoPlayer from "../components/VideoPlayer";

export default function Dashboard() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [trend, setTrend] = useState<TrendPoint[]>([]);
  const [topOpen, setTopOpen] = useState<LeaderboardItem[]>([]);
  const [playing, setPlaying] = useState<VideoInfo | null>(null);
  const [loading, setLoading] = useState(true);

  const load = () => {
    setLoading(true);
    Promise.all([
      statsApi.dashboard(),
      statsApi.trend("day"),
      statsApi.leaderboard("open", 5),
    ])
      .then(([s, t, l]) => {
        setStats(s);
        setTrend(t);
        setTopOpen(l);
      })
      .finally(() => setLoading(false));
  };

  useEffect(load, []);

  const trendOption = {
    tooltip: { trigger: "axis" },
    legend: { data: ["观看时长", "观看次数"] },
    grid: { left: 50, right: 50, top: 40, bottom: 30 },
    xAxis: { type: "category", data: trend.map((t) => t.label.slice(5)) },
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

  const playByPath = async (item: LeaderboardItem) => {
    try {
      const list = await videoApi.search(item.fileName, 5);
      const v = list.find((x) => x.id === item.videoId) || list[0];
      if (v) setPlaying(v);
    } catch {
      /* ignore */
    }
  };

  if (loading) return <div style={{ padding: 40, textAlign: "center" }}><Spin /></div>;

  const cards = [
    { label: "视频总数", value: stats?.totalVideos ?? 0, sub: `共 ${formatSize(stats?.totalFileSize ?? 0)}` },
    { label: "文件夹数", value: stats?.totalFolders ?? 0, sub: "已索引文件夹" },
    { label: "累计观看", value: `${stats?.totalOpenCount ?? 0} 次`, sub: `今日 ${stats?.todayOpenCount ?? 0} 次` },
    { label: "累计观看时长", value: formatWatchTime(stats?.totalWatchSeconds ?? 0), sub: `今日 ${formatWatchTime(stats?.todayWatchSeconds ?? 0)}` },
  ];

  return (
    <div className="page-container">
      <div className="page-title">仪表盘</div>

      <Row gutter={[14, 14]}>
        {cards.map((c) => (
          <Col xs={12} lg={6} key={c.label}>
            <div className="stat-card">
              <div className="label">{c.label}</div>
              <div className="value">{c.value}</div>
              <div className="sub">{c.sub}</div>
            </div>
          </Col>
        ))}
      </Row>

      <Row gutter={[14, 14]} style={{ marginTop: 14 }}>
        <Col xs={24} lg={16}>
          <div className="stat-card">
            <div className="label" style={{ marginBottom: 8 }}>最近 30 天观看趋势</div>
            {trend.length > 0 ? (
              <ReactECharts option={trendOption} style={{ height: 300 }} />
            ) : (
              <Empty description="暂无观看数据，打开视频后自动记录" />
            )}
          </div>
        </Col>
        <Col xs={24} lg={8}>
          <div className="stat-card">
            <div className="label" style={{ marginBottom: 8 }}>最多观看 TOP5</div>
            <Table
              size="small"
              rowKey="videoId"
              pagination={false}
              dataSource={topOpen}
              onRow={(r) => ({ onDoubleClick: () => playByPath(r) })}
              columns={[
                {
                  title: "排名",
                  width: 44,
                  render: (_: unknown, __: unknown, i: number) => (
                    <span style={{ fontWeight: 600, color: i < 3 ? "#f59e0b" : undefined }}>{i + 1}</span>
                  ),
                },
                {
                  title: "视频",
                  dataIndex: "fileName",
                  ellipsis: true,
                  render: (name: string) => (
                    <span title={name} style={{ cursor: "pointer" }}>{name}</span>
                  ),
                },
                {
                  title: "次数",
                  dataIndex: "openCount",
                  width: 64,
                  render: (n: number) => <AntTag color="blue">{n}</AntTag>,
                },
                {
                  title: "时长",
                  dataIndex: "duration",
                  width: 72,
                  render: (d: number | null) => formatDuration(d),
                },
              ]}
            />
          </div>
        </Col>
      </Row>

      {playing && <VideoPlayer video={playing} onClose={() => setPlaying(null)} />}
    </div>
  );
}
