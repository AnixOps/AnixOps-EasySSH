# AI全自动编程基础设施

> ⚠️ **仅debug模式编译，release版本不包含此功能**

---

## 1. 设计理念：AI自我改进闭环

```
┌─────────────────────────────────────────────────────────────────────┐
│                      AI Self-Improvement Loop                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│    ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐ │
│    │  Observe │ ──▶ │ Analyze  │ ──▶ │   Plan   │ ──▶ │  Modify  │ │
│    │  代码状态 │     │  问题定位 │     │  修复方案 │     │  改代码  │ │
│    └──────────┘     └──────────┘     └──────────┘     └──────────┘ │
│         ▲                                                       │ │
│         │                                                       ▼ │
│    ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐ │
│    │  Verify  │ ◀── │  Report  │ ◀── │  Execute │ ◀── │   Test   │ │
│    │  验证结果 │     │  生成报告 │     │  执行修复 │     │  跑测试  │ │
│    └──────────┘     └──────────┘     └──────────┘     └──────────┘ │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. AI编程Agent架构

### 2.1 核心组件

```rust
#[cfg(debug_assertions)]
pub mod ai_programming {

    // AI Agent状态
    pub struct ProgrammingAgent {
        pub id: AgentId,
        pub model: LLMModel,

        // 上下文
        pub code_context: CodeContext,
        pub task_history: Vec<Task>,

        // 能力边界
        pub permissions: AgentPermissions,
    }

    // Agent权限级别
    #[derive(Debug, Clone)]
    pub struct AgentPermissions {
        pub read_files: bool,        // 读取源代码
        pub write_files: bool,        // 修改代码
        pub run_tests: bool,          // 运行测试
        pub run_commands: bool,       // 执行shell命令
        pub git_operations: bool,     // Git操作
        pub create_commits: bool,     // 创建提交
        pub max_iterations: usize,    // 最大迭代次数
        pub requires_approval: bool,   // 是否需要人工批准
    }

    // 默认权限 (保守)
    impl Default for AgentPermissions {
        fn default() -> Self {
            Self {
                read_files: true,
                write_files: true,
                run_tests: true,
                run_commands: true,
                git_operations: true,
                create_commits: false,  // 禁止自动提交
                max_iterations: 5,
                requires_approval: true, // 默认需要批准
            }
        }
    }
}
```

### 2.2 完整AI接口

```rust
#[cfg(debug_assertions)]
mod ai_commands {

    // ============ 代码理解 ============
    #[tauri::command]
    pub async fn ai_read_code(path: String) -> Result<String, String>;

    #[tauri::command]
    pub async fn ai_list_files(dir: String, pattern: Option<String>) -> Result<Vec<FileInfo>, String>;

    #[tauri::command]
    pub async fn ai_search_code(query: String, path: Option<String>) -> Result<Vec<SearchResult>, String>;

    #[tauri::command]
    pub async fn ai_understand_symbol(symbol: String) -> Result<SymbolInfo, String>;

    // ============ 代码修改 ============
    #[tauri::command]
    pub async fn ai_write_file(path: String, content: String) -> Result<(), String>;

    #[tauri::command]
    pub async fn ai_edit_file(
        path: String,
        old_string: String,
        new_string: String,
    ) -> Result<EditResult, String>;

    #[tauri::command]
    pub async fn ai_create_file(
        path: String,
        template: Option<String>,  // 可选模板
    ) -> Result<(), String>;

    #[tauri::command]
    pub async fn ai_delete_file(path: String) -> Result<(), String>;

    // ============ 测试执行 ============
    #[tauri::command]
    pub async fn ai_run_tests(
        filter: Option<String>,  // 测试过滤
        verbose: bool,
    ) -> Result<TestResult, String>;

    #[tauri::command]
    pub async fn ai_run_specific_test(test_path: String) -> Result<TestResult, String>;

    #[tauri::command]
    pub async fn ai_check_types() -> Result<TypeCheckResult, String>;

    #[tauri::command]
    pub async fn ai_lint() -> Result<LintResult, String>;

    // ============ 构建验证 ============
    #[tauri::command]
    pub async fn ai_build(target: Option<String>) -> Result<BuildResult, String>;

    #[tauri::command]
    pub async fn ai_incremental_build(paths: Vec<String>) -> Result<BuildResult, String>;

    // ============ Git操作 ============
    #[tauri::command]
    pub async fn ai_git_status() -> Result<GitStatus, String>;

