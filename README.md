# jokerben-desktop-pet

A lightweight desktop pet in the shape of Clawd, the pixel mascot from Claude Code. It sits in a corner of your screen, reacts while you type and click, and counts your keyboard and mouse activity.

一个轻量级桌宠，形象是 Claude Code 的像素吉祥物 Clawd。它待在屏幕角落，会跟着你打字和点击做出反应，并统计你的键盘和鼠标操作次数。

> **Unofficial fan project.** This project is not affiliated with, endorsed by, or sponsored by Anthropic. Claude, Claude Code and Clawd are trademarks or characters of Anthropic.
>
> **非官方同人作品**，与 Anthropic 无关，也未获其认可或赞助。Claude、Claude Code 和 Clawd 是 Anthropic 的商标或角色。

## Privacy

Only counts are stored: how many times each key was pressed, clicks, scroll gestures, and how far the mouse moved. The app never records what you type or the order of your key presses. All data stays on your machine.

只保存计数：每个键按了几次、点击、滚动手势和鼠标移动距离。不记录输入内容，也不记录按键顺序。所有数据只存在本机。

## Status

Work in progress. See the plan for the roadmap: macOS first, then Windows and Linux (X11).

## Development

Requires Node 24+ and Rust (stable).

```sh
npm install
npm run tauri dev
```

## License

[GPL-3.0](LICENSE)
