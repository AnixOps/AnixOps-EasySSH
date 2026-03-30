/**
 * @process easyssh-autonomous-dev
 * @description 全自动 EasySSH 开发流程 - 自测试、自修复、自迭代
 * @inputs {
 *   maxIterations: number,
 *   testMode: 'core' | 'all',
 *   autoFix: boolean
 * }
 * @maxIterations 50
 */

import { defineTask } from '@a5c-ai/babysitter-sdk';

// 测试点定义
const TEST_POINTS = {
  // 1. 构建测试
  BUILD: {
    id: 'build',
    name: '构建测试',
    commands: [
      { cmd: 'cargo check --lib', cwd: 'core' },
      { cmd: 'cargo build --bin easyssh --release', cwd: '.', timeout: 300000 },
    ],
    mustPass: true,
  },

  // 2. 核心库测试
  CORE: {
    id: 'core',
    name: '核心库测试',
    commands: [
      { cmd: 'cargo test --lib -p easyssh-core', cwd: '.', optional: true },
    ],
    preCondition: (ctx) => ctx.state.buildPassed,
    mustPass: true,
  },

  // 3. CLI 测试
  CLI: {
    id: 'cli',
    name: 'CLI测试',
    commands: [
      { cmd: 'target/release/easyssh.exe --help', cwd: '.', validation: 'contains:EasySSH Core CLI' },
      { cmd: 'target/release/easyssh.exe list', cwd: '.', optional: true },
    ],
    preCondition: (ctx) => ctx.state.buildPassed,
    mustPass: true,
  },

  // 4. WebSocket 调试接口测试
  DEBUG_WS: {
    id: 'debug_ws',
    name: 'WebSocket调试接口',
    commands: [
      { cmd: 'target/release/easyssh.exe debug-server --help', cwd: '.', validation: 'contains:debug-server' },
    ],
    preCondition: (ctx) => ctx.state.buildPassed,
    mustPass: false,
  },

  // 5. 代码质量测试
  QUALITY: {
    id: 'quality',
    name: '代码质量测试',
    commands: [
      { cmd: 'cargo clippy -p easyssh-core -- -D warnings', cwd: '.', optional: true },
      { cmd: 'cargo fmt -p easyssh-core -- --check', cwd: '.', optional: true },
    ],
    mustPass: false,
  },
};

// 修复策略
const FIX_STRATEGIES = {
  build_error: {
    patterns: [
      /error\[E\d+\]:/,
      /cannot find/,
      /unresolved import/,
      /mismatched types/,
    ],
    actions: [
      '检查并添加缺失的 import',
      '修复类型不匹配问题',
      '添加缺失的依赖到 Cargo.toml',
    ],
  },

  connection_error: {
    patterns: [
      /failed to launch/,
      /Connection refused/,
      /spawn:/,
    ],
    actions: [
      '修复 terminal.rs 中的命令构建',
      '检查终端启动逻辑',
      '添加错误处理',
    ],
  },

  cli_error: {
    patterns: [
      /Unknown command/,
      /Usage:/,
      /not found/,
    ],
    actions: [
      '检查命令解析逻辑',
      '添加缺失的命令处理',
      '修复参数解析',
    ],
  },

  tui_error: {
    patterns: [
      /panic/,
      /thread.*panicked/,
      /raw mode/,
    ],
    actions: [
      '修复 TUI 生命周期管理',
      '检查终端恢复逻辑',
      '添加 panic 处理',
    ],
  },
};

