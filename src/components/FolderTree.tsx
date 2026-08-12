import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { Tree, Empty, Button, Tooltip } from "antd";
import { FolderOutlined, ReloadOutlined } from "@ant-design/icons";
import { scanApi } from "../services/api";

interface Props {
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  refreshKey: number;
}

interface TreeDataItem {
  key: string;
  title: ReactNode;
  isLeaf?: boolean;
  children?: TreeDataItem[];
}

/** 懒加载文件夹树 */
export default function FolderTree({ selectedId, onSelect, refreshKey }: Props) {
  const [treeData, setTreeData] = useState<TreeDataItem[]>([]);
  const [loaded, setLoaded] = useState(false);

  const loadRoot = async () => {
    try {
      const roots = await scanApi.getRootFolders();
      setTreeData(
        roots.map((f) => ({
          key: `f-${f.id}`,
          title: (
            <span>
              <FolderOutlined style={{ color: "#f6c343" }} /> {f.name}
              <span style={{ color: "#999", fontSize: 12, marginLeft: 6 }}>{f.videoCount}</span>
            </span>
          ),
          isLeaf: !f.hasChildren,
        })),
      );
      setLoaded(true);
    } catch {
      /* ignore */
    }
  };

  useEffect(() => {
    loadRoot();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey]);

  const loadChildren = async (node: TreeDataItem): Promise<TreeDataItem[]> => {
    const id = Number(node.key.replace("f-", ""));
    const children = await scanApi.getFolderChildren(id);
    return children.map((f) => ({
      key: `f-${f.id}`,
      title: (
        <span>
          <FolderOutlined style={{ color: "#f6c343" }} /> {f.name}
          <span style={{ color: "#999", fontSize: 12, marginLeft: 6 }}>{f.videoCount}</span>
        </span>
      ),
      isLeaf: !f.hasChildren,
    }));
  };

  if (!loaded) {
    return (
      <div style={{ padding: 16 }}>
        <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="尚未扫描任何文件夹" />
        <div style={{ color: "#999", fontSize: 12, textAlign: "center" }}>
          请点击左侧「选择文件夹并扫描」
        </div>
      </div>
    );
  }

  return (
    <div className="folder-tree">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "0 4px 8px" }}>
        <span style={{ fontSize: 13, color: "#888" }}>文件夹</span>
        <Tooltip title="刷新树">
          <Button size="small" type="text" icon={<ReloadOutlined />} onClick={loadRoot} />
        </Tooltip>
      </div>
      <Tree
        treeData={treeData}
        selectedKeys={selectedId !== null ? [`f-${selectedId}`] : []}
        onSelect={(keys) => {
          if (keys.length > 0) {
            onSelect(Number(String(keys[0]).replace("f-", "")));
          } else {
            onSelect(null);
          }
        }}
        loadData={async (node) => {
          const kids = await loadChildren(node as TreeDataItem);
          const items = treeData.map((t) => ({ ...t }));
          // 更新节点 children
          const patch = (list: TreeDataItem[]): TreeDataItem[] =>
            list.map((item) => {
              if (item.key === node.key) {
                return { ...item, children: kids };
              }
              if (item.children) return { ...item, children: patch(item.children) };
              return item;
            });
          setTreeData(patch(items));
        }}
        showLine
        showIcon={false}
        blockNode
      />
      <div style={{ marginTop: 8, padding: "0 4px" }}>
        <Button
          size="small"
          type={selectedId === null ? "primary" : "default"}
          onClick={() => onSelect(null)}
          block
        >
          全部视频
        </Button>
      </div>
    </div>
  );
}
