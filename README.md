# jokerben-desktop-pet

A lightweight desktop pet in the shape of Clawd, the pixel mascot from Claude Code. It sits in a corner of your screen, reacts while you type and click, and counts your keyboard and mouse activity.

一个轻量级桌宠，形象是 Claude Code 的像素吉祥物 Clawd。它待在屏幕角落，会跟着你打字和点击做出反应，并统计你的键盘和鼠标操作次数。

> **Unofficial fan project.** This project is not affiliated with, endorsed by, or sponsored by Anthropic. Claude, Claude Code and Clawd are trademarks or characters of Anthropic.
>
> **非官方同人作品**，与 Anthropic 无关，也未获其认可或赞助。Claude、Claude Code 和 Clawd 是 Anthropic 的商标或角色。

## Features / 功能

- Counts key presses (per key), left/right/middle clicks, scroll gestures and mouse travel (in metres), stored per day
- A number above the pet's head: today's count or live per-minute speed, from the keyboard, the mouse, or both combined
- Hover the pet for today's stats; right-click (or use the tray icon) for the menu
- Clawd types along with you, reacts to clicks and scrolling, follows your cursor with its eyes, falls asleep when you're away, and likes being poked
- 中英双语，跟随系统语言

## Privacy / 隐私

Only counts are stored: how many times each key was pressed, clicks, scroll gestures, and how far the mouse moved. The app never records what you type or the order of your key presses. All data stays on your machine.

只保存计数：每个键按了几次、点击、滚动手势和鼠标移动距离。不记录输入内容，也不记录按键顺序。所有数据只存在本机。

## Install / 安装

Download the installer for your platform from [Releases](https://github.com/sparkjokerben/jokerben-desktop-pet/releases). The builds are not signed by Apple or Microsoft, so your system will warn you the first time.

安装包没有经过 Apple 或微软的签名，第一次打开时系统会拦截，按下面的步骤放行即可。

### macOS

1. Unzip and move `jokerben-desktop-pet.app` to Applications. The `aarch64` build is for Apple Silicon, `x64` for Intel Macs.
2. Open it once. When macOS says it can't verify the developer, go to **System Settings → Privacy & Security** and click **Open Anyway**. (On macOS 15 and later, right-click → Open no longer works.) Alternatively run:
   ```sh
   xattr -dr com.apple.quarantine /Applications/jokerben-desktop-pet.app
   ```
3. Allow **Input Monitoring** when asked (System Settings → Privacy & Security → Input Monitoring). Without it the pet shows a confused face and counts nothing.

- 解压后把 app 拖进「应用程序」。Apple 芯片选 `aarch64`，Intel 选 `x64`。
- 首次打开被拦截时，到「系统设置 → 隐私与安全性」点「仍要打开」（macOS 15 起右键「打开」已经不管用），或者在终端运行上面的 `xattr` 命令。
- 按提示授予「输入监控」权限。没有这个权限，桌宠会一脸问号，也统计不到任何操作。
- If an update ever stops counting, remove the old entry from Input Monitoring and add the app again. 如果更新后不计数了，在「输入监控」里删掉旧条目再重新添加。

### Windows

Run the `.exe` installer. If SmartScreen says "Windows protected your PC", click **More info → Run anyway**. Some antivirus tools flag any program that listens to the keyboard; this app only counts key presses.

运行 `.exe` 安装程序。SmartScreen 提示「Windows 已保护你的电脑」时，点「更多信息 → 仍要运行」。部分杀毒软件会把监听键盘的程序当成可疑程序；本程序只做计数。

### Linux (X11)

Use the AppImage (auto-updates) or the `.deb`. Only X11 sessions are supported: Wayland does not let apps listen to global input. A compositor is needed for the transparent window and an AppIndicator-compatible tray for the menu.

使用 AppImage（支持自动更新）或 `.deb`。只支持 X11；Wayland 不允许程序监听全局输入。

## Known limitations / 已知限制

- macOS: keys typed while **Secure Keyboard Entry** is on (e.g. enabled in Terminal or iTerm, or in password fields) are hidden from every app, including this one. The pet puts on a blindfold while that happens.
- Windows: input sent to windows running as administrator is not seen unless the pet also runs as administrator. The pet stays on the virtual desktop it was started on.
- Linux: X11 only.
- The pet steps aside while a fullscreen app (video, game, presentation) is in front: on macOS because it never joins fullscreen Spaces, on Windows and X11 by hiding until the app leaves fullscreen.

## Where the data lives / 数据位置

- macOS: `~/Library/Application Support/io.github.sparkjokerben.jokerben-desktop-pet/`
- Windows: `%APPDATA%\io.github.sparkjokerben.jokerben-desktop-pet\`
- Linux: `~/.local/share/io.github.sparkjokerben.jokerben-desktop-pet/`

`stats.sqlite` holds the counts; `settings.json` (in the config directory) holds your settings.

## Development / 开发

Requires Node 24+ and Rust (stable).

```sh
npm install
npm run tauri dev     # run the app
npm test              # frontend tests
npm run check         # type check
cd src-tauri && cargo test
npm run sprites       # render every animation frame to sprites.png
npm run icons         # regenerate app and tray icons from the sprite
```

With `npm run dev` running, `http://localhost:1420/preview.html?view=stats` (or `settings`, `onboarding`, `pet`) renders a window in a normal browser with fake data, which is handy for layout work.

On macOS, `tauri dev` runs the app as a child of your terminal, so grant Input Monitoring to the terminal app. To test the real permission flow, build a bundle with `npm run tauri build -- --debug --bundles app`. Every rebuild changes an ad-hoc signature, so clear the stale grant with:

```sh
tccutil reset ListenEvent io.github.sparkjokerben.jokerben-desktop-pet
```

## Releasing / 发布

1. Bump `version` in `package.json` (the app reads it from there) and commit.
2. Tag and push: `git tag v0.1.0 && git push origin v0.1.0`.
3. The Release workflow builds macOS (arm64 and x64), Windows and Linux into a draft release with the updater manifest (`latest.json`).
4. Check the draft and publish it. Installed apps pick the update up within six hours and install it on restart.

The updater signing key lives in the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets. Keep a backup: without it, installed copies can never be updated again.

## License

[GPL-3.0](LICENSE)
