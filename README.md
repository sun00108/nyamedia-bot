Nyamedia Bot

cargo install diesel_cli --no-default-features --features "sqlite-bundled"

### 小工具
##### 1. Webhook
- 用于接收 Emby Webhook 事件
- 推送至指定的 Telegram 群聊
##### 2. FolderGuard
- 用于监控自动下载的文件夹
- folderguard /path/to/folder
- 确保目录下有 config.json 和 configpending.json
