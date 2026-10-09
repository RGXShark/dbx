# 企业 AI 桌面安装包

此功能仅用于桌面版，Web/Docker 不启用企业策略。企业版的个人 Key 由 IT 在总部 New API 平台上创建、登记和撤销，无需新增网关或 aTrust 登录集成。

## 打包

1. 安装项目文档要求的 Node、pnpm、Rust、Windows C++ 构建工具和 Tauri 打包依赖。
2. 将 `config/enterprise-ai.example.json` 复制为 `config/enterprise-ai.json`。
3. 填写服务名称、OpenAI 兼容 API 基础地址、允许的模型 ID 和默认模型。**不要填写 API Key**。
4. 执行 `pnpm install --frozen-lockfile`，然后 `pnpm build:enterprise`。
5. 将生成的安装包交给业务人员，另行下发每人独立的 Key。

```json
{
  "schemaVersion": 1,
  "serviceName": "公司 AI 服务",
  "endpoint": "https://ai.company.internal/v1",
  "models": ["model-a", "model-b"],
  "defaultModel": "model-a"
}
```

所有字段必填。地址允许 HTTP/HTTPS，不允许用户凭据、查询参数和片段；模型列表非空且不重复，默认模型必须在列表中。使用 Chat Completions、Bearer 认证；Agent 使用的模型必须支持工具调用和流式回复。

实际配置被 Git 忽略，仅示例提交到仓库。Rust 编译时校验并嵌入配置，安装后不读取此文件。显式企业构建缺失文件或校验失败会中止，不能回退到普通模式。

常规 `pnpm tauri build` 在没有 `DBX_ENTERPRISE_AI_CONFIG` 环境变量时仍构建原版。需要自定义配置路径或开发调试时，在启动 Cargo/Tauri 前将该变量设为配置文件绝对路径。不要在生产安装包中使用示例地址和模型。

## 员工使用

打开 AI 设置，输入 IT 发放的个人 Key，保存后选择允许的模型并测试连接。保存后的 Key 不回显，可以更新或清除。接口、供应商、模型范围和认证请求头不可编辑。普通问数、Ask/Agent 和 SQL 写入确认流程继续保留。

旧 AI 配置不删除，也不自动迁移 Key。企业个人 Key 在独立、绑定接口地址的命名空间中加密保存，不进入普通配置导出或云同步。更换接口地址后必须重新输入 Key；仅修改模型列表不需要更换 Key。模型被移除后自动选择新的默认模型。

凭据存储复用 DBX 的 SQLite 密文和系统凭据存储保护的加密密钥。此机制限制应用内配置，不能保证个人 Key 无法被电脑使用者提取。整盘、整个数据目录或原始数据库备份应按包含凭据的备份管理。

## 更新与撤销

企业版关闭官方主程序检查、下载、缓存安装和 updater 插件，IT 合并上游更新后重新打包分发。数据库驱动和插件更新不受影响。修改实际配置后重新执行企业打包，Cargo 会重新嵌入配置。

禁用 aTrust 账号不会自动撤销 New API Key。IT 必须在总部平台撤销个人 Key；如果没有撤销权限，由总部协助。

开发在 `feature/enterprise-ai-config` 分支进行，`main` 保留上游跟踪，不直接合并企业改动。

## 验证

前端针对企业状态、组件交互和原版设置回归运行 Vitest；完整类型检查使用 `pnpm typecheck`，前端构建使用 `pnpm build`。Rust 测试位于企业策略、配置解析及构建嵌入模块，覆盖真实本地 HTTP 服务与加密存储，不使用总部的真实 Key。

Windows 的桌面 lib 测试程序可能因缺少 Common Controls v6 清单，以 `STATUS_ENTRYPOINT_NOT_FOUND` 退出。正式 Tauri 应用带有自己的清单。可在 Visual Studio 开发者 PowerShell 中对测试产物应用仓库提供的测试清单后运行：

```powershell
cargo test -p dbx --no-default-features --features sqlite-bundled --lib --no-run
$testBinary = (Get-ChildItem target/debug/deps/dbx_lib-*.exe | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
mt.exe -nologo -manifest src-tauri/windows/tests.manifest "-outputresource:$testBinary;#1"
& $testBinary enterprise_ai
```

正式分发前，IT 需填入真实基础地址与模型 ID，并使用测试员工 Key 验证总部平台的权限、流式回复和工具调用。本地示例安装包不代表已完成总部平台联调。