    #[tauri::command]
    pub async fn ai_git_diff(path: Option<String>) -> Result<String, String>;

    #[tauri::command]
    pub async fn ai_git_commit(message: String, paths: Vec<String>) -> Result<CommitResult, String>;

    #[tauri::command]
    pub async fn ai_git_log(count: usize) -> Result<Vec<GitCommit>, String>;

    #[tauri::command]
    pub async fn ai_git_branch() -> Result<Vec<BranchInfo>, String>;

    // ============ AI自动化循环 ============
    #[tauri::command]
    pub async fn ai_self_fix(
        problem_description: String,
        max_iterations: Option<usize>,
    ) -> Result<SelfFixResult, String>;

    #[tauri::command]
    pub async fn ai_self_test(
        feature_description: String,
    ) -> Result<SelfTestResult, String>;

    #[tauri::command]
    pub async fn ai_self_refactor(
        target: String,
        goal: String,
    ) -> Result<RefactorResult, String>;

    #[tauri::command]
    pub async fn ai_self_optimize(
        target: String,  // "performance" | "memory" | "lines"
    ) -> Result<OptimizationResult, String>;

    // ============ 任务执行 ============
    #[tauri::command]
    pub async fn ai_execute_task(
        task: Task,
        permissions: AgentPermissions,
    ) -> Result<TaskResult, String>;

    #[tauri::command]
    pub async fn ai_plan_task(
        goal: String,
        context: Option<String>,
    ) -> Result<TaskPlan, String>;

    // ============ 上下文管理 ============
    #[tauri::command]
    pub async fn ai_set_context(key: String, value: String) -> Result<(), String>;

    #[tauri::command]
    pub async fn ai_get_context(key: String) -> Result<Option<String>, String>;

    #[tauri::command]
    pub async fn ai_clear_context() -> Result<(), String>;

    // ============ 状态查询 ============
    #[tauri::command]
    pub async fn ai_get_capabilities() -> Result<AgentCapabilities, String>;

    #[tauri::command]
    pub async fn ai_get_status() -> Result<AgentStatus, String>;

    #[tauri::command]
    pub async fn ai_cancel_task(task_id: String) -> Result<(), String>;
}
```

---

## 3. AI自我修复闭环

### 3.1 自我修复流程

```rust
pub async fn ai_self_fix(
    problem_description: String,
    max_iterations: Option<usize>,
) -> Result<SelfFixResult, String> {

    let max_iterations = max_iterations.unwrap_or(5);
    let mut attempts = Vec::new();

    for iteration in 0..max_iterations {
        // 1. 分析问题
        let analysis = analyze_problem(&problem_description, &attempts).await?;

        if analysis.root_cause.is_none() {
            return Ok(SelfFixResult {
                success: false,
                iterations: attempts.len(),
                attempts,
                error: Some("Could not determine root cause".into()),
            });
        }

        // 2. 生成修复
        let fix = generate_fix(&analysis).await?;

        // 3. 应用修复
        let edit_result = apply_fix(&fix).await?;

        if !edit_result.success {
            attempts.push(FixAttempt {
                iteration,
                fix: fix.clone(),
                result: edit_result.clone(),
                status: "apply_failed".into(),
            });
            continue;
        }

        // 4. 运行测试验证
        let test_result = run_relevant_tests(&edit_result.modified_files).await?;

        if test_result.all_passed {
            return Ok(SelfFixResult {
                success: true,
                iterations: attempts.len(),
                attempts,
                error: None,
            });
        }

        // 5. 记录失败并继续
        attempts.push(FixAttempt {
            iteration,
            fix: fix.clone(),
            result: edit_result,
            status: "test_failed".into(),
            test_failures: Some(test_result.failures),
        });

        // 6. 更新问题描述，加入失败信息
        problem_description = update_problem_description(
            problem_description,
            &attempts,
        ).await?;
    }

    Ok(SelfFixResult {
        success: false,
        iterations: attempts.len(),
        attempts,
        error: Some("Max iterations reached".into()),
    })
}

struct FixAttempt {
    iteration: usize,
    fix: CodeFix,
    result: EditResult,
    status: String,
    test_failures: Option<Vec<TestFailure>>,
}

struct SelfFixResult {
    success: bool,
    iterations: usize,
    attempts: Vec<FixAttempt>,
    error: Option<String>,
}
```

### 3.2 修复生成策略

```rust
enum FixStrategy {
    // 1. 语法错误修复
    SyntaxError {
        location: FileLocation,
        expected: String,
        found: String,
    },

