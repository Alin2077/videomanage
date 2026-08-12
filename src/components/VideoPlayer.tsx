import { useCallback, useEffect, useRef, useState } from "react";
import { Button, Space } from "antd";
import { CloseOutlined, FastBackwardOutlined, FastForwardOutlined } from "@ant-design/icons";
import { toAssetUrl, videoApi } from "../services/api";
import { useSettingsStore } from "../stores/useSettingsStore";
import { useDataVersionStore } from "../stores/useDataVersionStore";
import type { VideoInfo } from "../types";
import { formatDuration } from "../utils/format";

interface Props {
  video: VideoInfo;
  onClose: () => void;
}

/**
 * 内嵌播放器：打开时记录观看日志，关闭时写入关闭时间与时长。
 * 键盘控制：←/→ 快退/快进（步长可在设置中调整）、空格 播放/暂停、Esc 关闭。
 */
export default function VideoPlayer({ video, onClose }: Props) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const logIdRef = useRef<number | null>(null);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);

  // 快进/快退步长（秒），设置页可配置，默认 5s
  const seekStep = useSettingsStore((s) => {
    const v = parseFloat(s.settings.seek_step);
    return Number.isFinite(v) && v > 0 ? v : 5;
  });

  const bumpLogs = useDataVersionStore((s) => s.bumpLogs);

  const close = useCallback(() => {
    if (logIdRef.current !== null) {
      videoApi.logClose(logIdRef.current).catch(() => {});
      logIdRef.current = null;
      bumpLogs();
    }
    onClose();
  }, [onClose, bumpLogs]);

  const seekBy = useCallback(
    (delta: number) => {
      const v = videoRef.current;
      if (!v) return;
      const max = v.duration || 0;
      v.currentTime = Math.min(max, Math.max(0, v.currentTime + delta));
      setCurrentTime(v.currentTime);
    },
    [],
  );

  useEffect(() => {
    videoApi
      .logOpen(video.id)
      .then((logId) => {
        logIdRef.current = logId;
        bumpLogs();
      })
      .catch(() => {});

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        close();
        return;
      }
      if (e.key === "ArrowRight") {
        e.preventDefault();
        seekBy(seekStep);
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        seekBy(-seekStep);
      } else if (e.key === " ") {
        e.preventDefault();
        const v = videoRef.current;
        if (v) {
          if (v.paused) v.play();
          else v.pause();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      // 组件卸载兜底：若未正常关闭则补记日志
      if (logIdRef.current !== null) {
        videoApi.logClose(logIdRef.current).catch(() => {});
        logIdRef.current = null;
        bumpLogs();
      }
    };
  }, [video.id, close, seekBy, seekStep, bumpLogs]);

  return (
    <div className="player-container" onClick={(e) => e.stopPropagation()}>
      <video
        ref={videoRef}
        className="player-video"
        src={toAssetUrl(video.filePath)}
        controls
        autoPlay
        onTimeUpdate={(e) => setCurrentTime(e.currentTarget.currentTime)}
        onLoadedMetadata={(e) => setDuration(e.currentTarget.duration)}
      />
      <div className="player-bar">
        <Space>
          <span style={{ fontWeight: 600, maxWidth: 420, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {video.customTitle || video.fileName}
          </span>
          <span style={{ opacity: 0.7 }}>
            {formatDuration(currentTime)} / {formatDuration(duration)}
            {video.width && video.height ? ` · ${video.width}×${video.height}` : ""}
          </span>
        </Space>
        <Space>
          <span style={{ opacity: 0.7, fontSize: 12 }}>
            <FastBackwardOutlined /> / <FastForwardOutlined /> {seekStep}s · 空格 播放/暂停 · Esc 关闭
          </span>
          <Button icon={<CloseOutlined />} onClick={close} size="small">
            关闭 (Esc)
          </Button>
        </Space>
      </div>
    </div>
  );
}
