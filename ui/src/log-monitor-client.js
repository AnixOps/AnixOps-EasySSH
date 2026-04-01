/**
 * LogMonitorClient - WebSocket客户端 for EasySSH日志监控中心
 *
 * 功能：
 * - 实时接收多源日志流
 * - 搜索和过滤
 * - 告警通知
 * - 统计图表更新
 * - 日志着色显示
 */

class LogMonitorClient {
    constructor(url = 'ws://127.0.0.1:8765') {
        this.url = url;
        this.ws = null;
        this.reconnectInterval = 3000;
        this.maxReconnectAttempts = 10;
        this.reconnectAttempts = 0;
        this.isConnected = false;

        // 事件处理器
        this.handlers = {
            onConnect: [],
            onDisconnect: [],
            onEntry: [],
            onBatch: [],
            onStats: [],
            onAlert: [],
            onSourceConnected: [],
            onSourceDisconnected: [],
            onError: []
        };

        // 日志缓冲区
        this.entries = [];
        this.maxBufferSize = 10000;

        // 过滤配置
        this.currentFilter = {
            minLevel: 'TRACE',
            keywords: [],
            sourceIds: [],
            regexPattern: null
        };

        // 统计
        this.stats = {
            totalEntries: 0,
            entriesByLevel: {},
            entriesBySource: {},
            entriesPerMinute: 0,
            errorRate: 0
        };

        // 告警规则
        this.alertRules = [];

        // 日志级别颜色映射
        this.levelColors = {
            'TRACE': '#6c757d',
            'DEBUG': '#0d6efd',
            'INFO': '#198754',
            'WARN': '#ffc107',
            'ERROR': '#dc3545',
            'FATAL': '#721c24',
            'UNKNOWN': '#6c757d'
        };
    }

    // 连接管理
    connect() {
        if (this.ws?.readyState === WebSocket.OPEN) {
            console.log('WebSocket already connected');
            return;
        }

        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
            console.log('LogMonitor WebSocket connected');
            this.isConnected = true;
            this.reconnectAttempts = 0;
            this.emit('onConnect');
        };

