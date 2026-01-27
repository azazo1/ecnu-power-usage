use clap::{Parser, Subcommand, ValueEnum};
use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose, PKCS_ECDSA_P256_SHA256, PKCS_ED25519, PKCS_RSA_SHA256, SanType,
};
use std::{fs, io::Write, path::Path};
use time::{Duration, OffsetDateTime};

#[derive(Parser, Debug)]
#[command(author, version, about = "证书生成与签发工具", long_about = None)]
pub struct AppArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 生成自签名的根证书 (CA)
    ///
    /// 该证书将作为信任根，用于签发后续的服务器和客户端证书。
    SelfSigned {
        /// 证书有效期 (例如: 10y, 365d, 24h)
        #[arg(short, long, default_value = "10y")]
        age: String,

        /// 输出文件的前缀 (会生成 .crt 和 .key)
        #[arg(short, long, default_value = "root_ca")]
        out: String,

        /// 证书持有者名称 (Common Name)
        #[arg(long, default_value = "ECNU Power Usage Root CA")]
        cn: String,

        /// 密钥算法
        #[arg(long, value_enum, default_value_t = KeyAlgorithm::Ed25519)]
        algo: KeyAlgorithm,
    },

    /// 使用指定的根证书签发子证书 (服务器或客户端)
    ///
    /// 签发时会根据 --is-client 自动设置证书的 Extended Key Usage。
    Sign {
        /// 根证书的前缀 (需存在 .crt 和 .key 文件)
        #[arg(short, long)]
        root: String,

        /// 证书有效期 (例如: 1y, 10y)
        #[arg(short, long, default_value = "1y")]
        age: String,

        /// 输出文件的前缀
        #[arg(short, long)]
        out: String,

        /// 证书持有者名称 (对于服务器证书，通常设为 localhost 或域名)
        #[arg(long, default_value = "localhost")]
        cn: String,

        /// 主体别名 (SAN)，用于指定多个域名.
        #[arg(long, value_delimiter = ',')]
        sans: Vec<String>,

        /// SAN 的 IP 附加, 用于没有域名只有 IP 地址时使用.
        #[arg(long, value_delimiter = ',')]
        ip_sans: Vec<String>,

        /// 是否为客户端证书 (默认为 false，即服务器证书)
        #[arg(long)]
        client: bool,

        /// 密钥算法
        #[arg(short, long, value_enum, default_value_t = KeyAlgorithm::Ed25519)]
        algo: KeyAlgorithm,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum KeyAlgorithm {
    Ed25519,
    Rsa,
    EcdsaP256,
}

impl KeyAlgorithm {
    fn generate_pair(&self) -> KeyPair {
        match self {
            Self::Ed25519 => KeyPair::generate_for(&PKCS_ED25519).unwrap(),
            Self::Rsa => KeyPair::generate_for(&PKCS_RSA_SHA256).unwrap(),
            Self::EcdsaP256 => KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = AppArgs::parse();

    match args.command {
        Commands::SelfSigned { age, out, cn, algo } => {
            let duration = parse_age(&age)?;
            create_ca_cert(&cn, &out, duration, algo)?;
        }
        Commands::Sign {
            root,
            age,
            out,
            cn,
            sans,
            ip_sans,
            client: is_client,
            algo,
        } => {
            let duration = parse_age(&age)?;
            let issuer = load_ca_cert_issuer(&root)?;
            create_leaf_cert(&issuer, &cn, &out, duration, sans, ip_sans, is_client, algo)?;
        }
    }

    Ok(())
}

/// 解析简单的年龄字符串，如 "10y", "365d"
fn parse_age(age: &str) -> anyhow::Result<Duration> {
    let unit = age.chars().last().unwrap_or('d');
    let val: i64 = age[..age.len() - 1].parse()?;
    match unit {
        'y' => Ok(Duration::days(val * 365)),
        'd' => Ok(Duration::days(val)),
        'h' => Ok(Duration::hours(val)),
        _ => Err(anyhow::anyhow!("未知的时间单位: {}", unit)),
    }
}

fn create_ca_cert(
    cn: &str,
    out_prefix: &str,
    duration: Duration,
    algo: KeyAlgorithm,
) -> anyhow::Result<()> {
    let mut params = CertificateParams::default();
    params.distinguished_name.push(DnType::CommonName, cn);
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];

    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + duration;

    let key_pair = algo.generate_pair();
    let cert = params.self_signed(&key_pair)?;

    write_files(out_prefix, cert.pem(), key_pair.serialize_pem())?;
    println!(
        "根证书生成: {:?}",
        Path::new(&out_prefix).with_added_extension("crt")
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_leaf_cert(
    issuer: &Issuer<'_, KeyPair>,
    cn: &str,
    out_prefix: &str,
    duration: Duration,
    sans: Vec<String>,
    ip_sans: Vec<String>,
    is_client: bool,
    algo: KeyAlgorithm,
) -> anyhow::Result<()> {
    let mut params = CertificateParams::default();
    params.distinguished_name.push(DnType::CommonName, cn);

    // 设置 SANs
    params.subject_alt_names = sans
        .into_iter()
        .map(|s| Ok(SanType::DnsName(s.parse()?)))
        .collect::<anyhow::Result<_>>()?;
    params.subject_alt_names.extend(
        ip_sans
            .into_iter()
            .map(|s| Ok(SanType::IpAddress(s.parse()?)))
            .collect::<anyhow::Result<Vec<SanType>>>()?,
    );

    params.is_ca = IsCa::ExplicitNoCa;
    params.not_before = OffsetDateTime::now_utc();
    params.not_after = params.not_before + duration;

    // 根据类型设置 Key Usage
    if is_client {
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    } else {
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    }
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];

    let key_pair = algo.generate_pair();
    let cert = params.signed_by(&key_pair, issuer)?;

    write_files(out_prefix, cert.pem(), key_pair.serialize_pem())?;
    println!(
        "{} 证书密钥对生成: {:?} / {:?}",
        if is_client { "客户端" } else { "服务端" },
        Path::new(&out_prefix).with_added_extension("crt"),
        Path::new(&out_prefix).with_added_extension("key")
    );
    Ok(())
}

fn load_ca_cert_issuer(prefix: impl AsRef<Path>) -> anyhow::Result<Issuer<'static, KeyPair>> {
    let cert_pem = fs::read_to_string(prefix.as_ref().with_added_extension("crt"))?;
    let key_pem = fs::read_to_string(prefix.as_ref().with_added_extension("key"))?;
    let key_pair = KeyPair::from_pem(&key_pem)?;

    Ok(Issuer::from_ca_cert_pem(&cert_pem, key_pair)?)
}

// 在 write_files 中调用
fn write_files(prefix: impl AsRef<Path>, cert_pem: String, key_pem: String) -> anyhow::Result<()> {
    let cert_path = prefix.as_ref().with_added_extension("crt");
    let key_path = prefix.as_ref().with_added_extension("key");

    let mut options = fs::File::options();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::prelude::OpenOptionsExt;

        options.mode(0o600); // 只有所有者可读写
    }
    options.open(cert_path)?.write_all(cert_pem.as_ref())?;
    options.open(key_path)?.write_all(key_pem.as_ref())?;
    Ok(())
}