    // 2. 类型错误修复
    TypeError {
        location: FileLocation,
        expected_type: String,
        found_type: String,
        suggestion: TypeSuggestion,
    },

    // 3. 逻辑错误修复
    LogicError {
        location: FileLocation,
        description: String,
        fix_description: String,
    },

    // 4. 测试失败修复
    TestFailure {
        test_path: String,
        assertion: String,
        suggestion: String,
    },

    // 5. 性能问题修复
    PerformanceIssue {
        location: FileLocation,
        issue: String,
        suggestion: String,
    },

    // 6. 安全漏洞修复
    SecurityVulnerability {
        location: FileLocation,
        cve: Option<String>,
        fix: String,
    },
}

impl FixStrategy {
    fn generate_fix(&self) -> CodeFix {
        match self {
            FixStrategy::TypeError { found_type, expected_type, .. } => {
                // 智能类型推断修复
                let suggested_type = infer_correct_type(found_type, expected_type);
                CodeFix {
                    description: format!("Change type from {} to {}", found_type, suggested_type),
                    edits: vec![Edit {
                        file: self.location().file.clone(),
                        range: self.location().range.clone(),
                        new_content: suggested_type,
                    }],
                }
            }
            // ... 其他策略
        }
    }
}
```

---

## 4. AI自我测试

### 4.1 自动生成测试

```rust
pub async fn ai_self_test(feature_description: String) -> Result<SelfTestResult, String> {

    // 1. 理解功能
    let feature = understand_feature(&feature_description).await?;

    // 2. 分析现有测试
    let existing_tests = analyze_existing_tests(&feature).await?;

    // 3. 识别测试缺口
    let gaps = identify_test_gaps(&feature, &existing_tests);

    // 4. 生成新测试
    let new_tests = generate_tests(&gaps).await?;

    // 5. 写入测试文件
    let write_results = write_tests(&new_tests).await?;

    // 6. 运行测试验证
    let test_result = run_tests(&new_tests).await?;

    Ok(SelfTestResult {
        feature_description,
        existing_tests_count: existing_tests.len(),
        new_tests_added: new_tests.len(),
        test_results: test_result,
        coverage_delta: calculate_coverage_delta(&existing_tests, &new_tests),
    })
}

struct TestGeneration {
    pub test_name: String,
    pub test_body: String,
    pub assertions: Vec<Assertion>,
    pub edge_cases: Vec<String>,
    pub mock_requirements: Vec<String>,
}
```

### 4.2 智能测试选择

```rust
// 只运行受代码修改影响的测试
pub async fn ai_run_relevant_tests(
    modified_files: Vec<String>,
) -> Result<TestResult, String> {

    // 1. 构建依赖图
    let dependency_graph = build_dependency_graph().await?;

    // 2. 找出受影响的测试
    let affected_tests = dependency_graph.get_affected_tests(&modified_files);

    // 3. 按优先级排序
    let prioritized = prioritize_tests(&affected_tests);

    // 4. 并行执行
    let results = run_tests_parallel(prioritized).await?;

    Ok(results)
}
```

---

## 5. AI上下文理解

### 5.1 代码索引

```rust
pub struct CodeIndex {
    // 符号索引: symbol -> location
    symbols: HashMap<String, SymbolLocation>,

    // 文件索引: file -> symbols
    file_symbols: HashMap<String, Vec<String>>,

    // 依赖索引: file -> dependencies
    dependencies: HashMap<String, Vec<String>>,

    // 调用图: function -> callers
    call_graph: HashMap<String, Vec<String>>,
}

impl CodeIndex {
    pub async fn build(project_root: &Path) -> Result<Self, String> {
        // 1. 扫描所有源文件
        // 2. 解析符号定义
        // 3. 构建依赖关系
        // 4. 构建调用图
    }

    pub fn find_usages(&self, symbol: &str) -> Vec<Usage> {
        // 快速查找符号的所有使用位置
    }

