import { Badge, Tag as AntTag, Tooltip } from "antd";
import { PlayCircleOutlined } from "@ant-design/icons";
import { toAssetUrl } from "../services/api";
import type { VideoInfo } from "../types";
import { formatDuration, formatSize, formatResolution } from "../utils/format";

interface Props {
  video: VideoInfo;
  selected: boolean;
  onClick: (video: VideoInfo) => void;
  onPlay: (video: VideoInfo) => void;
}

/** 网格视图下的视频卡片 */
export default function VideoCard({ video, selected, onClick, onPlay }: Props) {
  const title = video.customTitle || video.fileName;

  return (
    <div
      className={`video-card${selected ? " selected" : ""}`}
      onClick={() => onClick(video)}
      onDoubleClick={() => onPlay(video)}
    >
      {video.coverPath ? (
        <img
          className="video-cover"
          src={toAssetUrl(video.coverPath)}
          alt={title}
          loading="lazy"
        />
      ) : (
        <div className="video-cover">
          <PlayCircleOutlined />
        </div>
      )}
      <div className="video-card-body">
        <Tooltip title={title}>
          <div className="video-card-title">{title}</div>
        </Tooltip>
        <div className="video-card-meta">
          <span>{formatDuration(video.duration)}</span>
          <span>{formatResolution(video.width, video.height)}</span>
        </div>
        <div className="video-card-meta" style={{ marginTop: 4 }}>
          <span>{formatSize(video.fileSize)}</span>
          <span>
            {video.openCount > 0 && (
              <Badge count={video.openCount} size="small" color="#4a7dff" />
            )}
          </span>
        </div>
        {video.tags.length > 0 && (
          <div style={{ marginTop: 6, display: "flex", flexWrap: "wrap", gap: 4 }}>
            {video.tags.slice(0, 3).map((t) => (
              <AntTag key={t.id} color={t.color} style={{ marginInlineEnd: 0, fontSize: 11 }}>
                {t.name}
              </AntTag>
            ))}
            {video.tags.length > 3 && (
              <AntTag style={{ marginInlineEnd: 0, fontSize: 11 }}>+{video.tags.length - 3}</AntTag>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
