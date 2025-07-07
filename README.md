# iroh-chat-cli
---
```meta
date: 2025-06-17
authors: []
version: 0.1.1
```


#### ch01. docs and run
1. docs
- p2p chat, in rust, from scratch: https://www.youtube.com/watch?v=ogN_mBkWu7o
- https://www.iroh.computer/docs/examples/gossip-chat
- https://github.com/n0-computer/iroh-blobs/blob/main/examples/transfer.rs
- https://github.com/n0-computer/iroh/releases

2. create a chat room
```
make Alice  # cargo run -- --name Alice open -w configs/Alice.topic.ticket
make Bob    # cargo run -- --name Bob join configs/Alice.topic.ticket -w configs/Bob.topic.ticket
make John   # cargo run -- --name John join configs/Bob.topic.ticket -w configs/John.topic.ticket
```

#### ch02. chatting
1. send an oneline message
```
Hello\n
```

2. send a multiline message(keep a space at the end of line)
```
Hello \n
I'm Alice. \n
How are you today?\n
```

3. show me
```
::me\n
```

4. help
```
::help\n
```

5. quit
```
::quit\n
```

6. show online members
```
::members\n
```

7. send a small file directly (max size=8M)
```
::send_file [path/to/file]\n
```

8. share a file (any size)
```
::share_file [path/to/file]\n
```

9. receive a shared file
```
::receive_file [blobs_ticket] [path/to/save]\n
```

10. run a local command
```
::run ls -alh
```
