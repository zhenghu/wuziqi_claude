# wuziqi_claude — 五子棋 (Gomoku)

用 Rust + [macroquad](https://github.com/not-fl3/macroquad) 编写的五子棋小游戏,支持人机对战和双人对战。附带一个逻辑完全一致的网页移植版,无需安装任何东西即可游玩。

## 玩法

- 🖱️ 鼠标点击交叉点落子(默认你执黑,AI 执白)
- `R` 重新开始
- `U` 悔棋(人机模式自动回退两步)
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

单层贪心评估:对棋盘上已有棋子附近 2 格内的每个空点,按四个方向统计「连子数 + 开放端数」映射为棋型分值(活四 100 万 → 眠一 10 分),以 `进攻 ×10 + 防守 ×9 + 中心偏好` 选择最高分落点。详见 `src/main.rs` 中的 `ai_move`。

## 项目结构

```
├── Cargo.toml            # Rust 项目配置 (依赖: macroquad 0.4)
├── src/main.rs           # 游戏本体 (棋盘/AI/渲染, ~450 行)
├── wuziqi.html           # 网页移植版 (单文件, 零依赖)
└── run_wuziqi.command    # macOS 一键启动脚本
```