    pub fn get_callers(&self, function: &str) -> Vec<String> {
        // 找出调用某个函数的所有函数
    }
}
```

### 5.2 语义理解

```rust
pub async fn ai_understand_code(
    target: &str,  // 函数名、类名、文件等
) -> Result<CodeUnderstanding, String> {

    let index = get_code_index().await?;

    Ok(CodeUnderstanding {
        definition: index.find_definition(target),
        usages: index.find_usages(target),
        doc_comment: extract_doc_comment(target).await?,
        type_info: get_type_info(target).await?,
        complexity: calculate_complexity(target),
        suggested_improvements: suggest_improvements(target),
    })
}
```

---

## 6. Claude Code工具定义 (完整版)

### 6.1 工具JSON Schema

```json
{
  "name": "easyssh-ai-programming",
  "description": "AI Full-Stack Programming Tools - Self-fix, Self-test, Self-refactor capabilities",

  "tools": [
    // ============ 代码读取 ============
    {
      "name": "read_source_code",
      "description": "Read source code from a file",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" },
          "line_start": { "type": "number" },
          "line_end": { "type": "number" }
        },
        "required": ["path"]
      }
    },
    {
      "name": "list_directory",
      "description": "List files in a directory with optional pattern filter",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" },
          "pattern": { "type": "string" },
          "recursive": { "type": "boolean", "default": false }
        },
        "required": ["path"]
      }
    },
    {
      "name": "search_in_codebase",
      "description": "Search for text pattern in code",
      "input_schema": {
        "type": "object",
        "properties": {
          "query": { "type": "string" },
          "path": { "type": "string" },
          "regex": { "type": "boolean", "default": false },
          "case_sensitive": { "type": "boolean", "default": true }
        },
        "required": ["query"]
      }
    },

    // ============ 代码修改 ============
    {
      "name": "edit_file",
      "description": "Edit a file by replacing old_string with new_string",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" },
          "old_string": { "type": "string" },
          "new_string": { "type": "string" }
        },
        "required": ["path", "old_string", "new_string"]
      }
    },
    {
      "name": "write_file",
      "description": "Write content to a file (overwrites existing)",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" },
          "content": { "type": "string" }
        },
        "required": ["path", "content"]
      }
    },
    {
      "name": "create_file",
      "description": "Create a new file with optional template",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" },
          "template": { "type": "string" }
        },
        "required": ["path"]
      }
    },
    {
      "name": "delete_file",
      "description": "Delete a file",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" },
          "force": { "type": "boolean", "default": false }
        },
        "required": ["path"]
      }
    },

    // ============ 测试执行 ============
    {
      "name": "run_tests",
      "description": "Run tests with optional filter",
      "input_schema": {
        "type": "object",
        "properties": {
          "filter": { "type": "string", "description": "Test name filter (supports * wildcard)" },
          "verbose": { "type": "boolean", "default": false },
          "coverage": { "type": "boolean", "default": false }
        }
      }
    },
    {
      "name": "run_type_check",
      "description": "Run TypeScript/Rust type checker",
      "input_schema": {
        "type": "object",
        "properties": {}
      }
    },
    {
      "name": "run_linter",
      "description": "Run linter with auto-fix",
      "input_schema": {
        "type": "object",
        "properties": {
          "fix": { "type": "boolean", "default": false }
        }
      }
    },
    {
      "name": "run_build",
      "description": "Build the project",
      "input_schema": {
        "type": "object",
        "properties": {
          "target": { "type": "string" }
        }
      }
    },

    // ============ Git操作 ============
    {
      "name": "git_status",
      "description": "Get git status",
      "input_schema": { "type": "object", "properties": {} }
    },
    {
      "name": "git_diff",
      "description": "Get git diff for a file or all changes",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" }
        }
      }
    },
    {
      "name": "git_commit",
      "description": "Create a git commit",
      "input_schema": {
        "type": "object",
        "properties": {
          "message": { "type": "string" },
          "files": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["message"]
      }
    },

    // ============ AI自动化循环 ============
    {
      "name": "ai_self_fix",
      "description": "AI self-diagnosis and self-fix loop",
      "input_schema": {
        "type": "object",
        "properties": {
          "problem": { "type": "string", "description": "Problem description or error message" },
          "max_iterations": { "type": "number", "default": 5 }
        },
        "required": ["problem"]
      }
    },
    {
      "name": "ai_self_test",
      "description": "AI automatically generates and runs tests for a feature",
      "input_schema": {
        "type": "object",
        "properties": {
          "feature": { "type": "string", "description": "Feature description" }
        },
        "required": ["feature"]
      }
    },
    {
      "name": "ai_self_refactor",
      "description": "AI refactors code to improve quality",
      "input_schema": {
        "type": "object",
        "properties": {
          "target": { "type": "string", "description": "File or function to refactor" },
          "goal": { "type": "string", "description": "Refactoring goal" }
        },
        "required": ["target", "goal"]
      }
    },
    {
      "name": "ai_explain_error",
      "description": "AI explains an error and suggests fixes",
      "input_schema": {
        "type": "object",
        "properties": {
          "error": { "type": "string" },
          "context": { "type": "string" }
        },
        "required": ["error"]
      }
    },

    // ============ 上下文管理 ============
    {
      "name": "ai_set_context",
      "description": "Set AI context variable for future reference",
      "input_schema": {
        "type": "object",
        "properties": {
          "key": { "type": "string" },
          "value": { "type": "string" }
        },
        "required": ["key", "value"]
      }
    },
    {
      "name": "ai_get_context",
      "description": "Get AI context variable",
      "input_schema": {
        "type": "object",
        "properties": {
          "key": { "type": "string" }
        },
        "required": ["key"]
      }
    },

    // ============ 项目分析 ============
    {
      "name": "analyze_project",
      "description": "Analyze project structure and dependencies",
      "input_schema": {
        "type": "object",
        "properties": {}
      }
    },
    {
      "name": "find_related_files",
      "description": "Find files related to a target",
      "input_schema": {
        "type": "object",
        "properties": {
          "target": { "type": "string" }
        },
        "required": ["target"]
      }
    },
    {
      "name": "get_code_metrics",
      "description": "Get code quality metrics",
      "input_schema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" }
        }
      }
    }
  ],

  "capabilities": {
    "self_fix": true,
    "self_test": true,
    "self_refactor": true,
    "auto_iterate": true,
    "context_aware": true
  }
}
```

---

## 7. 自我改进循环示例

### 7.1 完整循环流程

```
用户/AI: ai_self_fix("TypeScript error: Argument of type 'string | null' is not assignable to parameter of type 'string'")

 Iteration 1:
   ├─ Analyze: 问题在 auth.ts:42，参数可能是null
   ├─ Fix: 添加 null check
   ├─ Test: run_tests() → FAIL (2 tests failed)
   │
   ├─ Iteration 2:
   │  ├─ Analyze: null check改变了返回值类型
   │  ├─ Fix: 更新函数返回类型注解
   │  ├─ Test: run_tests() → FAIL (1 test failed)
   │  │
   │  ├─ Iteration 3:
   │  │  ├─ Analyze: 测试期望旧的行为
   │  │  ├─ Fix: 更新测试用例
   │  │  ├─ Test: run_tests() → PASS ✓
   │  │  └─ Success!
   │  │
   └─ Result: { success: true, iterations: 3 }
