# iroh-chat-cli
---
```meta
date: 2025-06-17
authors: []
version: 0.1.0
```


#### ch01. 文档与运行
1. docs
- p2p chat, in rust, from scratch: https://www.youtube.com/watch?v=ogN_mBkWu7o
- https://www.iroh.computer/docs/examples/gossip-chat
- https://github.com/n0-computer/iroh-blobs/blob/main/examples/transfer.rs
- https://github.com/n0-computer/iroh/releases

2. chat
```
cargo run --bin iroh-chat-cli -- --name Alice open          # make Alice, the ticket will be printed
cargo run --bin iroh-chat-cli -- --name Bob join <ticket>   # make Bob
cargo run --bin iroh-chat-cli -- --name John join <ticket>  # make John
```

3. share a file
```
cargo run --bin iroh-share-file -- share <filepath>             # make share_file, share a file
cargo run --bin iroh-share-file -- receive <ticket> <filepath>  # make receive_file, receive a file
``


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

4. 退出聊天
```
::quit\n
```

5. 查看当前在线用户
```
::members\n
```

6. 直接发送一个小文件（最大支持 8MB）
```
::send_file path/to/file\n
```

7. 分享一个任意大小的文件
```
::share_file path/to/file\n
```

8. 接收一个被分享的文件
```
::receive_file blobs_ticket path/to/save\n
```

9. 本地执行一个命令
```
::command ls -alh
```
