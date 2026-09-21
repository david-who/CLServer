# CLS Server

A small Rust workspace with three IPC tools (TCP/UDP, Sharememory, mmap) built entirely on the standard library (no external dependencies):

| Crate        | Binary       | What it does                                                 |
| ------------ | ------------ | ------------------------------------------------------------ |
| `cls-server` | `cls-server` | A multi-client TCP **relay/bridge Server**. Every line a client sends is relayed to ECLS, and output of ECLS is relayed back to the client. |
| `cls-client` | `cls-client` | A TCP/UDP client, Mocking host to send command to ECLS, and recieve data from ECLS. |

| Crate     | Module    | What it does                                                 |
| --------- | --------- | ------------------------------------------------------------ |
| `EcIoShm` | `EcIoShm` | ShareMemory Module to share **IO data** between ECLS and Applications. |
| `EcPmShm` | `EcPmShm` | ShareMemory Module with *mmap* to share **Patameter data**.  |

Both tools are configurable by a positional `HOST:PORT` argument, fall back to
an environment variable, then to a sensible default:

- `cls-server` — arg / `LISTEN` / `127.0.0.1:9000`
- `cls-client` — arg / `TARGET` / `127.0.0.1:9001`

## Create

1. 创建工作区，并初始化 git，手动创建不会生成 git 相关文件

   ```bash
   cargo new my-workspace       # 创建 crate 项目
   ```

2. 将 .gitignore 内容改为

   ```ini
   .idea
   **/target
   ```

3. 删除 src 目录，或重命名为 crates 目录，删除里面的 main.rs 文件

4. 将 Cargo.toml 的内容更改为 **workspace** 内容

   ```ini
   [workspace]
    resolver = "2"
    members  = ["crates/*"]
   
   [workspace.package]
    name = "CLServer"
    version = "0.1.0"
    edition = "2024"
   
   [workspace.dependencies]
    thiserror = "1.0.61"
   # workspace members, sub-crate may dependency them.
    cli = { path = "crates/cli" }
    server = { path = "crates/server" }
    sdk = { path = "crates/sdk" }
   ```

5. 进入工作区目录，创建项目，使用工作区的版本控制文件

   ```bash
   cd CLServer
   cargo new packages/cls-server --vcs none    # 不生成版本控制文件
   cargo new packages/cls-client --vcs none    # 不生成版本控制文件
   ```

目录结构如下：

```cmd
CLServer [workspace]
├── README
├── LICENSE
├── cargo.toml
├── cargo.lock
├── packages [package]
│   └── ecls [project]
│   │   ├── cargo.toml
│   │   ├── cargo.lock
│   │   ├── src           # 源代码, 主要程序
│   │   │   ├── main.rs             [crate] # 可执行程序入口, 包含 main 函数
│   │   │   ├── lib.rs              [crate] # 库根模块, 供其他模块和程序引用, 可选
│   │   │   └── modules
│   │   │   │   ├── mod.rs # 缺省模块, 可选; 名称为目录名称
│   │   │   │   ├── server.rs
│   │   │   │   ├── ...
│   │   │   │   └── client.rs
│   │   │   └── bin       # 额外的可执行程序，例如测试程序
│   │   │       ├── other binaray   [crate]
│   │   │       ├── ...             [crate]
│   │   │       └── multi-file-executable
│   │   │           ├── main.rs     [crate]
│   │   │           └── some_module.rs
│   │   ├── tests         # 功能测试, 单元测试
│   │   │   ├── ...
│   │   │   ├── test_n.rs           [crate]
│   │   │   └── multi-file-test
│   │   │       ├── main.rs         [crate]
│   │   │       └── test_module.rs
│   │   ├── examples/     # 示例程序
│   │   ├── docs/         # 文档
│   │   └── benches       # 性能测试, 集成测试
│   │       ├── ...
│   │       ├── test_n.rs           [crate]
│   │       └── multi-file-bench
│   │           ├── main.rs         [crate]
│   │           └── bench_module.rs
│   └── demo [project]
│       ├── cargo.toml
│       ├── cargo.lock
│       └── src
│           ├── main.rs              [crate]
│           └── bin
│              ├── tcp-server-thread [crate]
│              ├── tcp-server-task   [crate]
│              ├── tcp-client        [crate]
│              ├── udp-server        [crate]
│              ├── tcp-client        [crate]
│              └── ...               [crate]
├── target
```

## Build

```bash
cargo build          # debug
cargo build --release
cargo clippy         # lint
cargo run -p tcp-server
```

## Usage

### CLS server

```bash
# Terminal 1 — start the server (logs to stderr)
cargo run -p cls-server

# Terminal 2 & 3 — connect two clients
nc 127.0.0.1 9000
nc 127.0.0.1 9000
```

Type a line in one client; it is relayed (prefixed with `[#id]`) to **both** clients. Messages from either peer appear in both, so it can be used as a quick chat/relay channel.

### CLS client

```bash
# One-shot: send a line to a UDP endpoint listening on 127.0.0.1:9001
echo "ping" | cargo run -p cls-client

# Interactive: keep stdin open to keep sending and see replies
cargo run -p cls-client 10.0.0.5:5000
```

## Notes

- Framing: cls-server relays newline-framed lines; UDP treats each sent line as a single datagram.
- The UDP reply thread prints incoming datagrams on stdout as they arrive. With piped input (e.g. `echo … | cls-client`) the process exits when stdin closes, so keep stdin open interactively if you want to observe replies.