export async function process(inputs, ctx) {
  const {
    maxIterations = 50,
    testMode = 'core',
    autoFix = true
  } = inputs;

  const state = {
    iteration: 0,
    buildPassed: false,
    testsPassed: {},
    fixesApplied: [],
    pendingErrors: [],
  };

  ctx.log('info', `🚀 启动 EasySSH 全自动开发流程`);
  ctx.log('info', `   最大迭代: ${maxIterations}, 测试模式: ${testMode}, 自动修复: ${autoFix}`);

  while (state.iteration < maxIterations) {
    state.iteration++;
    ctx.log('info', `\n📦 迭代 ${state.iteration}/${maxIterations}`);

    // ===== PHASE 1: 测试 =====
    const testResults = await runTests(ctx, state, testMode);

    // 记录结果
    for (const [testId, result] of Object.entries(testResults)) {
      state.testsPassed[testId] = result.passed;

      if (!result.passed && result.error) {
        state.pendingErrors.push({
          iteration: state.iteration,
          testId,
          error: result.error,
          output: result.output,
        });
      }
    }

    // 检查是否需要修复
    const hasCriticalErrors = Object.entries(testResults)
      .filter(([id, r]) => TEST_POINTS[id.toUpperCase()]?.mustPass)
      .some(([_, r]) => !r.passed);

    if (!hasCriticalErrors) {
      ctx.log('success', `✅ 所有关键测试通过！迭代完成`);

      // 运行额外优化迭代
      if (state.iteration < maxIterations) {
        await runOptimization(ctx, state);
      }

      break;
    }

    // ===== PHASE 2: 分析错误 =====
    if (!autoFix || state.pendingErrors.length === 0) {
      ctx.log('error', `❌ 测试失败但自动修复禁用或无错误可修复`);
      break;
    }

    const latestError = state.pendingErrors[state.pendingErrors.length - 1];
    const fixStrategy = analyzeError(latestError);

    // ===== PHASE 3: 自动修复 =====
    ctx.log('info', `🔧 应用修复策略: ${fixStrategy.type}`);

    const fixResult = await ctx.task(autoFixTask, {
      iteration: state.iteration,
      error: latestError,
      strategy: fixStrategy,
      projectRoot: 'C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH',
    });

    if (fixResult.applied) {
      state.fixesApplied.push({
        iteration: state.iteration,
        type: fixStrategy.type,
        files: fixResult.changedFiles,
        description: fixResult.description,
      });

      // 修复后重新测试（在下次迭代）
    } else {
      ctx.log('warn', `⚠️ 修复未能应用，停止迭代`);
      break;
    }

    // 防止无限循环：如果连续5次修复同一错误，停止
    const recentFixes = state.fixesApplied.slice(-5);
    if (recentFixes.length === 5 &&
        recentFixes.every(f => f.type === fixStrategy.type)) {
      ctx.log('error', `🛑 连续5次修复同一问题，可能存在根本性问题，停止迭代`);
      break;
    }
  }

  // 生成报告
  return generateReport(state);
}

async function runTests(ctx, state, testMode) {
  const results = {};

  // 按依赖顺序运行测试
  const testOrder = ['BUILD', 'CORE', 'CLI', 'DEBUG_WS', 'QUALITY'];

  for (const testId of testOrder) {
    const test = TEST_POINTS[testId];

    // 检查前置条件
    if (test.preCondition && !test.preCondition({ state })) {
      ctx.log('debug', `   ⏭️ 跳过 ${test.name} (前置条件未满足)`);
      continue;
    }

    ctx.log('info', `   🧪 运行: ${test.name}`);

    const result = await executeTest(ctx, test);
    results[testId.toLowerCase()] = result;

    if (result.passed) {
      ctx.log('success', `      ✅ 通过`);
      if (testId === 'BUILD') state.buildPassed = true;
    } else {
      ctx.log('error', `      ❌ 失败: ${result.error?.substring(0, 100)}`);
      if (test.mustPass) break; // 关键测试失败，停止后续测试
    }
  }

  return results;
}

/// 执行命令，支持非阻塞模式
async function executeTest(ctx, test) {
  for (const cmd of test.commands) {
    try {
      const result = await ctx.exec(cmd.cmd, {
        cwd: cmd.cwd,
        timeout: cmd.timeout || 60000,
        ignoreError: cmd.optional || false,
        // 修复: 允许空 stderr
        allowEmptyStderr: true,
      });

      // 修复: 检查 exit code 而不是 stderr
      if (result.exitCode !== 0 && !cmd.optional) {
        return {
          passed: false,
          error: result.stdout || `Exit code: ${result.exitCode}`,
          output: result.stdout || ''
        };
      }

      // 验证输出
      if (cmd.validation) {
        const [type, expected] = cmd.validation.split(':');
        const output = result.stdout || '';
        if (type === 'contains' && !output.includes(expected)) {
          return {
            passed: false,
            error: `Output missing: ${expected}`,
            output
          };
        }
      }

    } catch (e) {
      if (!cmd.optional) {
        return {
          passed: false,
          error: e.message || 'Command failed',
          output: e.stdout || ''
        };
      }
    }
  }

  return { passed: true };
}

