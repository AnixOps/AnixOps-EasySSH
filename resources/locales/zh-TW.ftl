### EasySSH - 繁體中文翻譯

## 應用資訊
-app-name = EasySSH
-app-description = 全平台現代SSH客戶端
-app-version = 版本 { $version }

## 通用UI
-general-ok = 確定
-general-cancel = 取消
-general-save = 儲存
-general-delete = 刪除
-general-edit = 編輯
-general-add = 新增
-general-remove = 移除
-general-close = 關閉
-general-back = 返回
-general-next = 下一步
-general-finish = 完成
-general-apply = 套用
-general-reset = 重設
-general-search = 搜尋
-general-filter = 篩選
-general-loading = 載入中...
-general-empty = 空
-general-error = 錯誤
-general-success = 成功
-general-warning = 警告
-general-info = 資訊
-general-confirm = 確認
-general-yes = 是
-general-no = 否
-general-copy = 複製
-general-paste = 貼上
-general-cut = 剪下
-general-select-all = 全選
-general-undo = 復原
-general-redo = 重做
-general-refresh = 重新整理
-general-reload = 重新載入
-general-settings = 設定
-general-preferences = 偏好設定
-general-help = 說明
-general-about = 關於
-general-feedback = 意見回饋
-general-donate = 贊助
-general-exit = 結束
-general-quit = 結束

## 主視窗
-main-title = { -app-name }
-main-sidebar-servers = 伺服器
-main-sidebar-groups = 群組
-main-sidebar-snippets = 程式碼片段
-main-sidebar-history = 歷史記錄
-main-sidebar-settings = 設定
-main-sidebar-add-server = 新增伺服器
-main-sidebar-add-group = 新增群組
-main-sidebar-import = 匯入
-main-sidebar-export = 匯出
-main-status-ready = 就緒
-main-status-connecting = 連線中...
-main-status-connected = 已連線
-main-status-disconnected = 已斷線
-main-status-error = 錯誤
-main-status-syncing = 同步中...

## 伺服器列表
-server-list-title = 伺服器列表
-server-list-empty = 未設定伺服器
-server-list-empty-hint = 點擊「新增伺服器」開始
-server-list-search-placeholder = 搜尋伺服器...
-server-list-filter-all = 全部伺服器
-server-list-filter-favorites = 我的最愛
-server-list-filter-recent = 最近連線
-server-list-sort-name = 名稱
-server-list-sort-host = 主機
-server-list-sort-last-connected = 最後連線
-server-list-context-connect = 連線
-server-list-context-edit = 編輯
-server-list-context-delete = 刪除
-server-list-context-duplicate = 複製
-server-list-context-copy-ip = 複製IP
-server-list-context-add-to-group = 新增至群組
-server-list-context-remove-from-group = 從群組移除
-server-group-uncategorized = 未分類

## 伺服器詳情
-server-detail-title = 伺服器詳情
-server-detail-connect = 連線
-server-detail-disconnect = 中斷連線
-server-detail-edit = 編輯
-server-detail-delete = 刪除
-server-detail-name = 名稱
-server-detail-host = 主機
-server-detail-port = 連接埠
-server-detail-username = 使用者名稱
-server-detail-auth-method = 驗證方式
-server-detail-auth-password = 密碼
-server-detail-auth-key = SSH金鑰
-server-detail-auth-agent = SSH代理
-server-detail-group = 群組
-server-detail-tags = 標籤
-server-detail-color = 顏色
-server-detail-notes = 備註
-server-detail-last-connected = 上次連線
-server-detail-never-connected = 從未連線
-server-detail-created = 建立時間
-server-detail-updated = 更新時間
-server-detail-connection-status = 連線狀態
-server-detail-terminal = 終端機
-server-detail-sftp = SFTP
-server-detail-monitoring = 監控
-server-detail-logs = 紀錄

## 新增/編輯伺服器對話框
-server-dialog-add-title = 新增伺服器
-server-dialog-edit-title = 編輯伺服器
-server-dialog-name-label = 伺服器名稱
-server-dialog-name-placeholder = 例如：生產環境Web伺服器
-server-dialog-host-label = 主機/IP位址
-server-dialog-host-placeholder = 例如：192.168.1.100 或 server.example.com
-server-dialog-port-label = 連接埠
-server-dialog-port-default = 預設：22
-server-dialog-username-label = 使用者名稱
-server-dialog-username-placeholder = 例如：root 或 admin
-server-dialog-auth-label = 驗證方式
-server-dialog-auth-password-label = 密碼
-server-dialog-auth-password-placeholder = 輸入密碼
-server-dialog-auth-key-label = 私鑰
-server-dialog-auth-key-select = 選擇金鑰檔案
-server-dialog-auth-key-passphrase = 金鑰密碼（選填）
-server-dialog-auth-agent-label = 使用SSH代理
-server-dialog-group-label = 群組
-server-dialog-group-none = 無群組
-server-dialog-group-new = 新增群組...
-server-dialog-tags-label = 標籤
-server-dialog-tags-placeholder = 新增標籤，以逗號分隔
-server-dialog-color-label = 顏色
-server-dialog-advanced = 進階選項
-server-dialog-advanced-jump = 跳板主機
-server-dialog-advanced-proxy = 代理伺服器
-server-dialog-advanced-keepalive = 保持連線間隔
-server-dialog-save-and-connect = 儲存並連線
-server-dialog-save = 儲存
-server-dialog-test-connection = 測試連線
-server-dialog-test-success = 連線成功！
-server-dialog-test-failed = 連線失敗：{ $error }
-server-dialog-validation-required = { $field }不能為空
-server-dialog-validation-invalid-port = 連接埠必須在1-65535之間
-server-dialog-validation-invalid-host = 無效的主機位址

