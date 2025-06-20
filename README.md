# iroh-chat-cli
---
```meta
date: 2025-06-19
authors: []
version: 0.1.0
```


#### ch01. docs and run
1. docs
- p2p chat, in rust, from scratch
  https://www.youtube.com/watch?v=ogN_mBkWu7o
- https://www.iroh.computer/docs/examples/gossip-chat
- https://github.com/n0-computer/iroh-blobs/blob/main/examples/transfer.rs

2. run
```
cargo run -- --name Alice open # the ticke will be printed
cargo run -- --name Bob join <ticket>
cargo run -- --name Jone join <ticket>
```

#### ch02. chat
1. send an oneline message
```
Hello\r
```

2. send a multiline message (keep a space at the end of line)
```
Hello \r
I'm Alice. \r
How are you today?\r
```

3. show me
```
:me\r
```

4. quit
```
:quit\r
```

5. show online accounts
```
:online\r
```

6. send a small file directly (max size=8M)
```
:send path/to/file\r
```

7. share a file (any size)
```
:share path/to/file\r
```

8. receive a shared file
```
:receive blobs_ticket path/to/save\r
```
