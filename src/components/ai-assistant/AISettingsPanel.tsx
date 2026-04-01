/**
 * AI Settings Panel Component
 * @module components/ai-assistant/AISettingsPanel
 */

import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  X,
  Server,
  Thermometer,
  Mic,
  Check,
  AlertCircle,
} from 'lucide-react';

// Store
import { useAIAssistantStore, useAISettings, useAvailableModels } from '../../stores/aiAssistantStore';

// Types
import type { AIProvider, AIModel } from '../../types/aiAssistant';

// =============================================================================
// Component
// =============================================================================

export const AISettingsPanel: React.FC = () => {
  const settings = useAISettings();
  const models = useAvailableModels();
  const uiState = useAIAssistantStore((state) => ({
    isSettingsOpen: state.isSettingsOpen,
    closeSettings: state.closeSettings,
    updateSettings: state.updateSettings,
    updateProviderConfig: state.updateProviderConfig,
    setActiveProvider: state.setActiveProvider,
    setActiveModel: state.setActiveModel,
  }));

  const [activeTab, setActiveTab] = useState<'providers' | 'general' | 'voice'>('providers');
  const [showApiKey, setShowApiKey] = useState<Record<string, boolean>>({});
  const [testResults, setTestResults] = useState<Record<string, 'success' | 'error' | null>>({});

  if (!uiState.isSettingsOpen) return null;

  // Provider display info
  const providerInfo: Record<AIProvider, { name: string; color: string; icon: string }> = {
    claude: { name: 'Claude (Anthropic)', color: '#f97316', icon: '🟠' },
    openai: { name: 'OpenAI', color: '#22c55e', icon: '🟢' },
    gemini: { name: 'Gemini (Google)', color: '#3b82f6', icon: '🔵' },
    local: { name: '本地模型', color: '#a855f7', icon: '🟣' },
    custom: { name: '自定义', color: '#6b7280', icon: '⚪' },
  };

  // Get models for provider
  const getModelsForProvider = (provider: AIProvider): AIModel[] => {
    return models.filter((m) => m.provider === provider);
  };

  // Handle provider test
  const handleTestProvider = async (provider: AIProvider) => {
    setTestResults((prev) => ({ ...prev, [provider]: null }));

    // Simulate test
    await new Promise((r) => setTimeout(r, 1500));

    const config = settings.providers.find((p) => p.provider === provider);
    const success = config?.enabled && (provider === 'local' || config?.apiKey);

    setTestResults((prev) => ({ ...prev, [provider]: success ? 'success' : 'error' }));
  };

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-apple-4"
        onClick={(e) => {
          if (e.target === e.currentTarget) uiState.closeSettings();
        }}
      >
        <motion.div
          initial={{ scale: 0.95, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          exit={{ scale: 0.95, opacity: 0 }}
          className="w-full max-w-2xl max-h-[85vh] bg-apple-bg-primary rounded-apple-xl shadow-apple-xl border border-apple-border overflow-hidden"
        >
          {/* Header */}
          <div className="flex items-center justify-between px-apple-6 py-apple-4 border-b border-apple-border">
            <div>
              <h2 className="text-apple-lg font-semibold text-apple-text-primary">AI 设置</h2>
              <p className="text-apple-sm text-apple-text-secondary">配置 AI 提供商和偏好设置</p>
            </div>
            <button
              onClick={uiState.closeSettings}
              className="p-apple-2 rounded-apple-lg hover:bg-apple-bg-tertiary transition-colors"
            >
              <X className="w-5 h-5 text-apple-text-secondary" />
            </button>
          </div>

          {/* Tabs */}
          <div className="flex border-b border-apple-border">
            {[
              { id: 'providers', label: 'AI 提供商', icon: Server },
              { id: 'general', label: '通用设置', icon: Thermometer },
              { id: 'voice', label: '语音设置', icon: Mic },
            ].map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as any)}
                className={`flex items-center gap-apple-2 px-apple-6 py-apple-3 text-apple-sm font-medium transition-colors border-b-2 ${
                  activeTab === tab.id
                    ? 'text-apple-accent-blue border-apple-accent-blue'
                    : 'text-apple-text-secondary border-transparent hover:text-apple-text-primary'
                }`}
              >
                <tab.icon className="w-4 h-4" />
                {tab.label}
              </button>
            ))}
          </div>

          {/* Content */}
          <div className="overflow-y-auto max-h-[60vh] p-apple-6">
            {/* Providers Tab */}
            {activeTab === 'providers' && (
              <div className="space-y-apple-6">
                {/* Provider list */}
                {settings.providers.map((provider) => {
                  const info = providerInfo[provider.provider];
                  const providerModels = getModelsForProvider(provider.provider);
                  const testResult = testResults[provider.provider];

                  return (
                    <div
                      key={provider.provider}
                      className={`p-apple-4 rounded-apple-lg border ${
                        settings.activeProvider === provider.provider
                          ? 'border-apple-accent-blue bg-apple-accent-blue/5'
                          : 'border-apple-border bg-apple-bg-secondary'
                      }`}
                    >
                      {/* Provider header */}
                      <div className="flex items-center justify-between mb-apple-4">
                        <div className="flex items-center gap-apple-3">
                          <span className="text-apple-xl">{info.icon}</span>
                          <div>
                            <h3 className="text-apple-sm font-semibold text-apple-text-primary">
                              {info.name}
                            </h3>
                            <p className="text-apple-xs text-apple-text-tertiary">
                              {providerModels.length} 个可用模型
                            </p>
                          </div>
                        </div>

                        <div className="flex items-center gap-apple-2">
                          {/* Enable toggle */}
                          <button
                            onClick={() =>
                              uiState.updateProviderConfig(provider.provider, {
                                enabled: !provider.enabled,
                              })
                            }
                            className={`relative w-11 h-6 rounded-apple-full transition-colors ${
                              provider.enabled ? 'bg-apple-accent-blue' : 'bg-apple-bg-tertiary'
                            }`}
                          >
                            <span
                              className={`absolute top-1 w-4 h-4 rounded-apple-full bg-white transition-transform ${
                                provider.enabled ? 'left-6' : 'left-1'
                              }`}
                            />
                          </button>

                          {/* Set as active */}
                          {provider.enabled && (
                            <button
                              onClick={() => uiState.setActiveProvider(provider.provider)}
                              className={`px-apple-3 py-apple-1.5 rounded-apple-md text-apple-xs font-medium transition-colors ${
                                settings.activeProvider === provider.provider
                                  ? 'bg-apple-accent-blue text-white'
                                  : 'bg-apple-bg-tertiary text-apple-text-secondary hover:bg-apple-bg-primary'
                              }`}
                            >
                              {settings.activeProvider === provider.provider ? '使用中' : '使用'}
                            </button>
                          )}
                        </div>
                      </div>

                      {provider.enabled && (
                        <div className="space-y-apple-4">
                          {/* API Key (not for local) */}
                          {provider.provider !== 'local' && (
                            <div>
                              <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
                                API 密钥
                              </label>
                              <div className="relative">
                                <input
                                  type={showApiKey[provider.provider] ? 'text' : 'password'}
                                  value={provider.apiKey}
                                  onChange={(e) =>
                                    uiState.updateProviderConfig(provider.provider, {
                                      apiKey: e.target.value,
                                    })
                                  }
                                  placeholder={`输入 ${info.name} API 密钥`}
                                  className="w-full pl-apple-3 pr-apple-10 py-apple-2 bg-apple-bg-primary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary focus:border-apple-accent-blue focus:outline-none"
                                />
                                <button
                                  onClick={() =>
                                    setShowApiKey((prev) => ({
                                      ...prev,
                                      [provider.provider]: !prev[provider.provider],
                                    }))
                                  }
                                  className="absolute right-apple-3 top-1/2 -translate-y-1/2 text-apple-text-tertiary hover:text-apple-text-primary"
                                >
                                  {showApiKey[provider.provider] ? '隐藏' : '显示'}
                                </button>
                              </div>
                            </div>
                          )}

                          {/* Endpoint (for local/custom) */}
                          {(provider.provider === 'local' || provider.provider === 'custom') && (
                            <div>
                              <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
                                API 端点
                              </label>
                              <input
                                type="text"
                                value={provider.endpoint}
                                onChange={(e) =>
                                  uiState.updateProviderConfig(provider.provider, {
                                    endpoint: e.target.value,
                                  })
                                }
                                placeholder="http://localhost:11434"
                                className="w-full px-apple-3 py-apple-2 bg-apple-bg-primary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary focus:border-apple-accent-blue focus:outline-none"
                              />
                            </div>
                          )}

                          {/* Default model */}
                          <div>
                            <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
                              默认模型
                            </label>
                            <select
                              value={provider.defaultModel}
                              onChange={(e) =>
                                uiState.updateProviderConfig(provider.provider, {
                                  defaultModel: e.target.value,
                                })
                              }
                              className="w-full px-apple-3 py-apple-2 bg-apple-bg-primary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary focus:border-apple-accent-blue focus:outline-none"
                            >
                              {providerModels.map((model) => (
                                <option key={model.id} value={model.id}>
                                  {model.name} - {model.description}
                                </option>
                              ))}
                            </select>
                          </div>

                          {/* Test connection */}
                          <div className="flex items-center gap-apple-3">
                            <button
                              onClick={() => handleTestProvider(provider.provider)}
                              className="px-apple-4 py-apple-2 bg-apple-bg-tertiary hover:bg-apple-bg-primary rounded-apple-lg text-apple-sm text-apple-text-primary transition-colors"
                            >
                              测试连接
                            </button>
                            {testResult === 'success' && (
                              <span className="flex items-center gap-apple-1 text-apple-sm text-apple-accent-green">
                                <Check className="w-4 h-4" />
                                连接成功
                              </span>
                            )}
                            {testResult === 'error' && (
                              <span className="flex items-center gap-apple-1 text-apple-sm text-apple-accent-red">
                                <AlertCircle className="w-4 h-4" />
                                连接失败
                              </span>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}

            {/* General Tab */}
            {activeTab === 'general' && (
              <div className="space-y-apple-6">
                {/* Temperature */}
                <div>
                  <label className="flex items-center justify-between text-apple-sm font-medium text-apple-text-primary mb-apple-2">
                    <span>温度 (Temperature)</span>
                    <span className="text-apple-accent-blue">{settings.temperature}</span>
                  </label>
                  <input
                    type="range"
                    min="0"
                    max="2"
                    step="0.1"
                    value={settings.temperature}
                    onChange={(e) =>
                      uiState.updateSettings({ temperature: parseFloat(e.target.value) })
                    }
                    className="w-full"
                  />
                  <p className="text-apple-xs text-apple-text-tertiary mt-apple-1">
                    较低的值使输出更确定，较高的值使输出更随机
                  </p>
                </div>

                {/* Max tokens */}
                <div>
                  <label className="flex items-center justify-between text-apple-sm font-medium text-apple-text-primary mb-apple-2">
                    <span>最大令牌数</span>
                    <span className="text-apple-accent-blue">{settings.maxTokens}</span>
                  </label>
                  <input
                    type="range"
                    min="256"
                    max="8192"
                    step="256"
                    value={settings.maxTokens}
                    onChange={(e) =>
                      uiState.updateSettings({ maxTokens: parseInt(e.target.value) })
                    }
                    className="w-full"
                  />
                </div>

                {/* Display options */}
                <div className="space-y-apple-3">
                  <label className="flex items-center gap-apple-3 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={settings.showTokens}
                      onChange={(e) => uiState.updateSettings({ showTokens: e.target.checked })}
                      className="w-4 h-4 rounded-apple-sm border-apple-border text-apple-accent-blue focus:ring-apple-accent-blue"
                    />
                    <span className="text-apple-sm text-apple-text-primary">显示令牌使用量</span>
                  </label>

                  <label className="flex items-center gap-apple-3 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={settings.autoSpeak}
                      onChange={(e) => uiState.updateSettings({ autoSpeak: e.target.checked })}
                      className="w-4 h-4 rounded-apple-sm border-apple-border text-apple-accent-blue focus:ring-apple-accent-blue"
                    />
                    <span className="text-apple-sm text-apple-text-primary">自动播报 AI 回复</span>
                  </label>
                </div>

                {/* Default export format */}
                <div>
                  <label className="block text-apple-sm font-medium text-apple-text-primary mb-apple-2">
                    默认导出格式
                  </label>
                  <select
                    value={settings.defaultExportFormat}
                    onChange={(e) =>
                      uiState.updateSettings({
                        defaultExportFormat: e.target.value as 'markdown' | 'pdf' | 'txt',
                      })
                    }
                    className="w-full px-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary focus:border-apple-accent-blue focus:outline-none"
                  >
                    <option value="markdown">Markdown</option>
                    <option value="pdf">PDF</option>
                    <option value="txt">纯文本</option>
                  </select>
                </div>

                {/* History retention */}
                <div>
                  <label className="block text-apple-sm font-medium text-apple-text-primary mb-apple-2">
                    历史记录保留天数
                  </label>
                  <select
                    value={settings.historyRetentionDays}
                    onChange={(e) =>
                      uiState.updateSettings({
                        historyRetentionDays: parseInt(e.target.value),
                      })
                    }
                    className="w-full px-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary focus:border-apple-accent-blue focus:outline-none"
                  >
                    <option value={7}>7 天</option>
                    <option value={30}>30 天</option>
                    <option value={90}>90 天</option>
                    <option value={365}>1 年</option>
                    <option value={0}>永久保留</option>
                  </select>
                </div>
              </div>
            )}

            {/* Voice Tab */}
            {activeTab === 'voice' && (
              <div className="space-y-apple-6">
                {/* Voice synthesis */}
                <div>
                  <h3 className="text-apple-sm font-semibold text-apple-text-primary mb-apple-4">
                    语音播报 (TTS)
                  </h3>

                  <div className="space-y-apple-4">
                    <div>
                      <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
                        语音引擎
                      </label>
                      <select
                        value={settings.voice.provider}
                        onChange={(e) =>
                          uiState.updateSettings({
                            voice: { ...settings.voice, provider: e.target.value as any },
                          })
                        }
                        className="w-full px-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary focus:border-apple-accent-blue focus:outline-none"
                      >
                        <option value="browser">浏览器内置</option>
                        <option value="elevenlabs">ElevenLabs</option>
                        <option value="azure">Azure Speech</option>
                      </select>
                    </div>

                    {settings.voice.provider === 'elevenlabs' && (
                      <div>
                        <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
                          ElevenLabs API 密钥
                        </label>
                        <input
                          type="password"
                          placeholder="sk-..."
                          className="w-full px-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary placeholder:text-apple-text-tertiary focus:border-apple-accent-blue focus:outline-none"
                        />
                      </div>
                    )}

                    <div>
                      <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
                        语速
                      </label>
                      <input
                        type="range"
                        min="0.5"
                        max="2"
                        step="0.1"
                        value={settings.voice.speed}
                        onChange={(e) =>
                          uiState.updateSettings({
                            voice: { ...settings.voice, speed: parseFloat(e.target.value) },
                          })
                        }
                        className="w-full"
                      />
                    </div>
                  </div>
                </div>

                {/* Speech recognition */}
                <div className="pt-apple-4 border-t border-apple-border">
                  <h3 className="text-apple-sm font-semibold text-apple-text-primary mb-apple-4">
                    语音识别 (STT)
                  </h3>

                  <div className="space-y-apple-4">
                    <div>
                      <label className="block text-apple-xs font-medium text-apple-text-secondary mb-apple-2">
                        识别语言
                      </label>
                      <select
                        value={settings.speech.language}
                        onChange={(e) =>
                          uiState.updateSettings({
                            speech: { ...settings.speech, language: e.target.value },
                          })
                        }
                        className="w-full px-apple-3 py-apple-2 bg-apple-bg-secondary border border-apple-border rounded-apple-lg text-apple-sm text-apple-text-primary focus:border-apple-accent-blue focus:outline-none"
                      >
                        <option value="zh-CN">中文 (简体)</option>
                        <option value="zh-TW">中文 (繁体)</option>
                        <option value="en-US">English (US)</option>
                        <option value="en-GB">English (UK)</option>
                        <option value="ja-JP">日本語</option>
                        <option value="ko-KR">한국어</option>
                        <option value="fr-FR">Français</option>
                        <option value="de-DE">Deutsch</option>
                        <option value="es-ES">Español</option>
                      </select>
                    </div>

                    <label className="flex items-center gap-apple-3 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={settings.speech.interimResults}
                        onChange={(e) =>
                          uiState.updateSettings({
                            speech: { ...settings.speech, interimResults: e.target.checked },
                          })
                        }
                        className="w-4 h-4 rounded-apple-sm border-apple-border text-apple-accent-blue focus:ring-apple-accent-blue"
                      />
                      <span className="text-apple-sm text-apple-text-primary">显示临时识别结果</span>
                    </label>

                    <label className="flex items-center gap-apple-3 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={settings.speech.continuous}
                        onChange={(e) =>
                          uiState.updateSettings({
                            speech: { ...settings.speech, continuous: e.target.checked },
                          })
                        }
                        className="w-4 h-4 rounded-apple-sm border-apple-border text-apple-accent-blue focus:ring-apple-accent-blue"
                      />
                      <span className="text-apple-sm text-apple-text-primary">持续监听模式</span>
                    </label>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-apple-3 px-apple-6 py-apple-4 border-t border-apple-border bg-apple-bg-secondary">
            <button
              onClick={uiState.closeSettings}
              className="px-apple-4 py-apple-2 text-apple-sm font-medium text-apple-text-secondary hover:text-apple-text-primary transition-colors"
            >
              取消
            </button>
            <button
              onClick={uiState.closeSettings}
              className="px-apple-4 py-apple-2 bg-apple-accent-blue text-white rounded-apple-lg text-apple-sm font-medium hover:bg-apple-accent-blue/90 transition-colors"
            >
              保存设置
            </button>
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
};
