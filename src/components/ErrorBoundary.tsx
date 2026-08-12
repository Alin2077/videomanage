import { Component, type ReactNode } from "react";
import { Button, Result } from "antd";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/** 全局错误边界：捕获渲染异常，避免整页白屏 */
export default class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: unknown) {
    console.error("[ErrorBoundary]", error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div style={{ height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
          <Result
            status="error"
            title="页面渲染出错"
            subTitle={
              <pre style={{ textAlign: "left", fontSize: 12, maxWidth: 640, overflow: "auto", whiteSpace: "pre-wrap" }}>
                {this.state.error.message}
                {"\n\n"}
                {this.state.error.stack}
              </pre>
            }
            extra={[
              <Button type="primary" key="reload" onClick={() => window.location.reload()}>
                重新加载
              </Button>,
              <Button key="back" onClick={() => {
                this.setState({ error: null });
                window.location.hash = "#/dashboard";
              }}>
                返回仪表盘
              </Button>,
            ]}
          />
        </div>
      );
    }
    return this.props.children;
  }
}
