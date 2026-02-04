# ECNU 电费查询

客户端需要提前安装 chrome 浏览器.

## 服务端

服务端负责持续监视并记录电量情况, 并向客户端提供记录信息.

### 服务端安装

可以在 [releases](https://github.com/azazo1/ecnu-power-usage/releases) 界面下载预编译二进制文件 `epu-server`, `epu-cert`.

也可以自行编译:

```shell
cargo install --git https://github.com/azazo1/ecnu-power-usage.git ecnu-power-usage
```

`epu-server` 和 `epu-cert` 会被安装到 `$CARGO_HOME/bin` 中.

### 服务端配置

第一次使用需要编写配置文件.

配置文件的目录路径随系统而各异, 下面是一些示例:

| 平台    | 路径格式                                                                | 示例                                                      |
|---------|-------------------------------------------------------------------------|-----------------------------------------------------------|
| Linux   | `$XDG_CONFIG_HOME`/ecnu-power-usage or `$HOME`/.config/ecnu-power-usage | /home/alice/.config/ecnu-power-usage                      |
| macOS   | `$HOME`/Library/Application Support/ecnu-power-usage                    | /Users/Alice/Library/Application Support/ecnu-power-usage |
| Windows | `{FOLDERID_RoamingAppData}`\ecnu-power-usage                            | C:\Users\Alice\AppData\Roaming\ecnu-power-usage           |

- 注: 如果使用 dev profile 编译运行, 则配置文件夹的名称会有后缀 `-debug`.

在这些目录下的 `server.toml` 文件即为配置文件, 配置文件编写示例:

```toml
# 服务器绑定地址
bind = "0.0.0.0:20531"
# mTLS 配置(可选)
[tls]
server_cert = "/path/to/server.crt" # 服务端证书
server_key = "/path/to/server.key" # 服务端密钥
root_ca = "/path/to/root-ca.crt" # 根证书
```

- `tls` 如果填写, 那么自动启用 mTLS, 验证客户端访问, 客户端需要使用同样的自签名证书签发的客户端证书才能访问. 证书的生成参见 [证书生成](#证书生成).
- 启用 tls 能够在公网安全地传输数据, 防止信息泄露.

### 服务端运行

```shell
epu-server
```

## 客户端

客户端使用 tauri gui 框架构建, 在 `tauri/` 文件夹中.

客户端可以直接下载 [releases](https://github.com/azazo1/ecnu-power-usage/releases) 中的可执行文件直接安装运行.
也可以克隆此仓库进入 `tauri/` 文件夹使用命令:

```shell
bun run tauri build
```

自行编译打包.

客户端第一次启动正常情况下因为没有进行配置, 无法连接到服务端, 点击弹窗右上角的设置按钮设置服务端的地址和 TLS 证书及密钥 (见 [证书生成](#证书生成)).

![打开设置](assets/open-settings.png)

## 证书生成

使用 `epu-cert` 工具可以快速生成可用的自签名证书, CS 证书和密钥.

```shell
# 生成有效期 10 年的自签名根证书 -> root-ca.crt 和 root-ca.key
epu-cert self-signed --age 10y --out root-ca

# 生成有效期 10 年的服务器证书 -> server.crt 和 server.key, 服务器的域名为 localhost, 请根据实际情况进行修改.
epu-cert sign --root root-ca --age 10y --sans localhost --out server
# 如果没有域名, 可以使用 --ip-sans, 使用下面这条命令替代上一条, IP 地址 127.0.0.1 需要根据实际情况替换成服务器 IP.
# epu-cert sign --root root-ca --age 10y --ip-sans 127.0.0.1 --out server

# 生成有效期 10 年的客户端证书 -> client.crt 和 client.key
epu-cert sign --root root-ca --age 10y --client --out client
```

- 证书生成方法不唯一, 也可以使用 `openssl` 自行生成.
- 注意生成服务端证书的时候 SAN / IP-SAN 至少需要指定一个, 不然可能无法成功建立 mTLS 链接.
- 可以使用 `wget --certificate=client.crt --private-key=client.key --ca-certificate=root-ca.crt https://localhost:20531/get-records` 命令查看证书是否正确生成.

当前目录下生成的 `server.key`, `server.crt`, `client.key`, `client.crt` 以及 `root-ca.crt` 就是配置中对应的证书和密钥文件.

服务端建议将证书保存到配置目录下, unix 系统可以设置当前用户只读权限.

客户端证书可以打开 gui 配置一键导入.
