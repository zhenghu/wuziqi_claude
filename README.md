# wuziqi_claude — 五子棋 (Gomoku)

用 Rust + [macroquad](https://github.com/not-fl3/macroquad) 编写的五子棋小游戏,支持人机对战和双人对战。附带一个逻辑完全一致的网页移植版,无需安装任何东西即可游玩。

## 玩法

- 🖱️ 鼠标点击交叉点落子(默认你执黑,AI 执白)
- `R` 重新开始
- `U` 悔棋(人机模式回退一整轮；AI 尚未回应时只回退玩家刚落的一步)
- `M` 切换 人机 / 双人 模式

## 运行

### 原生版 (Rust)

```bash
cargo run --release
```

没装 Rust 的话,macOS 下直接双击 `run_wuziqi.command`,脚本会自动通过 [rustup](https://rustup.rs) 安装工具链并编译运行。

### 网页版

直接用浏览器打开 `wuziqi.html` 即可,规则与 AI 算法和原生版完全一致(逐行移植)。

## AI 实现

战术限宽搜索:先在全部候选中识别立即获胜、唯一必防和一步双杀，再从已有棋子附近 2 格内选出评分最高的 12 个落点，逐一模拟玩家最强的 10 个回应。叶面同时评估双方最强的两个威胁，因此能够预判跳四、双杀和对手下一手组合进攻，同时把计算量控制在主线程可流畅运行的范围内。详见 `src/main.rs` 中的 `ai_move`。

## 项目结构

```
├── Cargo.toml            # Rust 项目配置 (依赖: macroquad 0.4)
├── src/main.rs           # 游戏本体 (棋盘/AI/渲染, ~450 行)
├── wuziqi.html           # 网页移植版 (单文件, 零依赖)
└── run_wuziqi.command    # macOS 一键启动脚本
```
