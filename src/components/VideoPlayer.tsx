import { useCallback, useEffect, useRef } from "react";
import { Button, Space } from "antd";
import { CloseOutlined } from "@ant-design/icons";
import { toAssetUrl, videoApi } from "../services/api";
import type { VideoInfo } from "../types";

interface Props {
  video: VideoInfo;
  onClose: () => void;
}

/**
 * 内嵌播放器：打开时记录观看日志，关闭时写入关闭时间与时长。
 */
export default function VideoPlayer({ video, onClose }: Props) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const logIdRef = useRef<number | null>(null);

  const close = useCallback(() => {
    if (logIdRef.current !== null) {
      videoApi.logClose(logIdRef.current).catch(() => {});
      logIdRef.current = null;
    }
    onClose();
  }, [onClose]);

  useEffect(() => {
    videoApi
      .logOpen(video.id)
      .then((logId) => {
        logIdRef.current = logId;
      })
      .catch(() => {});

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      // 组件卸载兜底
      if (logIdRef.current !== null) {
        videoApi.logClose(logIdRef.current).catch(() => {});
        logIdRef.current = null;
      }
    };
  }, [video.id, close]);

  return (
    <div className="player-container" onClick={(e) => e.stopPropagation()}>
      <video
        ref={videoRef}
        className="player-video"
        src={toAssetUrl(video.filePath)}
        controls
        autoPlay
      />
      <div className="player-bar">
        <Space>
          <span style={{ fontWeight: 600 }}>{video.customTitle || video.fileName}</span>
          <span style={{ opacity: 0.7 }}>
            {video.width && video.height ? `${video.width}×${video.height}` : ""}
            {video.codec ? ` · ${video.codec}` : ""}
          </span>
        </Space>
        <Button icon={<CloseOutlined />} onClick={close} size="small">
          关闭 (Esc)
        </Button>
      </div>
    </div>
  );
}
