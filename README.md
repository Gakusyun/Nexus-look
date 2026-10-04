# Nexus-look

一套给 Windows 桌面应用用的 GPUI-CE 组件库：**一种视觉语言、一份实现**。

**完整规范在 [`STYLE.md`](./STYLE.md)** —— 颜色、圆角、字号、间距、尺寸、动效、控件目录、
页面骨架都在那里。本文件只讲怎么把它接进项目。

---

## 它是什么

- **三个颜色**：黑、白、主题色。所有层次由黑白 α 叠出来；语义色（成功/失败/警告）
  只在"表达状态"的地方出现。
- **一个圆角**：所有矩形控件 `RADIUS = 8`；胶囊（chip/徽章/进度条/光标）是另一种形状。
- **两档高度**：`CONTROL = 32`、`CONTROL_LG = 40`。四档字号、六档间距（4 的倍数）。
- **一套控件**：按钮、图标按钮、输入框、分段器、标签页、chip、徽章、进度条、卡片、列表行、
  模态骨架、标题栏、设置表行、主题色色板、提示/空状态/渐隐。
- 输入框是**自持的**（`Entity<TextInput>`，自带缓冲区、焦点、光标、选区、IME、UTF-16 桥），
  调用方不用接管道。

## 它不是什么

- 不含任何用户可见文案（国际化是项目的责任）。
- 不含业务组件（比如"下载任务行"），只含"换个项目还成立"的东西。
- 不自带字体文件（默认走系统字体栈，可被项目覆盖）。
- 不追求"像 WinUI"：参考它，但每加一样东西都要先问"没有它，统一会破吗"。

---

## 接进项目

```toml
# Cargo.toml
[dependencies]
nexus-look = { git = "ssh://git@github.com/Gakusyun/Nexus-look.git", branch = "main" }
```

```rust
// main.rs
use nexus_look::{Look, Theme, space, control};

let look = Look::new()
    .accent(0x7c5cff)      // 项目默认主题色（用户可在设置里覆盖）
    .oled(false);          // 纯黑 OLED 背景

Application::new()
    .with_assets(nexus_look::Assets.chain(MyAssets))   // 库图标 + 项目图标
    .run(move |cx| {
        nexus_look::init(cx, look);                    // 安装 Theme 全局
        // ...
    });

let theme = Theme::of(cx);                             // 任何地方取色
```

**AssetSource 必须串联**：`gpui-ce` 的 `svg()` 只收路径、`Application::with_assets()` 只能注册
一个 `AssetSource`（`gpui-ce/src/app.rs:202`），所以库图标的解析得靠 `Assets::chain(你的)`。

主题切换（跟随系统 / 浅色 / 深色 / OLED / 改主题色）：

```rust
look.set_mode(cx, ThemeMode::System);
look.set_accent(cx, 0x12b8a6);
```

---

## GPUI-CE 的坑（写 UI 前必读）

这些是**元素样式自包含**导致的，不是审美问题：

1. **样式不从父级继承。** `compute_style_internal` 从 `Style::default()` 起步。所以
   `text_color` 设在包裹层**不会**给子 `Svg` 上色，而 `Svg` 自己没有颜色时**整个 draw 被跳过**
   —— 症状是"位置留着、什么都没有"。库里的 `icon()` 强制传色就是为了堵这个。
2. **`Window::text_style()` 在 `render` 期间返回窗口默认样式**，不是继承来的。
   量文字宽度必须自己拼 `Font`。
3. **悬停变色也不能继承**，要么挂在元素自己身上，要么用命名 group。
   `group` / `group_hover` 收的是 `Into<SharedString>`，**不是 `ElementId`**。
4. **`.on_click` 需要 `.id()`**，否则报「no method named on_click」。
5. **遮罩层**：`.occlude()` + `.absolute().top_0().left_0().size_full()`，**没有 `inset_0()`**；
   卡片上要 `.on_click(|_, _, cx| cx.stop_propagation())`，否则点按钮同时算作点遮罩。
6. **`Styled` 没有 `opacity()`**，只有 `ColorExt::opacity`（作用于颜色）。
7. **动效尊重 `reduce_motion`** 是框架给的（`animation.rs:46`），但前提是你用 `with_animation`。
8. **这个版本的 GPUI-CE 不画滚动条**，任何 `overflow_y_scroll()` 区域只能靠滚轮滚，视觉提示得
   自己给（库里用 `scroll_fade`）。
9. **inset `BoxShadow`** 是库画焦点环的方式（2px、blur 0、spread 2）—— 不占布局、不改尺寸。
10. 原生文件对话框是内置的：`cx.prompt_for_paths(..)` / `prompt_for_new_path(..)`。
    **不要**为了选文件夹引入 `rfd`。

---

## 开发

```sh
cargo fmt
cargo clippy --all-targets   # 必须零 warning
cargo test
```

许可：MIT。
