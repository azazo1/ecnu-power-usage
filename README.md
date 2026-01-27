# ECNU 电费查询

客户端需要提前安装 chrome 浏览器.

## 服务端

服务端负责持续监视并记录电量情况, 并向客户端提供记录信息.

### 服务端安装

```shell
just install
```

`ecnu_power_usage_server` 和 `ecnu_power_usage_cert_tool` 会被安装到 `$CARGO_HOME/bin` 中.

### 服务端配置

第一次使用需要编写配置文件.

配置文件的目录路径随系统而各异, 下面是一些示例:

| 平台    | 路径格式                                                                | 示例                                                      |
|---------|-------------------------------------------------------------------------|-----------------------------------------------------------|
| Linux   | `$XDG_CONFIG_HOME`/ecnu-power-usage or `$HOME`/.config/ecnu-power-usage | /home/alice/.config/ecnu-power-usage                      |
| macOS   | `$HOME`/Library/Application Support/ecnu-power-usage                    | /Users/Alice/Library/Application Support/ecnu-power-usage |
| Windows | `{FOLDERID_RoamingAppData}`\ecnu-power-usage                            | C:\Users\Alice\AppData\Roaming\ecnu-power-usage           |

在这些目录下的 `server.toml` 文件即为配置文件, 配置文件编写示例:

```toml
# 服务器绑定地址
bind = "0.0.0.0:20531"
# mTLS 配置(可选)
[tls]
server_cert = "/path/to/server.crt" # 服务端证书
server_key = "/path/to/server.key" # 服务端密钥
root_ca = "/path/to/root_ca.crt" # 根证书
```

- `tls` 如果填写, 那么自动启用 mTLS, 验证客户端访问, 客户端需要使用同样的自签名证书签发的客户端证书才能访问. 证书的生成参见 [证书生成].

### 服务端运行

```shell
ecnu_power_usage_server
```

## 证书生成

使用 `ecnu_power_usage_cert_tool` 工具可以快速生成可用的自签名证书, CS 证书和密钥.

```shell
# 生成有效期 10 年的自签名根证书 -> root_ca.crt 和 root_ca.key
ecnu_power_usage_cert_tool self-signed --age 10y --out root_ca

# 生成有效期 10 年的服务器证书 -> server.crt 和 server.key, 服务器的域名为 example.com, 请根据实际情况进行修改.
ecnu_power_usage_cert_tool sign --root root_ca --age 10y --sans example.com --out server
# 如果没有域名, 可以使用 --ip-sans, 使用下面这条命令替代上一条, IP 地址 127.0.0.1 需要根据实际情况替换成服务器 IP.
# ecnu_power_usage_cert_tool sign --root root_ca --age 10y --ip-sans 127.0.0.1 --out server

# 生成有效期 10 年的客户端证书 -> client.crt 和 client.key
ecnu_power_usage_cert_tool sign --root root_ca --age 10y --client --out client
```

- 证书生成方法不唯一, 也可以使用 `openssl` 自行生成.
- 注意生成服务端证书的时候 SAN / IP-SAN 至少需要指定一个, 不然可能无法成功建立 mTLS 链接.

当前目录下生成的 `server.key`, `server.crt`, `client.key`, `client.crt` 以及 `root_ca.crt` 就是配置中对应的证书和密钥文件.

服务端建议将证书保存到配置目录下, unix 系统可以设置当前用户只读权限.

客户端证书可以打开 gui 配置一键导入.