## 群組
group-list-title = 群組列表
group-list-empty = 未建立群組
group-list-add = 新增群組
group-dialog-add-title = 建立群組
group-dialog-edit-title = 編輯群組
group-dialog-name-label = 群組名稱
group-dialog-name-placeholder = 例如：生產環境伺服器
group-dialog-parent-label = 上層群組（選填）
group-dialog-color-label = 群組顏色
group-dialog-delete-confirm = 刪除群組「{ $name }」？
group-dialog-delete-warning = 此群組中的伺服器將變為未分類。

## 終端機
-terminal-title = 終端機
-terminal-new-tab = 新增分頁
-terminal-close-tab = 關閉分頁
-terminal-split-horizontal = 水平分割
-terminal-split-vertical = 垂直分割
-terminal-copy = 複製
-terminal-paste = 貼上
-terminal-select-all = 全選
-terminal-find = 尋找
-terminal-clear = 清除
-terminal-reset = 重設
-terminal-zoom-in = 放大
-terminal-zoom-out = 縮小
-terminal-zoom-reset = 重設縮放
-terminal-font-size = 字型大小
-terminal-theme = 佈景主題
-terminal-theme-dark = 深色
-terminal-theme-light = 淺色
-terminal-cursor-style = 游標樣式
-terminal-cursor-blink = 閃爍游標
-terminal-alt-screen = 替代螢幕
-terminal-mouse-reporting = 滑鼠回報
-terminal-disconnected = 工作階段已中斷。按Enter鍵重新連線。
-terminal-reconnecting = 重新連線中...
-terminal-command-palette = 命令面板
-terminal-sftp-open = 開啟SFTP面板

## SFTP
-sftp-title = SFTP檔案傳輸
-sftp-local = 本機
-sftp-remote = 遠端
-sftp-upload = 上傳
-sftp-download = 下載
-sftp-delete = 刪除
-sftp-rename = 重新命名
-sftp-new-folder = 新增資料夾
-sftp-refresh = 重新整理
-sftp-transfer-progress = 正在傳輸 { $filename }（{ $progress }%）
-sftp-transfer-complete = 傳輸完成：{ $filename }
-sftp-transfer-error = 傳輸失敗：{ $error }
-sftp-path-home = 主目錄
-sftp-path-root = 根目錄
-sftp-context-upload = 上傳至此位置
-sftp-context-download = 下載
-sftp-context-delete = 刪除
-sftp-context-rename = 重新命名
-sftp-context-properties = 內容

## 程式碼片段
-snippets-title = 程式碼片段
-snippets-empty = 未儲存程式碼片段
-snippets-add = 新增程式碼片段
-snippets-edit = 編輯程式碼片段
-snippets-delete = 刪除程式碼片段
-snippets-run = 執行程式碼片段
-snippets-name = 名稱
-snippets-content = 內容
-snippets-variables = 變數
-snippets-description = 描述
-snippets-global = 全域程式碼片段
-snippets-server-specific = 伺服器專用

## 連線
-connection-connecting = 正在連線至 { $host }...
-connection-connected = 已連線至 { $host }
-connection-disconnected = 已從 { $host } 中斷連線
-connection-error = 連線錯誤：{ $error }
-connection-auth-failed = 驗證失敗
-connection-host-key-changed = 警告：遠端主機金鑰已變更！
-connection-host-key-unknown = 未知主機金鑰。是否接受？
-connection-timeout = 連線逾時
-connection-refused = 連線被拒絕
-connection-reset = 連線被對等端重設
-connection-retrying = 正在重試...（{ $attempt }/{ $max }）

