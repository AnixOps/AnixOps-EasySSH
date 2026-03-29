import React, { useEffect, useRef, useState } from 'react';
import ReactDOM from 'react-dom/client';
import { useSessionStore } from '../stores/sessionStore';
import { Terminal } from './Terminal';

import { GoldenLayout } from 'golden-layout';

// 存储每个container对应的React root
const containerRoots = new Map<string, ReactDOM.Root>();

export const SplitScreen: React.FC = () => {
  const containerRef = useRef<HTMLDivElement>(null);
  const layoutRef = useRef<any>(null);
  const [isInitialized, setIsInitialized] = useState(false);

  const {
    terminalSessions,
    activeSessionId,
    sshDisconnect,
    clearAllSessions,
    setActiveSession,
  } = useSessionStore();

  // Keep refs to avoid stale closures and prevent effect from re-running on every render
  const sshDisconnectRef = useRef(sshDisconnect);
  const setActiveSessionRef = useRef(setActiveSession);
  sshDisconnectRef.current = sshDisconnect;
  setActiveSessionRef.current = setActiveSession;

  // 初始化 Golden Layout
  useEffect(() => {
    if (!containerRef.current || layoutRef.current) return;

    const handleTabClose = async (terminalSessionId: string) => {
      await sshDisconnectRef.current(terminalSessionId);
    };

    // These refs are stable - we read .current at call time
    // eslint-disable-next-line react-hooks/exhaustive-deps
    const onActivate = (terminalSessionId: string) => {
      if (terminalSessionId !== 'new') {
        setActiveSessionRef.current(terminalSessionId);
      }
    };

    const config = {
      root: {
        type: 'row',
        content: [],
      },
      dimensions: {
        borderWidth: 1,
        headerHeight: 35,
        minItemHeight: 100,
        minItemWidth: 200,
      },
      header: {
        show: true,
        popout: true,
        maximise: true,
        close: true,
      },
      content: [],
    };

    // @ts-ignore - golden-layout constructor
    const layout = new GoldenLayout(config, containerRef.current);

    // 注册终端组件
    layout.registerComponent('terminal', (container: any, props: any) => {
      const { terminalSessionId } = props;
      const element = container.element[0] || container.element;

      // 清除容器内容
      element.innerHTML = '';

      // 创建div用于React渲染
      const div = document.createElement('div');
      div.style.width = '100%';
      div.style.height = '100%';
      div.id = `terminal-${terminalSessionId}`;
      element.appendChild(div);

      // 渲染React Terminal组件
      const root = ReactDOM.createRoot(div);
      containerRoots.set(terminalSessionId, root);

      root.render(
        React.createElement(Terminal, {
          sessionId: terminalSessionId === 'new' ? '' : terminalSessionId,
          onActivate: () => onActivate(terminalSessionId),
          onClose: () => {
            if (terminalSessionId !== 'new') {
              handleTabClose(terminalSessionId);
            }
          },
        })
      );

      // 清理函数
      container.addEventListener('close', () => {
        if (terminalSessionId !== 'new') {
          handleTabClose(terminalSessionId);
        }
        root.unmount();
        containerRoots.delete(terminalSessionId);
      });
    });

    layout.init();
    layoutRef.current = layout;
    setIsInitialized(true);

    return () => {
      // Call layout.destroy() first - it triggers 'close' events on all containers
      // which will properly unmount each React root and clean up containerRoots
      layout.destroy();
      // Final safety clear in case any roots weren't cleaned up by close events
      containerRoots.clear();
      layoutRef.current = null;
      setIsInitialized(false);
    };
  // Only depend on stable values - refs handle current values at call time
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 监听会话变化，自动添加/移除tab
  useEffect(() => {
    if (!layoutRef.current || !isInitialized) return;

    const layout = layoutRef.current;

    // 添加新会话到布局 (use terminalSessionId = session.id)
    terminalSessions.forEach((session) => {
      const existingComponent = layout.root.contentItems.find((item: any) =>
        item.componentType === 'terminal' && item.config?.terminalSessionId === session.id
      );

      if (!existingComponent) {
        layout.addComponent('terminal', {
          terminalSessionId: session.id,
          title: session.serverName,
        });
      }
    });

    // 移除已关闭的会话 (compare terminalSessionId = session.id)
    layout.root.contentItems.forEach((item: any) => {
      if (item.type === 'component') {
        const terminalSessionId = item.config?.terminalSessionId;
        if (terminalSessionId && terminalSessionId !== 'new' && !terminalSessions.find(s => s.id === terminalSessionId)) {
          item.remove();
        }
      }
    });
  }, [terminalSessions, isInitialized]);

  // 水平分屏
  const splitHorizontal = () => {
    if (!layoutRef.current || !activeSessionId) return;

    const session = terminalSessions.find(s => s.id === activeSessionId);
    if (!session) return;

    // 找到当前活动的stack
    const stacks = layoutRef.current.root.getItemsByType('stack');
    if (stacks.length > 0) {
      const activeStack = stacks.find((s: any) =>
        s.contentItems.some((c: any) => c.config?.terminalSessionId === session.id)
      );
      if (activeStack) {
        const newIndex = activeStack.contentItems.length;
        activeStack.addComponent(newIndex, 'terminal', {
          terminalSessionId: session.id,
          title: session.serverName + ' (copy)',
        });
      }
    }
  };

  // 垂直分屏
  const splitVertical = () => {
    if (!layoutRef.current || !activeSessionId) return;

    const session = terminalSessions.find(s => s.id === activeSessionId);
    if (!session) return;

    // 获取当前活动组件
    const stacks = layoutRef.current.root.getItemsByType('stack');
    if (stacks.length === 0) return;

    const activeStack = stacks.find((s: any) =>
      s.contentItems.some((c: any) => c.config?.terminalSessionId === session.id)
    );
    if (!activeStack) return;

    // 找到当前组件的索引
    const activeIndex = activeStack.contentItems.findIndex(
      (c: any) => c.config?.terminalSessionId === session.id
    );

    // 创建垂直布局
    const columnConfig = {
      type: 'column',
      width: 50,
      content: [
        {
          type: 'component',
          componentType: 'terminal',
          terminalSessionId: session.id,
          title: session.serverName,
        },
        {
          type: 'component',
          componentType: 'terminal',
          terminalSessionId: session.id,
          title: session.serverName + ' (copy)',
        },
      ],
    };

    // 替换当前组件为垂直布局
    activeStack.replaceChild(activeIndex, columnConfig);
  };

  // 新建终端窗口
  const addNewTerminal = () => {
    if (!layoutRef.current) return;

    layoutRef.current.addComponent('terminal', {
      terminalSessionId: 'new',
      title: 'New Terminal',
    });
  };

  // 关闭所有 - rely on store + effect for cleanup
  const closeAll = () => {
    void clearAllSessions();
  };

  return (
    <div className="relative flex flex-col h-full">
      {/* 分屏工具栏 */}
      <div className="flex items-center justify-between px-3 py-2 bg-slate-800 border-b border-slate-700">
        <div className="flex items-center gap-2">
          <span className="text-sm text-slate-300">
            Terminals: {terminalSessions.length}
          </span>
          {activeSessionId && (
            <span className="text-xs text-slate-500">
              Active: {terminalSessions.find(s => s.id === activeSessionId)?.serverName}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={addNewTerminal}
            aria-label="Open new terminal tab"
            className="px-3 py-1 text-xs text-slate-300 hover:text-white hover:bg-slate-700 rounded"
            title="New Terminal"
          >
            + New
          </button>
          <button
            onClick={splitHorizontal}
            disabled={!activeSessionId || terminalSessions.length === 0}
            aria-label="Split terminal horizontally"
            className="px-3 py-1 text-xs text-slate-300 hover:text-white hover:bg-slate-700 rounded disabled:opacity-50 disabled:cursor-not-allowed"
            title="Split Horizontal"
          >
            ⬜ Split H
          </button>
          <button
            onClick={splitVertical}
            disabled={!activeSessionId || terminalSessions.length === 0}
            aria-label="Split terminal vertically"
            className="px-3 py-1 text-xs text-slate-300 hover:text-white hover:bg-slate-700 rounded disabled:opacity-50 disabled:cursor-not-allowed"
            title="Split Vertical"
          >
            ▤ Split V
          </button>
          <button
            onClick={closeAll}
            disabled={terminalSessions.length === 0}
            aria-label="Close all terminal sessions"
            className="px-3 py-1 text-xs text-slate-300 hover:text-white hover:bg-slate-700 rounded disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Close All
          </button>
        </div>
      </div>

      {/* Golden Layout 容器 */}
      <div ref={containerRef} className="flex-1 overflow-hidden" />

      {/* 未连接状态 - 使用嵌入式提示而非覆盖层 */}
      {terminalSessions.length === 0 && isInitialized && (
        <div className="flex-1 flex items-center justify-center bg-slate-900/50">
          <div className="text-center max-w-md p-8">
            <div className="text-4xl mb-4">🖥️</div>
            <p className="text-slate-400 mb-2">暂无活动终端</p>
            <p className="text-slate-500 text-sm mb-4">
              从左侧边栏选择一个服务器，点击"连接"开始会话
            </p>
            <p className="text-slate-600 text-xs">
              Or click <span className="text-cyan-400">+ New</span> to open a blank terminal
            </p>
          </div>
        </div>
      )}
    </div>
  );
};