        this.ws.onclose = () => {
            console.log('LogMonitor WebSocket disconnected');
            this.isConnected = false;
            this.emit('onDisconnect');
            this.attemptReconnect();
        };

        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            this.emit('onError', { message: 'WebSocket error', error });
        };

        this.ws.onmessage = (event) => {
            try {
                const message = JSON.parse(event.data);
                this.handleMessage(message);
            } catch (e) {
                console.error('Failed to parse message:', e);
            }
        };
    }

    disconnect() {
        this.reconnectAttempts = this.maxReconnectAttempts; // 防止自动重连
        if (this.ws) {
            this.ws.close();
        }
    }

    attemptReconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            console.log(`Reconnecting... attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts}`);
            setTimeout(() => this.connect(), this.reconnectInterval);
        } else {
            console.error('Max reconnect attempts reached');
        }
    }

    // 消息处理
    handleMessage(message) {
        switch (message.type) {
            case 'connected':
                console.log('Server acknowledged connection:', message.message);
                break;

            case 'entry':
                this.handleNewEntry(message.entry);
                break;

            case 'batch':
                this.handleBatchEntries(message.entries);
                break;

            case 'stats':
                this.handleStatsUpdate(message.stats);
                break;

            case 'alert':
                this.handleAlert(message.alert);
                break;

            case 'source_connected':
                this.emit('onSourceConnected', {
                    sourceId: message.source_id,
                    sourceName: message.source_name
                });
                break;

            case 'source_disconnected':
                this.emit('onSourceDisconnected', {
                    sourceId: message.source_id,
                    reason: message.reason
                });
                break;

            case 'error':
                this.emit('onError', { message: message.message });
                break;

            case 'search_results':
                this.emit('onSearchResults', message.entries);
                break;

            case 'analysis':
                this.emit('onAnalysis', message.result);
                break;

            case 'sources':
                this.emit('onSources', message.sources);
                break;

            default:
                console.log('Unknown message type:', message.type);
        }
    }

    handleNewEntry(entry) {
        // 应用过滤
        if (!this.matchesFilter(entry)) {
            return;
        }

        // 添加到缓冲区
        this.entries.push(entry);
        if (this.entries.length > this.maxBufferSize) {
            this.entries.shift();
        }

        // 更新统计
        this.updateEntryStats(entry);

        // 检查告警
        this.checkAlerts(entry);

        // 发射事件
        this.emit('onEntry', entry);
    }

    handleBatchEntries(entries) {
        const filtered = entries.filter(e => this.matchesFilter(e));

        filtered.forEach(entry => {
            this.entries.push(entry);
            this.updateEntryStats(entry);
        });

        // 限制缓冲区大小
        if (this.entries.length > this.maxBufferSize) {
            this.entries = this.entries.slice(-this.maxBufferSize);
        }

        this.emit('onBatch', filtered);
    }

    handleStatsUpdate(stats) {
        this.stats = stats;
        this.emit('onStats', stats);
    }

    handleAlert(alert) {
        this.emit('onAlert', alert);

        // 桌面通知
        if (Notification.permission === 'granted') {
            new Notification('EasySSH 日志告警', {
                body: alert.message,
                icon: '/alert-icon.png'
            });
        }
    }

    // 过滤逻辑
    matchesFilter(entry) {
        // 级别过滤
        const levelPriority = this.getLevelPriority(entry.level);
        const minPriority = this.getLevelPriority(this.currentFilter.minLevel);
        if (levelPriority < minPriority) {
            return false;
        }

        // 源过滤
        if (this.currentFilter.sourceIds.length > 0 &&
            !this.currentFilter.sourceIds.includes(entry.source_id)) {
            return false;
        }

        // 关键词过滤
        if (this.currentFilter.keywords.length > 0) {
            const content = (entry.message + ' ' + entry.raw_line).toLowerCase();
            const hasAllKeywords = this.currentFilter.keywords.every(kw =>
                content.includes(kw.toLowerCase())
            );
            if (!hasAllKeywords) {
                return false;
            }
        }

        // 正则过滤
        if (this.currentFilter.regexPattern) {
            try {
                const regex = new RegExp(this.currentFilter.regexPattern, 'i');
                if (!regex.test(entry.raw_line)) {
                    return false;
                }
            } catch (e) {
                // 无效正则，忽略
            }
        }

        return true;
    }

    getLevelPriority(level) {
        const priorities = {
            'TRACE': 0, 'DEBUG': 1, 'INFO': 2,
            'WARN': 3, 'ERROR': 4, 'FATAL': 5, 'UNKNOWN': 2
        };
        return priorities[level] ?? 2;
    }

    updateEntryStats(entry) {
        this.stats.totalEntries++;
        this.stats.entriesByLevel[entry.level] =
            (this.stats.entriesByLevel[entry.level] || 0) + 1;
        this.stats.entriesBySource[entry.source_id] =
            (this.stats.entriesBySource[entry.source_id] || 0) + 1;
    }

    // 告警检查
    checkAlerts(entry) {
        this.alertRules.forEach(rule => {
            if (!rule.enabled) return;

            const triggered = this.checkAlertRule(rule, entry);
            if (triggered) {
                this.handleAlert({
                    rule_id: rule.id,
                    rule_name: rule.name,
                    triggered_at: Date.now() / 1000,
                    log_entry: entry,
                    message: `Alert: ${rule.name} - ${entry.message}`
                });
            }
        });
    }

    checkAlertRule(rule, entry) {
        switch (rule.condition.type) {
            case 'keyword_match':
                const keywords = rule.condition.keywords;
                const content = entry.raw_line.toLowerCase();
                return keywords.some(kw => content.includes(kw.toLowerCase()));

            case 'level_threshold':
                const minLevelPriority = this.getLevelPriority(rule.condition.min_level);
                const entryPriority = this.getLevelPriority(entry.level);
                return entryPriority >= minLevelPriority;

            case 'pattern_match':
                try {
                    const regex = new RegExp(rule.condition.regex);
                    return regex.test(entry.raw_line);
                } catch (e) {
                    return false;
                }

            default:
                return false;
        }
    }

    // API 命令
    sendCommand(action, params = {}) {
        if (!this.isConnected) {
            console.error('WebSocket not connected');
            return;
        }

        this.ws.send(JSON.stringify({
            action,
            ...params
        }));
    }

    search(filter) {
        this.sendCommand('search', { ...filter });
    }

    getStats(rangeSeconds = 3600) {
        this.sendCommand('stats', { range_seconds: rangeSeconds });
    }

    analyze(rangeSeconds = 3600) {
        this.sendCommand('analyze', { range_seconds: rangeSeconds });
    }

    getSources() {
        this.sendCommand('get_sources');
    }

    getAlerts() {
        this.sendCommand('get_alerts');
    }

    // 过滤配置
    setFilter(filter) {
        this.currentFilter = { ...this.currentFilter, ...filter };
    }

    setMinLevel(level) {
        this.currentFilter.minLevel = level;
    }

    setKeywords(keywords) {
        this.currentFilter.keywords = keywords;
    }

    setSourceFilter(sourceIds) {
        this.currentFilter.sourceIds = sourceIds;
    }

    setRegexPattern(pattern) {
        this.currentFilter.regexPattern = pattern;
    }

    // 告警规则管理
    addAlertRule(rule) {
        this.alertRules.push(rule);
    }

    removeAlertRule(ruleId) {
        this.alertRules = this.alertRules.filter(r => r.id !== ruleId);
    }

    // 事件订阅
    on(event, handler) {
        if (this.handlers[event]) {
            this.handlers[event].push(handler);
        }
    }

    off(event, handler) {
        if (this.handlers[event]) {
            this.handlers[event] = this.handlers[event].filter(h => h !== handler);
        }
    }

    emit(event, data) {
        if (this.handlers[event]) {
            this.handlers[event].forEach(handler => {
                try {
                    handler(data);
                } catch (e) {
                    console.error(`Error in ${event} handler:`, e);
                }
            });
        }
    }

    // 日志渲染
    renderEntry(entry, options = {}) {
        const color = entry.color || this.levelColors[entry.level] || '#6c757d';
        const timestamp = new Date(entry.timestamp * 1000).toLocaleTimeString();
        const source = options.showSource ? `<span class="source" style="color:${entry.color || '#666'}">[${entry.source_name}]</span>` : '';

        return `
            <div class="log-entry" data-level="${entry.level}" data-source="${entry.source_id}" style="color: ${color}">
                <span class="timestamp">${timestamp}</span>
                <span class="level-badge" style="background: ${color}; color: white; padding: 2px 6px; border-radius: 3px; font-size: 0.8em;">${entry.level}</span>
                ${source}
                <span class="message">${this.escapeHtml(entry.message)}</span>
            </div>
        `;
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    // 获取过滤后的条目
    getFilteredEntries() {
        return this.entries.filter(e => this.matchesFilter(e));
    }

    // 清空缓冲区
    clear() {
        this.entries = [];
        this.stats = {
            totalEntries: 0,
            entriesByLevel: {},
            entriesBySource: {},
            entriesPerMinute: 0,
            errorRate: 0
        };
    }

    // 导出日志
    exportToFile(format = 'json', filename = 'logs') {
        const entries = this.getFilteredEntries();
        let content, mimeType, extension;

        switch (format) {
            case 'json':
                content = JSON.stringify(entries, null, 2);
                mimeType = 'application/json';
                extension = 'json';
                break;
            case 'csv':
                const headers = ['timestamp', 'level', 'source', 'message'];
                const rows = entries.map(e => [
                    new Date(e.timestamp * 1000).toISOString(),
                    e.level,
                    e.source_name,
                    e.message
                ].map(v => `"${String(v).replace(/"/g, '""')}"`).join(','));
                content = [headers.join(','), ...rows].join('\n');
                mimeType = 'text/csv';
                extension = 'csv';
                break;
            case 'txt':
                content = entries.map(e =>
                    `[${new Date(e.timestamp * 1000).toISOString()}] [${e.level}] ${e.source_name}: ${e.message}`
                ).join('\n');
                mimeType = 'text/plain';
                extension = 'txt';
                break;
            default:
                throw new Error('Unsupported format: ' + format);
        }

        const blob = new Blob([content], { type: mimeType });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `${filename}.${extension}`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }
}

