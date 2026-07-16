# wuziqi_claude — 五子棋 (Gomoku)

用 Rust + [macroquad](https://github.com/not-fl3/macroquad) 编写的五子棋小游戏,支持人机对战和双人对战。附带一个逻辑完全一致的网页移植版,无需安装任何东西即可游玩。

## 玩法

- 🖱️ 鼠标点击交叉点落子(默认你执黑,AI 执白)
- `R` 重新开始
- `U` 悔棋(人机模式回退一整轮；AI 尚未回应时只回退玩家刚落的一步)
- `M` 切换 人机 / 双人 模式
- `A` 切换 经典战术搜索 / 大模型 AI（原生版）
- `C` 打开大模型配置页面（原生版）

## 运行

### 原生版 (Rust)

```bash
cargo run --release
```

### 大模型 AI（原生版）

大模型模式使用 OpenAI Responses API。点击顶部 `Config (C)` 或按 `C` 打开配置页面，可填写 API Key、模型名称和 API 地址；设置仅保存在当前游戏进程内，退出后清除。也可以在启动前通过环境变量提供默认值：

```bash
export OPENAI_API_KEY="你的 API Key"
# 可选，默认 gpt-5-mini
export OPENAI_MODEL="gpt-5-mini"
# 可选，默认 https://api.openai.com/v1/responses
export OPENAI_API_URL="https://api.openai.com/v1/responses"
cargo run --release
```

配置页面支持 API Key 脱敏显示、显示/隐藏、`Cmd/Ctrl+V` 粘贴和保存前校验。保存后自动切换到大模型 AI。模型从本地战术引擎筛选出的合法候选点中选择最佳落点；请求超时、服务报错或模型返回非法坐标时，会自动降级到经典搜索。静态网页版不会读取 API Key，以免密钥暴露在浏览器中。

没装 Rust 的话,macOS 下直接双击 `run_wuziqi.command`,脚本会自动通过 [rustup](https://rustup.rs) 安装工具链并编译运行。

### 网页版

将 `wuziqi.html` 和 `ai.js` 放在同一目录,直接用浏览器打开 `wuziqi.html` 即可。棋局规则与原生版一致，AI 使用无需联网的经典战术搜索。

## AI 实现

经典算法采用战术限宽搜索：先在全部候选中识别立即获胜、唯一必防和一步双杀，再从已有棋子附近 2 格内选出评分最高的 12 个落点，逐一模拟玩家最强的 10 个回应。大模型算法复用同一套战术约束生成候选集，再由语言模型结合攻防、后续威胁和中心控制做最终决策。网络请求在独立线程执行，不会阻塞窗口渲染。

## 项目结构

```
├── Cargo.toml              # Rust 项目配置 (依赖: macroquad 0.4)
├── src/
│   ├── main.rs             # 游戏状态、输入与渲染
│   ├── ai.rs               # Rust 版 AI 搜索
│   ├── config_ui.rs        # 大模型配置页面
│   └── llm_ai.rs           # 大模型 API、提示词、结果校验
├── ai.js                   # 网页版 AI 搜索
├── wuziqi.html             # 网页版游戏界面与交互
├── tests/unit_tests/mod.rs # Rust 单元测试
└── run_wuziqi.command      # macOS 一键启动脚本
```