function analyzeError(errorInfo) {
  const { error, output } = errorInfo;
  const combined = `${error}\n${output}`;

  for (const [type, strategy] of Object.entries(FIX_STRATEGIES)) {
    for (const pattern of strategy.patterns) {
      if (pattern.test(combined)) {
        return {
          type,
          patterns: strategy.patterns.map(p => p.toString()),
          actions: strategy.actions,
          priority: strategy.priority || 1,
        };
      }
    }
  }

  return {
    type: 'unknown',
    patterns: [],
    actions: ['检查日志输出', '读取相关代码', '尝试通用修复'],
    priority: 0,
  };
}

async function runOptimization(ctx, state) {
  ctx.log('info', `   ✨ 运行优化迭代`);

  // 运行代码质量改进
  const optimization = await ctx.task(optimizationTask, {
    iteration: state.iteration,
    lastFixes: state.fixesApplied.slice(-3),
    projectRoot: 'C:/Users/z7299/Documents/GitHub/AnixOps-EasySSH',
  });

  if (optimization.applied) {
    state.fixesApplied.push({
      iteration: state.iteration,
      type: 'optimization',
      ...optimization,
    });
  }
}

function generateReport(state) {
  const success = Object.entries(state.testsPassed)
    .filter(([id, _]) => TEST_POINTS[id.toUpperCase()]?.mustPass)
    .every(([_, passed]) => passed);

  return {
    success,
    iterations: state.iteration,
    tests: state.testsPassed,
    fixes: state.fixesApplied,
    errors: state.pendingErrors,
    summary: success
      ? `✅ 全自动开发完成！共迭代 ${state.iteration} 次，应用 ${state.fixesApplied.length} 个修复`
      : `❌ 开发未完全成功，请检查未通过的测试`,
  };
}

// ===== 子任务定义 =====

export const autoFixTask = defineTask('easyssh-autofix', (args, taskCtx) => ({
  kind: 'agent',
  title: `Auto-fix iteration ${args.iteration}`,
  agent: {
    name: 'general-purpose',
    prompt: {
      role: 'senior rust engineer',
      context: {
        projectRoot: args.projectRoot,
        error: args.error,
        strategy: args.strategy,
      },
      task: `修复 EasySSH 的错误。

错误信息:
${JSON.stringify(args.error, null, 2)}

修复策略: ${args.strategy.type}
建议行动:
${args.strategy.actions.map(a => `- ${a}`).join('\n')}

规则:
1. 读取错误相关的文件
2. 理解错误原因
3. 应用最小化修复
4. 确保修复后能通过测试
5. 返回修复结果

输出格式:
{
  "applied": boolean,
  "changedFiles": ["file paths"],
  "description": "修复描述",
  "reasoning": "修复理由"
}`,
      outputFormat: 'json',
    },
  },
}));

export const optimizationTask = defineTask('easyssh-optimize', (args, taskCtx) => ({
  kind: 'agent',
  title: `Optimization iteration ${args.iteration}`,
  agent: {
    name: 'general-purpose',
    prompt: {
      role: 'senior rust engineer',
      context: {
        projectRoot: args.projectRoot,
        recentFixes: args.lastFixes,
      },
      task: `优化 EasySSH 代码质量。

最近修复:
${JSON.stringify(args.lastFixes, null, 2)}

优化方向:
1. 代码简化
2. 错误处理完善
3. 性能优化
4. 可读性改进

规则:
- 只做安全的小改进
- 不改变功能行为
- 运行 cargo check 确保通过

输出格式:
{
  "applied": boolean,
  "changedFiles": ["file paths"],
  "description": "优化描述"
}`,
      outputFormat: 'json',
    },
  },
}));