// 图表组件
class LogMonitorCharts {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.ctx = this.canvas.getContext('2d');
        this.data = [];
    }

    updateTimeSeries(timeSeriesData) {
        this.data = timeSeriesData;
        this.draw();
    }

    draw() {
        const ctx = this.ctx;
        const width = this.canvas.width;
        const height = this.canvas.height;

        // 清空画布
        ctx.clearRect(0, 0, width, height);

        if (this.data.length === 0) return;

        // 找出最大值
        const maxValue = Math.max(...this.data.map(d => d.total_count), 1);

        // 绘制网格
        ctx.strokeStyle = '#eee';
        ctx.lineWidth = 1;
        for (let i = 0; i <= 4; i++) {
            const y = height - (i * height / 4);
            ctx.beginPath();
            ctx.moveTo(0, y);
            ctx.lineTo(width, y);
            ctx.stroke();
        }

        // 绘制时间序列线
        const barWidth = width / this.data.length;

        this.data.forEach((point, index) => {
            const x = index * barWidth;

            // 绘制堆叠柱状图
            const errorHeight = (point.error_count / maxValue) * height * 0.8;
            const warnHeight = (point.warn_count / maxValue) * height * 0.8;
            const infoHeight = (point.info_count / maxValue) * height * 0.8;

            // ERROR - 红色
            if (point.error_count > 0) {
                ctx.fillStyle = '#dc3545';
                ctx.fillRect(x, height - errorHeight, barWidth - 1, errorHeight);
            }

            // WARN - 黄色
            if (point.warn_count > 0) {
                ctx.fillStyle = '#ffc107';
                ctx.fillRect(x, height - errorHeight - warnHeight, barWidth - 1, warnHeight);
            }

            // INFO - 绿色
            if (point.info_count > 0) {
                ctx.fillStyle = '#198754';
                ctx.fillRect(x, height - errorHeight - warnHeight - infoHeight, barWidth - 1, infoHeight);
            }
        });
    }
}

// 导出
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { LogMonitorClient, LogMonitorCharts };
} else {
    window.LogMonitorClient = LogMonitorClient;
    window.LogMonitorCharts = LogMonitorCharts;
}
