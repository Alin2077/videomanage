import { useEffect, useState } from "react";
import {
  Button,
  Card,
  ColorPicker,
  Empty,
  Input,
  Modal,
  Popconfirm,
  Space,
  Tag as AntTag,
  message,
} from "antd";
import {
  DeleteOutlined,
  EditOutlined,
  FolderAddOutlined,
  PlusOutlined,
} from "@ant-design/icons";
import { statsApi } from "../services/api";
import { useTagStore } from "../stores/useTagStore";
import type { TagStat } from "../types";

export default function Tags() {
  const { tagGroups, createTag, updateTag, createGroup, deleteGroup } =
    useTagStore();
  const [tagCounts, setTagCounts] = useState<Record<number, number>>({});

  // 新建/编辑标签弹窗
  const [tagModal, setTagModal] = useState<{ groupId: number | null; tag?: { id: number; name: string; color: string } } | null>(null);
  const [tagName, setTagName] = useState("");
  const [tagColor, setTagColor] = useState("#1890ff");

  // 新建/编辑组弹窗
  const [groupModal, setGroupModal] = useState<{ group?: { id: number; name: string } } | null>(null);
  const [groupName, setGroupName] = useState("");

  useEffect(() => {
    statsApi.tagStats().then((list: TagStat[]) => {
      const map: Record<number, number> = {};
      for (const t of list) map[t.tagId] = t.videoCount;
      setTagCounts(map);
    });
  }, [tagGroups]);

  const openCreateTag = (groupId: number | null) => {
    setTagModal({ groupId });
    setTagName("");
    setTagColor("#1890ff");
  };

  const openEditTag = (groupId: number | null, tag: { id: number; name: string; color: string }) => {
    setTagModal({ groupId, tag });
    setTagName(tag.name);
    setTagColor(tag.color);
  };

  const saveTag = async () => {
    if (!tagName.trim()) {
      message.warning("请输入标签名");
      return;
    }
    try {
      if (tagModal?.tag) {
        await updateTag({ id: tagModal.tag.id, groupId: tagModal.groupId, name: tagName.trim(), color: tagColor });
        message.success("标签已更新");
      } else {
        await createTag({ groupId: tagModal?.groupId ?? null, name: tagName.trim(), color: tagColor });
        message.success("标签已创建");
      }
      setTagModal(null);
    } catch (e) {
      message.error(`操作失败: ${e}`);
    }
  };

  const saveGroup = async () => {
    if (!groupName.trim()) {
      message.warning("请输入组名");
      return;
    }
    try {
      await createGroup({ id: groupModal?.group?.id, name: groupName.trim() });
      message.success("标签组已保存");
      setGroupModal(null);
    } catch (e) {
      message.error(`操作失败: ${e}`);
    }
  };

  return (
    <div className="page-container">
      <div className="page-title">标签管理</div>

      <Space style={{ marginBottom: 14 }}>
        <Button
          type="primary"
          icon={<FolderAddOutlined />}
          onClick={() => {
            setGroupModal({});
            setGroupName("");
          }}
        >
          新建标签组
        </Button>
        <Button icon={<PlusOutlined />} onClick={() => openCreateTag(null)}>
          新建标签（默认组）
        </Button>
      </Space>

      {tagGroups.length === 0 ? (
        <Empty description="暂无标签，点击上方按钮创建" />
      ) : (
        <div style={{ display: "flex", flexWrap: "wrap", gap: 14 }}>
          {tagGroups.map((g) => (
            <Card
              key={g.id}
              size="small"
              style={{ width: 340 }}
              title={
                <Space>
                  {g.name}
                  <span style={{ color: "#999", fontWeight: 400, fontSize: 12 }}>
                    {g.tags.length} 个标签
                  </span>
                </Space>
              }
              extra={
                g.id !== 0 && (
                  <Space>
                    <Button
                      size="small"
                      type="text"
                      icon={<EditOutlined />}
                      onClick={() => {
                        setGroupModal({ group: { id: g.id, name: g.name } });
                        setGroupName(g.name);
                      }}
                    />
                    <Popconfirm
                      title="删除该标签组？组内标签将移入默认组。"
                      onConfirm={() => deleteGroup(g.id).then(() => message.success("已删除"))}
                    >
                      <Button size="small" type="text" danger icon={<DeleteOutlined />} />
                    </Popconfirm>
                  </Space>
                )
              }
            >
              <div style={{ display: "flex", flexWrap: "wrap", gap: 8, minHeight: 36 }}>
                {g.tags.map((t) => (
                  <AntTag
                    key={t.id}
                    color={t.color}
                    style={{ cursor: "pointer", userSelect: "none" }}
                    onDoubleClick={() => openEditTag(g.id === 0 ? null : g.id, t)}
                  >
                    {t.name}
                    {tagCounts[t.id] !== undefined && (
                      <span style={{ opacity: 0.75, marginLeft: 4 }}>{tagCounts[t.id]}</span>
                    )}
                  </AntTag>
                ))}
                {g.tags.length === 0 && (
                  <span style={{ color: "#bbb", fontSize: 12 }}>（空）</span>
                )}
              </div>
              <Button
                size="small"
                type="dashed"
                block
                icon={<PlusOutlined />}
                style={{ marginTop: 10 }}
                onClick={() => openCreateTag(g.id === 0 ? null : g.id)}
              >
                添加标签
              </Button>
            </Card>
          ))}
        </div>
      )}

      {/* 标签弹窗 */}
      <Modal
        title={tagModal?.tag ? "编辑标签" : "新建标签"}
        open={!!tagModal}
        onCancel={() => setTagModal(null)}
        onOk={saveTag}
        okText="保存"
        destroyOnClose
      >
        <Space direction="vertical" style={{ width: "100%" }}>
          <Input
            placeholder="标签名（组内唯一）"
            value={tagName}
            onChange={(e) => setTagName(e.target.value)}
            onPressEnter={saveTag}
          />
          <Space>
            <span>颜色：</span>
            <ColorPicker value={tagColor} onChange={(c) => setTagColor(c.toHexString())} />
          </Space>
        </Space>
      </Modal>

      {/* 组弹窗 */}
      <Modal
        title={groupModal?.group ? "重命名标签组" : "新建标签组"}
        open={!!groupModal}
        onCancel={() => setGroupModal(null)}
        onOk={saveGroup}
        okText="保存"
        destroyOnClose
      >
        <Input
          placeholder="组名，如：类型 / 心情 / 项目"
          value={groupName}
          onChange={(e) => setGroupName(e.target.value)}
          onPressEnter={saveGroup}
        />
      </Modal>
    </div>
  );
}
