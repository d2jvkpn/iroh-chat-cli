# iroh-chat-cli
---
```meta
date: 2025-06-17
authors: []
version: 0.1.0
```


#### ch01. docs and run
1. docs
- p2p chat, in rust, from scratch: https://www.youtube.com/watch?v=ogN_mBkWu7o
- https://www.iroh.computer/docs/examples/gossip-chat
- https://github.com/n0-computer/iroh-blobs/blob/main/examples/transfer.rs

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
:me\n
```

4. quit
```
:quit\n
```

5. show online accounts
```
:online\n
```

6. send a small file directly (max size=8M)
```
:send path/to/file\n
```

7. share a file (any size)
```
:share path/to/file\n
```

8. receive a shared file
```
:receive blobs_ticket path/to/save\n
```