```

### 7.2 自动测试生成示例

```
用户/AI: ai_self_test("SSH connection timeout handling")

 1. Understand feature:
    - 建立SSH连接
    - 超时时应抛出 TimeoutError
    - 应支持可配置超时时间

 2. Generate tests:
    - test_connection_timeout_default()
    - test_connection_timeout_custom()
    - test_connection_success_after_timeout()
    - test_concurrent_connection_timeout()

 3. Write and run tests:
    - 4 tests added
    - 4 tests passed
    - Coverage: +15%

 4. Result:
    { new_tests: 4, passed: 4, coverage_delta: +15% }
```

---

## 8. 安全与审计

### 8.1 操作审计

```rust
struct AIAuditEntry {
    timestamp: DateTime<Utc>,
    agent_id: String,
    action: AIAction,
    target: String,
    result: ActionResult,
    approval: Option<Approval>,
}

struct Approval {
    approved_by: String,
    timestamp: DateTime<Utc>,
    reason: Option<String>,
}
```

### 8.2 权限配置

```rust
// Claude Code 配置
{
  "permissions": {
    "auto_write_files": false,    // 写文件需确认
    "auto_commit": false,         // 禁止自动提交
    "max_iterations": 5,          // 最多5次自我修复迭代
    "require_approval_for": [
      "delete_file",
      "git_commit",
      "ai_self_fix"  // 危险操作需确认
    ]
  }
}
```

---

## 9. 实现状态

| 功能 | 状态 | 说明 |
|------|------|------|
| 代码读取 | ✅ | read_source_code, list_directory, search_in_codebase |
| 代码修改 | ✅ | edit_file, write_file, create_file, delete_file |
| 测试执行 | ✅ | run_tests, run_type_check, run_linter |
| Git操作 | ✅ | git_status, git_diff, git_commit |
| 自我修复 | 🔄 | ai_self_fix 核心逻辑 |
| 自我测试 | 🔄 | ai_self_test 测试生成 |
| 自我重构 | 📋 | ai_self_refactor (规划中) |
| 上下文管理 | ✅ | ai_set_context, ai_get_context |