## 設定
-settings-title = 設定
-settings-general = 一般
-settings-appearance = 外觀
-settings-terminal = 終端機
-settings-ssh = SSH
-settings-advanced = 進階
-settings-language = 語言
-settings-language-auto = 自動偵測
-settings-theme = 佈景主題
-settings-theme-system = 系統預設
-settings-theme-dark = 深色
-settings-theme-light = 淺色
-settings-font-size = 字型大小
-settings-font-family = 字型
-settings-show-tray-icon = 顯示在系統匣
-settings-minimize-to-tray = 最小化到系統匣
-settings-startup = 啟動
-settings-startup-launch = 開機時啟動
-settings-startup-restore = 還原上次工作階段
-settings-updates = 更新
-settings-updates-check = 檢查更新
-settings-updates-auto = 自動安裝更新
-settings-shortcuts = 鍵盤快速鍵
-settings-shortcuts-new-tab = 新增分頁
-settings-shortcuts-close-tab = 關閉分頁
-settings-shortcuts-next-tab = 下一個分頁
-settings-shortcuts-prev-tab = 上一個分頁
-settings-backup = 備份與還原
-settings-backup-export = 匯出設定
-settings-backup-import = 匯入設定
-settings-reset = 重設為預設值
-settings-reset-confirm = 將所有設定重設為預設值？

## 加密與安全性
-security-unlock-title = 解鎖 EasySSH
-security-unlock-prompt = 輸入主密碼以繼續
-security-unlock-button = 解鎖
-security-forgot-password = 忘記密碼？
-security-set-password = 設定主密碼
-security-change-password = 變更主密碼
-security-password-hint = 密碼提示（選填）
-security-biometric-enable = 啟用{ $type }
-security-biometric-type-face = 面容辨識
-security-biometric-type-fingerprint = 指紋辨識
-security-biometric-type-generic = 生物辨識認證
-security-auto-lock = 自動鎖定
-security-auto-lock-never = 永不
-security-auto-lock-1m = 1分鐘
-security-auto-lock-5m = 5分鐘
-security-auto-lock-15m = 15分鐘
-security-auto-lock-1h = 1小時
-security-export-encrypted = 加密匯出資料

## 匯入/匯出
-import-title = 匯入設定
-import-source = 來源
-import-source-file = 從檔案
-import-source-url = 從URL
-import-source-clipboard = 從剪貼簿
-import-format = 格式
-import-format-auto = 自動偵測
-import-format-json = JSON
-import-format-csv = CSV
-import-format-ssh-config = SSH設定
-import-options = 選項
-import-options-duplicate-skip = 跳過重複項目
-import-options-duplicate-replace = 取代重複項目
-import-options-duplicate-merge = 與現有合併
-import-preview = 預覽
-import-items-count = 找到 { $count } 個項目
-import-success = 匯入成功：已匯入 { $count } 個項目
-import-error = 匯入失敗：{ $error }
-export-title = 匯出設定
-export-format = 匯出格式
-export-what = 匯出內容
-export-what-all = 全部資料
-export-what-servers = 僅伺服器
-export-what-groups = 僅群組
-export-what-snippets = 僅程式碼片段
-export-encrypted = 加密匯出
-export-password = 匯出密碼
-export-success = 匯出成功
-export-error = 匯出失敗：{ $error }

## 監控（標準版/專業版）
-monitoring-title = 系統監控
-monitoring-cpu = CPU
-monitoring-memory = 記憶體
-monitoring-disk = 磁碟
-monitoring-network = 網路
-monitoring-processes = 處理程序
-monitoring-refresh = 重新整理頻率
-monitoring-alert-threshold = 警示閾值
-monitoring-history = 歷史紀錄
-monitoring-export-data = 匯出資料

## 團隊功能（專業版）
-team-title = 團隊管理
-team-members = 成員
-team-roles = 角色
-team-invite = 邀請成員
-team-remove = 移除成員
-team-role-owner = 擁有者
-team-role-admin = 管理員
-team-role-member = 成員
-team-role-viewer = 檢視者
-team-sync = 團隊同步
-team-audit = 稽核紀錄
-team-shared-snippets = 共享程式碼片段

## 通知
-notify-connection-lost = 連線中斷：{ $server }
-notify-transfer-complete = 檔案傳輸完成
-notify-transfer-failed = 檔案傳輸失敗
-notify-update-available = 有新版本可用：{ $version }
-notify-copy-success = 已複製到剪貼簿

## 錯誤訊息
-error-generic = 發生錯誤
-error-not-found = 找不到資源
-error-permission = 權限遭拒
-error-network = 網路錯誤
-error-timeout = 要求逾時
-error-invalid-input = 無效的輸入
-error-database = 資料庫錯誤
-error-crypto = 加密錯誤
-error-ssh = SSH錯誤：{ $message }
-error-keychain = 鑰匙圈錯誤
-error-terminal = 終端機錯誤
-error-import-failed = 匯入失敗
-error-export-failed = 匯出失敗
-error-file-not-found = 找不到檔案：{ $path }
-error-invalid-key = 無效的SSH金鑰
-error-connection-failed = 連線失敗

## 無障礙
-a11y-terminal-label = 終端機面板
-a11y-server-list-label = 伺服器列表
-a11y-sidebar-label = 側邊欄導覽
-a11y-close-dialog = 關閉對話框
-a11y-expand-section = 展開區段
-a11y-collapse-section = 摺疊區段
-a11y-more-options = 更多選項
-a11y-search-results = { $count } 個搜尋結果
-a11y-connection-status = 連線狀態：{ $status }
