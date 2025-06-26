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

2. chat
```
make Alice  # cargo run --bin iroh-chat-cli -- --name Alice open
make Bob    # cargo run --bin iroh-chat-cli -- --name Bob join [ticket_str | ticket_path]
make John   # cargo run --bin iroh-chat-cli -- --name John join [ticket_str | ticket_path]
```

3. share a file
```
make share_file    # cargo run --bin iroh-share-file -- share [filepath] [option(ticket_path)]
make receive_file  # cargo run --bin iroh-share-file -- receive [ticket_str | ticket_path] [filepath]
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

4. quit
```
::quit\n
```

5. show online members
```
::members\n
```

6. send a small file directly (max size=8M)
```
::send_file [path/to/file]\n
```

7. share a file (any size)
```
::share_file [path/to/file]\n
```

8. receive a shared file
```
::receive_file [blobs_ticket] [path/to/save]\n
```

9. run a local command
```
::run ls -alh
```
