# iroh-chat-cli
---
```meta
date: 2025-06-17
authors: []
version: 0.1.0
```


#### ch01. 文档与运行
1. 文档列表
- p2p chat, in rust, from scratch: https://www.youtube.com/watch?v=ogN_mBkWu7o
- https://www.iroh.computer/docs/examples/gossip-chat
- https://github.com/n0-computer/iroh-blobs/blob/main/examples/transfer.rs
- https://github.com/n0-computer/iroh/releases

2. 发起一个聊天
```
make Alice  # cargo run -- --name Alice open -w configs/Alice.topic.ticket
make Bob    # cargo run -- --name Bob join configs/Alice.topic.ticket -w configs/Bob.topic.ticket
make John   # cargo run -- --name John join configs/Bob.topic.ticket -w configs/John.topic.ticket
```

#### ch02. 聊天
1. 发送单行消息
```text
Hello\r
```

2. 发送多行消息（每行末尾保留一个空格）
```
Hello \n
I'm Alice. \n
How are you today?\n
```

3. 查看当前用户信息
```
::me\n
```

4. 帮助
```
::help\n
```

5. 退出聊天
```
::quit\n
```

6. 查看当前在线用户
```
::members\n
```

7. 直接发送一个小文件（最大支持 8MB）
```
::send_file [path/to/file]\n
```

8. 分享一个任意大小的文件
```
::share_file [path/to/file]\n
```

9. 接收一个被分享的文件
```
::receive_file [blobs_ticket] [path/to/save]\n
```

10. 本地执行一个命令
```
::run ls -alh
```
