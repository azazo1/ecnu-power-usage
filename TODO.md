# 待办

- [x] 移植 python 版本的内容到这里.
- [ ] 断网客户端断网提醒支持延迟提醒指定时间.
- [ ] tls 支持
- [x] archive 防止重名, 名称使用完整的时间跨度, archive 需要保存元信息, 并且在 list-archive 的时候快速提供.
  - [x] archive 元信息包括起止时间, 名称等, 在 /create-archive 中以参数的形式提供其名称, 客户端交互式确定(在 stats card 的 archive 按钮中).
- <del>gpui</del> tauri
  - [x] 创建 archive 交互 dialog.
  - [ ] 添加主屏幕, 主要显示当前的电量, 显示当前的状态 (未登录/缺少宿舍信息), 看看能不能添加一点统计信息.
  - [x] archive 和 archive 下载功能按钮
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
  - [ ] 配置页面: 配置tls证书, 服务端地址.

- [ ] 客户端提供清除缓存的功能.
