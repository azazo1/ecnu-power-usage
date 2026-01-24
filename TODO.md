# 待办

- [ ] 移植 python 版本的内容到这里.
- [ ] 断网客户端断网提醒支持延迟提醒指定时间.
- [ ] tls 支持
- [ ] archive 防止重名, 名称使用完整的时间跨度, archive 需要保存元信息, 并且在 list-archive 的时候快速提供.
- <del>gpui</del> tauri
  - [ ] archive 和 archive 下载功能
  - [ ] 统计电量使用场景分布 (各个小时, 各个星期, 各个日期, 各个月, 等)
  - [ ] 框选显示列表项统计信息, 然后这个统计信息有个按钮为 archive.
  - [x] 显示当前记录和 archives 的剩余电量曲线.
  - [ ] 各种图标准备.

    ```
    "bundle": {
      "active": true,
      "targets": "all",
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ]
    }
    ```

  - [ ] 后台运行, 托盘图标.

- [ ] 客户端提供清除缓存的功能.
