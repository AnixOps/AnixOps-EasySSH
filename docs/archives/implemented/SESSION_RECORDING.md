# EasySSH 终端会话录制系统

## 概述

EasySSH 实现了一个完整的终端会话录制回放系统，兼容 [asciinema](https://asciinema.org/) 标准格式。

## 功能特性

### 1. Asciinema 格式兼容
- 完全支持 asciinema v2 格式
- 可以播放任何 asciinema 录制的 `.cast` 文件
- 导出标准 asciinema 格式便于分享

### 2. 实时录制
```rust
use easyssh_core::session_recording::{
    SessionRecordingManager, RecordingConfig
};

let manager = SessionRecordingManager::new("./recordings")?;

let config = RecordingConfig {
    width: 80,
    height: 24,
    record_input: true,
    enable_privacy_filter: true,
    ..Default::default()
};

let recording_id = manager.start_recording(config, None).await?;

// 录制输出
manager.record_output(&recording_id, "Hello World\n").await?;

// 录制输入
manager.record_input(&recording_id, "ls\n").await?;

// 停止录制
let metadata = manager.stop_recording(&recording_id).await?;
```

### 3. 隐私保护
自动过滤以下敏感信息：
- 密码提示响应
- API Keys
- 私钥内容
- 认证 Token
- AWS Access Keys
- 信用卡号
- SSH 密钥指纹

```rust
let mut filter = SensitiveDataFilter::new();
filter.add_custom_pattern(r"secret\s*=\s*\S+", "[SECRET REDACTED]")?;
```

### 4. 回放播放器
```typescript
import { SessionPlayer } from './components/recording';

function App() {
  return (
    <SessionPlayer
      recordingUrl="/path/to/recording.cast"
      autoPlay={false}
      showControls={true}
      onTimeUpdate={(time, duration) => console.log(time)}
      onFinish={() => console.log('Playback finished')}
    />
  );
}
```

**快捷键：**
- `Space` - 播放/暂停
- `←` - 后退5秒
- `→` - 前进5秒
- `1-5` - 切换播放速度 (0.5x, 1x, 1.5x, 2x, 4x)

### 5. 时间轴控制
- 可跳转到任意时间点
- 实时显示当前播放位置
- 标记关键位置

### 6. 导出格式
| 格式 | 用途 | 依赖 |
|------|------|------|
| asciicast | 原始格式，最佳质量 | 无 |
| JSON | 结构化数据 | 无 |
| Text | 纯文本输出 | 无 |
| GIF | 嵌入README/博客 | [agg](https://github.com/asciinema/agg) |
| MP4 | 通用视频格式 | ffmpeg |

### 7. 搜索录制内容
```rust
// 在单个录制中搜索
let results = manager.search_in_recording(&recording_id, "error").await?;

// 跨所有录制搜索
let all_results = manager.search_all_recordings("deploy").await;
```

### 8. 注释标记
```rust
// 添加标记
manager.add_mark(&recording_id, "Deployment started", Some("#FF9800")).await?;

// 自动标记（检测到命令时）
// 配置: auto_mark_commands: true
```

### 9. 存储管理
- 自动 gzip 压缩
- 过期自动清理
- 存储配额管理

```rust
// 清理30天前的录制，限制总大小为1GB
storage.cleanup_expired(
    &mut recordings,
    30,              // days
    1024 * 1024 * 1024,  // bytes
).await?;
```

### 10. 云端分享
```rust
let config = CloudShareConfig {
    api_url: "https://asciinema.org".to_string(),
    api_token: Some("your-token".to_string()),
};

let manager = CloudShareManager::new(config);
let url = manager.upload_to_asciinema(&file_path, Some("My Recording")).await?;
```

## 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                     Frontend (React)                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ SessionPlayer│  │RecordingManager│  │ ExportManager│    │
│  │  (Playback)  │  │  (Recording)   │  │   (Export)   │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
└────────────────────┬───────────────────────────────────────┘
                     │ Tauri Commands
┌────────────────────┴───────────────────────────────────────┐
│                  Rust Backend (Core)                         │
│  ┌────────────────────────────────────────────────────┐   │
│  │         SessionRecordingManager                     │   │
│  │  ┌────────────┐ ┌────────────┐ ┌────────────┐      │   │
│  │  │ Recording  │ │  Player    │ │  Storage   │      │   │
│  │  │  Engine    │ │  Engine    │ │  Manager   │      │   │
│  │  └────────────┘ └────────────┘ └────────────┘      │   │
│  └────────────────────────────────────────────────────┘   │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐            │
│  │   Export   │ │   Cloud    │ │  Sensitive │            │
│  │  Manager   │ │   Share    │ │   Filter   │            │
│  └────────────┘ └────────────┘ └────────────┘            │
└────────────────────────────────────────────────────────────┘
                     │
┌────────────────────┴───────────────────────────────────────┐
│                   File System                                │
│              ./recordings/*.cast.gz                          │
└─────────────────────────────────────────────────────────────┘
```

## 文件结构

```
core/src/
├── session_recording.rs      # 核心录制模块
├── recording_commands.rs     # Tauri命令桥接
└── lib.rs                    # 模块导出

src/components/recording/
├── SessionPlayer.tsx         # 回放播放器组件
├── SessionPlayer.css         # 播放器样式
├── RecordingManager.tsx      # 录制管理组件
├── RecordingManager.css      # 管理器样式
├── ExportManager.tsx         # 导出管理组件
├── ExportManager.css         # 导出样式
└── index.ts                  # 组件导出
```

## 使用示例

### 完整录制流程
```typescript
import { RecordingManager, SessionPlayer } from './components/recording';

// 在终端组件中
function TerminalWithRecording() {
  const [isRecording, setIsRecording] = useState(false);
  const [recordingId, setRecordingId] = useState<string | null>(null);

  const startRecording = async () => {
    const id = await invoke('recording_start', {
      width: 80,
      height: 24,
      recordInput: true,
      enablePrivacyFilter: true,
    });
    setRecordingId(id);
    setIsRecording(true);
  };

  const handleTerminalData = (data: string) => {
    // 发送到SSH
    sendToSSH(data);

    // 同时录制
    if (isRecording && recordingId) {
      invoke('recording_record_input', { recordingId, data });
    }
  };

  const handleTerminalOutput = (data: string) => {
    displayInTerminal(data);

    // 录制输出
    if (isRecording && recordingId) {
      invoke('recording_record_output', { recordingId, data });
    }
  };

  return (
    <div>
      <button onClick={startRecording} disabled={isRecording}>
        {isRecording ? 'Recording...' : 'Start Recording'}
      </button>
      <Terminal onData={handleTerminalData} onOutput={handleTerminalOutput} />
    </div>
  );
}
```

### 播放录制
```typescript
function RecordingPlayer({ recordingId }: { recordingId: string }) {
  const [recordingData, setRecordingData] = useState<string>('');

  useEffect(() => {
    invoke('recording_get_player_data', { recordingId })
      .then(setRecordingData);
  }, [recordingId]);

  return (
    <SessionPlayer
      recordingContent={recordingData}
      showControls={true}
      theme="dark"
    />
  );
}
```

## API参考

### Rust API

**SessionRecordingManager**
- `start_recording(config, server_id) -> recording_id`
- `record_output(recording_id, data)`
- `record_input(recording_id, data)`
- `record_resize(recording_id, width, height)`
- `add_mark(recording_id, label, color)`
- `pause_recording(recording_id)`
- `resume_recording(recording_id)`
- `stop_recording(recording_id) -> metadata`
- `list_recordings() -> metadata[]`
- `delete_recording(recording_id)`
- `search_in_recording(recording_id, query) -> results`

**SessionPlayer**
- `load(path) -> player`
- `play()`
- `pause()`
- `stop()`
- `seek(time)`
- `set_speed(speed)`

### TypeScript Props

**SessionPlayer**
```typescript
interface SessionPlayerProps {
  recordingUrl?: string;
  recordingContent?: string;
  autoPlay?: boolean;
  initialSpeed?: 0.5 | 1 | 1.5 | 2 | 4;
  onFinish?: () => void;
  onTimeUpdate?: (time: number, duration: number) => void;
  onMark?: (mark: SessionMark) => void;
  showControls?: boolean;
  theme?: 'dark' | 'light';
}
```

## 性能优化

1. **压缩存储**: 使用 gzip 压缩，通常可减少 80%+ 体积
2. **增量写入**: 录制时直接写入文件，内存占用低
3. **懒加载**: 播放时按需加载事件
4. **渲染优化**: 使用 requestAnimationFrame 控制播放帧率

## 安全考虑

1. **隐私过滤**: 默认启用，自动检测并屏蔽敏感信息
2. **本地优先**: 录制数据默认本地存储
3. **加密存储**: 可选择对录制文件进行加密
4. **审计日志**: Pro 版本可记录谁访问了哪些录制

## 未来扩展

- [ ] AI 智能标记：自动检测关键操作并添加标记
- [ ] 差异对比：对比两个录制的差异
- [ ] 实时协作：多人同时观看录制
- [ ] 智能摘要：生成录制的文字摘要
- [ ] 语音解说：添加语音解说层
