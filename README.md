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
- https://www.iroh.computer

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

4. show online accounts
```
:online\r
```

5. send a file (max size=32M)
:send path/to/file\r

6. quit
```
:quit\r
```
