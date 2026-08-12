import { useCallback, useEffect, useRef } from "react";
import { Button, Space } from "antd";
import { CloseOutlined } from "@ant-design/icons";
import Artplayer from "artplayer";
import { toAssetUrl, videoApi } from "../services/api";
import { useSettingsStore } from "../stores/useSettingsStore";
import { useDataVersionStore } from "../stores/useDataVersionStore";
import type { VideoInfo } from "../types";

interface Props {
  video: VideoInfo;
  onClose: () => void;
}

/**
 * 基于 artplayer 的内嵌播放器。
 * - 打开/关闭时记录观看日志（只在 video.id 变化时执行一次，避免重复计数）
 * - 键盘控制：←/→ 快退/快进（步长可设置）、空格 播放/暂停、Esc 关闭
 */
export default function VideoPlayer({ video, onClose }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const artRef = useRef<Artplayer | null>(null);
  const logIdRef = useRef<number | null>(null);

  // 快进/快退步长（秒），设置页可配置，默认 5s
  const seekStep = useSettingsStore((s) => {
    const v = parseFloat(s.settings.seek_step);
    return Number.isFinite(v) && v > 0 ? v : 5;
  });

  const bumpLogs = useDataVersionStore((s) => s.bumpLogs);

  // 用 ref 保存最新值，供键盘监听使用（避免因引用变化重建 effect 导致日志重复计数）
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  const seekStepRef = useRef(seekStep);
  seekStepRef.current = seekStep;
  const bumpLogsRef = useRef(bumpLogs);
  bumpLogsRef.current = bumpLogs;
  const openPromiseRef = useRef<Promise<number> | null>(null);

  // 只在 video.id 变化时执行：创建播放器、记录打开日志、绑定键盘
  useEffect(() => {
    const bump = () => bumpLogsRef.current();

    // 记录打开日志（仅一次）
    const openP = videoApi.logOpen(video.id);
    openPromiseRef.current = openP;
    openP
      .then((logId) => {
        logIdRef.current = logId;
        bump();
      })
      .catch(() => {});

    // 创建 artplayer 实例
    if (containerRef.current) {
      const art = new Artplayer({
        container: containerRef.current,
        url: toAssetUrl(video.filePath),
        autoplay: true,
        // 关闭内置快捷键，使用自定义键盘控制（步长可配置）
        hotkey: false,
        theme: "#4a7dff",
        volume: 0.8,
        setting: true,
        pip: true,
        fullscreen: true,
        fullscreenWeb: true,
        flip: true,
        playbackRate: true,
        aspectRatio: true,
        screenshot: true,
        autoSize: false,
      });
      artRef.current = art;
    }

    const onKey = (e: KeyboardEvent) => {
      const art = artRef.current;
      if (!art) return;
      if (e.key === "Escape") {
        closeRef.current();
        return;
      }
      if (e.key === "ArrowRight") {
        e.preventDefault();
        art.currentTime = Math.min(art.duration || 0, art.currentTime + seekStepRef.current);
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        art.currentTime = Math.max(0, art.currentTime - seekStepRef.current);
      } else if (e.key === " ") {
        e.preventDefault();
        art.toggle();
      }
    };
    window.addEventListener("keydown", onKey);

    return () => {
      window.removeEventListener("keydown", onKey);
      // 关闭播放器时记录关闭日志（仅一次；若打开日志尚未完成则等待后补记）
      const p = openPromiseRef.current;
      if (p) {
        p.then(() => {
          if (logIdRef.current !== null) {
            videoApi.logClose(logIdRef.current).catch(() => {});
            logIdRef.current = null;
            bump();
          }
        }).catch(() => {});
      }
      artRef.current?.destroy();
      artRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [video.id]);

  const close = useCallback(() => {
    // 手动关闭：等待打开日志完成后补记关闭（cleanup 兜底同样逻辑）
    const p = openPromiseRef.current;
    if (p) {
      p.then(() => {
        if (logIdRef.current !== null) {
          videoApi.logClose(logIdRef.current).catch(() => {});
          logIdRef.current = null;
          bumpLogsRef.current();
        }
      }).catch(() => {});
    }
    onClose();
  }, [onClose]);

  return (
    <div className="player-container" onClick={(e) => e.stopPropagation()}>
      <div
        ref={containerRef}
        style={{ flex: 1, width: "100%", minHeight: 0, background: "#000" }}
      />
      <div className="player-bar">
        <Space>
          <span style={{ fontWeight: 600, maxWidth: 420, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {video.customTitle || video.fileName}
          </span>
          <span style={{ opacity: 0.7 }}>
            {video.width && video.height ? `${video.width}×${video.height}` : ""}
            {video.codec ? ` · ${video.codec}` : ""}
          </span>
        </Space>
        <Space>
          <span style={{ opacity: 0.7, fontSize: 12 }}>
            ←/→ {seekStep}s 快退/快进 · 空格 播放/暂停 · Esc 关闭
          </span>
          <Button icon={<CloseOutlined />} onClick={close} size="small">
            关闭 (Esc)
          </Button>
        </Space>
      </div>
    </div>
  );
}
