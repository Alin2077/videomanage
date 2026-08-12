import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { Tree, Empty, Button, Tooltip } from "antd";
import { FolderOutlined, ReloadOutlined, AppstoreOutlined } from "@ant-design/icons";
import { scanApi } from "../services/api";
import { useWorkspaceStore } from "../stores/useWorkspaceStore";
import { useDataVersionStore } from "../stores/useDataVersionStore";
import type { FolderNode } from "../types";

interface Props {
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  refreshKey: number;
  workspaceId: number | null;
}

interface TreeDataItem {
  key: string;
  title: ReactNode;
  isLeaf?: boolean;
  children?: TreeDataItem[];
}

/** 节点标题：名称过长截断（ellipsis），视频数不截断 */
const nodeTitle = (icon: ReactNode, name: string, count: number, bold = false) => (
  <span
    style={{
      display: "inline-flex",
      alignItems: "center",
      maxWidth: "100%",
      minWidth: 0,
      width: "100%",
    }}
  >
    <span style={{ flexShrink: 0, marginRight: 4 }}>{icon}</span>
    <span
      style={{
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        flex: 1,
        minWidth: 0,
        fontWeight: bold ? 600 : 400,
      }}
    >
      {name}
    </span>
    {count > 0 && (
      <span style={{ color: "#999", fontSize: 12, marginLeft: 6, flexShrink: 0 }}>{count}</span>
    )}
  </span>
);

/** 懒加载文件夹树（以工作区为根，按工作区过滤） */
export default function FolderTree({ selectedId, onSelect, refreshKey, workspaceId }: Props) {
  const [treeData, setTreeData] = useState<TreeDataItem[]>([]);
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);
  const [loaded, setLoaded] = useState(false);
  const workspace = useWorkspaceStore(
    (s) => s.workspaces.find((w) => w.id === workspaceId),
  );
  const bumpLibrary = useDataVersionStore((s) => s.bumpLibrary);

  const folderNode = (f: FolderNode): TreeDataItem => ({
    key: `f-${f.id}`,
    title: nodeTitle(<FolderOutlined style={{ color: "#f6c343" }} />, f.name, f.videoCount),
    isLeaf: !f.hasChildren,
  });

  const loadRoot = async () => {
    if (workspaceId === null) {
      setTreeData([]);
      setLoaded(false);
      return;
    }
    try {
      const roots = await scanApi.getRootFolders(workspaceId);
      if (roots.length === 0) {
        // 尚无文件夹记录（未扫描或根目录无视频）
        setTreeData([]);
      } else {
        // 以工作区为可视根节点，其下挂根文件夹
        setTreeData([
          {
            key: `ws-${workspaceId}`,
            title: nodeTitle(
              <AppstoreOutlined style={{ color: "#4a7dff" }} />,
              workspace?.name || "工作区",
              roots.reduce((a, r) => a + r.videoCount, 0),
              true,
            ),
            isLeaf: false,
            children: roots.map(folderNode),
          },
        ]);
        // 自动展开工作区节点，直白展示第一层目录
        setExpandedKeys([`ws-${workspaceId}`]);
      }
      setLoaded(true);
    } catch {
      /* ignore */
    }
  };

  // 刷新树并通知列表页同步刷新
  const refreshTree = () => {
    loadRoot();
    bumpLibrary();
  };

  useEffect(() => {
    loadRoot();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshKey, workspaceId]);

  // 工作区切换时重置展开状态
  useEffect(() => {
    setExpandedKeys([]);
  }, [workspaceId]);

  const loadChildren = async (node: TreeDataItem): Promise<TreeDataItem[]> => {
    const key = String(node.key);
    if (key.startsWith("ws-")) {
      // 工作区节点：返回其下根文件夹（已加载，无需再查）
      return (node.children ?? []) as TreeDataItem[];
    }
    const id = Number(key.replace("f-", ""));
    const children = await scanApi.getFolderChildren(id);
    return children.map(folderNode);
  };

  const patchTree = (list: TreeDataItem[], key: string, kids: TreeDataItem[]): TreeDataItem[] =>
    list.map((item) => {
      if (item.key === key) {
        return { ...item, children: kids };
      }
      if (item.children) {
        return { ...item, children: patchTree(item.children, key, kids) };
      }
      return item;
    });

  const loadedRoots = useMemo(
    () => treeData[0]?.children?.length ?? 0,
    [treeData],
  );

  if (!loaded || workspaceId === null) {
    return (
      <div style={{ padding: 16 }}>
        <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="尚未扫描任何文件夹" />
        <div style={{ color: "#999", fontSize: 12, textAlign: "center" }}>
          请点击左侧「新增文件夹并扫描」
        </div>
      </div>
    );
  }

  return (
    <div className="folder-tree">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "0 4px 8px" }}>
        <span style={{ fontSize: 13, color: "#888" }}>文件夹</span>
        <Tooltip title="刷新树并同步列表">
          <Button size="small" type="text" icon={<ReloadOutlined />} onClick={refreshTree} />
        </Tooltip>
      </div>
      {loadedRoots === 0 ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="该工作区暂无目录结构"
          style={{ marginTop: 30 }}
        />
      ) : (
        <Tree
          treeData={treeData}
          expandedKeys={expandedKeys}
          onExpand={(keys) => setExpandedKeys(keys)}
          selectedKeys={selectedId !== null ? [`f-${selectedId}`] : []}
          onSelect={(keys) => {
            const folderKey = keys.find((k) => String(k).startsWith("f-"));
            if (folderKey) {
              onSelect(Number(String(folderKey).replace("f-", "")));
            } else {
              onSelect(null);
            }
          }}
          loadData={async (node) => {
            const key = String(node.key);
            // 工作区节点或已展开过的文件夹：无需重复加载
            if (key.startsWith("ws-")) return;
            const kids = await loadChildren(node as TreeDataItem);
            setTreeData((prev) => patchTree(prev, key, kids));
          }}
          showLine
          showIcon={false}
          blockNode
        />
      )}
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
