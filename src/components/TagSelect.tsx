import { Checkbox, Collapse, Empty } from "antd";
import { useTagStore } from "../stores/useTagStore";
import type { Tag } from "../types";

interface Props {
  value: number[];
  onChange: (tagIds: number[]) => void;
  maxHeight?: number;
}

/** 标签选择器：按组展示，可多选 */
export default function TagSelect({ value, onChange, maxHeight = 260 }: Props) {
  const tagGroups = useTagStore((s) => s.tagGroups);

  const toggle = (tag: Tag) => {
    const has = value.includes(tag.id);
    onChange(has ? value.filter((id) => id !== tag.id) : [...value, tag.id]);
  };

  const tagCount = tagGroups.reduce((acc, g) => acc + g.tags.length, 0);
  if (tagCount === 0) {
    return <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无标签，请先在标签管理中创建" />;
  }

  return (
    <div style={{ maxHeight, overflow: "auto" }}>
      <Collapse
        ghost
        size="small"
        defaultActiveKey={tagGroups.map((g) => String(g.id))}
        items={tagGroups.map((g) => ({
          key: String(g.id),
          label: (
            <span>
              {g.name} <span style={{ color: "#999", fontSize: 12 }}>({g.tags.length})</span>
            </span>
          ),
          children: (
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
              {g.tags.map((t) => (
                <Checkbox
                  key={t.id}
                  checked={value.includes(t.id)}
                  onChange={() => toggle(t)}
                >
                  <span style={{ color: t.color }}>●</span> {t.name}
                </Checkbox>
              ))}
            </div>
          ),
        }))}
      />
    </div>
  );
}
