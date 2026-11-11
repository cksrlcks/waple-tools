//! 로컬 index(debug 전용)로 설치 → 실행 → 업데이트 → 제거. dev.copy를 DLL로 빌드해 쓴다.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use base64::Engine as _;
use waple_tools_core::{JobContext, ToolOutput};

use super::index::{Index, IndexEntry, Installed, InstalledTool};
use super::store::{SIG, parse_key, sha256_hex, version_dir, write_installed, write_version};
use super::*;

const ID: &str = "dev.copy";

fn b64(s: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(s)
}

struct Key {
    pk: PublicKey,
    sk: minisign::SecretKey,
}

fn key() -> Key {
    let kp = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let pk = parse_key(&b64(&kp.pk.to_box().unwrap().into_string())).unwrap();
    Key { pk, sk: kp.sk }
}

/// `tauri signer sign`과 같은 형식 (서명 상자를 base64로)
fn sign(k: &Key, bytes: &[u8]) -> String {
    b64(&minisign::sign(None, &k.sk, Cursor::new(bytes), None, None)
        .unwrap()
        .into_string())
}

fn dll() -> &'static [u8] {
    static DLL: OnceLock<Vec<u8>> = OnceLock::new();
    DLL.get_or_init(|| {
        let path = waple_tools_addon_abi::testkit::build_addon("waple-tools-tool-dev-copy");
        std::fs::read(path).unwrap()
    })
}

/// `<dir>/index.json` + `<dir>/<version>.dll`
fn publish(dir: &Path, k: &Key, versions: &[&str]) -> PathBuf {
    let bytes = dll();
    let tools = versions
        .iter()
        .map(|v| {
            std::fs::write(dir.join(format!("{v}.dll")), bytes).unwrap();
            IndexEntry {
                id: ID.into(),
                version: v.to_string(),
                abi: ABI_VERSION,
                url: format!("{v}.dll"),
                size: bytes.len() as u64,
                sha256: sha256_hex(bytes),
                signature: sign(k, bytes),
            }
        })
        .collect();
    let path = dir.join("index.json");
    std::fs::write(&path, serde_json::to_vec(&Index { tools }).unwrap()).unwrap();
    path
}

fn install(a: &Addons) -> Result<(), String> {
    tauri::async_runtime::block_on(a.install(ID, |_| {}))
}

fn versions_on_disk(root: &Path) -> Vec<String> {
    let mut v: Vec<_> = std::fs::read_dir(root.join(ID))
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

#[test]
fn install_run_update_uninstall() {
    let tmp = tempfile::tempdir().unwrap();
    let (root, srv) = (tmp.path().join("addons"), tmp.path().join("srv"));
    std::fs::create_dir(&srv).unwrap();
    let k = key();
    let index = publish(&srv, &k, &["1.0.0"]);

    let a = Addons::open(root.clone(), vec![k.pk.clone()], Some(index.clone()));
    assert_eq!(a.info(ID).status, AddonStatus::NotInstalled);
    let mut seen = vec![];
    tauri::async_runtime::block_on(a.install(ID, |p| seen.push(p))).unwrap();
    assert_eq!(seen.last().unwrap().received, dll().len() as u64);
    assert_eq!(a.info(ID).status, AddonStatus::Installed);
    assert_eq!(a.info(ID).version.as_deref(), Some("1.0.0"));

    // 실행
    let src = tmp.path().join("a.txt");
    std::fs::write(&src, b"abc").unwrap();
    let out = tmp.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let r = a
        .get(ID)
        .unwrap()
        .run(&ctx, &[src], &out, &serde_json::json!({}));
    assert!(matches!(r, Ok(ToolOutput::Files(_))));
    assert_eq!(std::fs::read(out.join("a.txt")).unwrap(), b"abc");

    // 새 버전 → 업데이트 표시 → 업데이트. 열린 예전 판 폴더는 남는다
    publish(&srv, &k, &["1.0.0", "1.1.0"]);
    tauri::async_runtime::block_on(a.refresh()).unwrap();
    assert_eq!(a.info(ID).status, AddonStatus::UpdateAvailable);
    assert_eq!(a.info(ID).latest.as_deref(), Some("1.1.0"));
    install(&a).unwrap();
    assert_eq!(a.info(ID).status, AddonStatus::Installed);
    assert_eq!(a.info(ID).version.as_deref(), Some("1.1.0"));
    assert_eq!(versions_on_disk(&root), ["1.0.0", "1.1.0"]);

    // 다시 시작: 예전 판 폴더를 지우고 새 판을 연다
    drop(a);
    let a = Addons::open(root.clone(), vec![k.pk.clone()], Some(index.clone()));
    assert_eq!(versions_on_disk(&root), ["1.1.0"]);
    assert!(a.get(ID).is_some());

    // 제거 → 다음 시작 때 폴더도 사라진다
    a.uninstall(ID).unwrap();
    assert!(a.get(ID).is_none());
    assert_eq!(a.info(ID).status, AddonStatus::NotInstalled);
    drop(a);
    let a = Addons::open(root.clone(), vec![k.pk], Some(index));
    assert!(a.get(ID).is_none());
    assert!(!root.join(ID).exists());
}

#[test]
fn wrong_signature_is_rejected_and_nothing_is_saved() {
    let tmp = tempfile::tempdir().unwrap();
    let (root, srv) = (tmp.path().join("addons"), tmp.path().join("srv"));
    std::fs::create_dir(&srv).unwrap();
    let index = publish(&srv, &key(), &["1.0.0"]); // 앱이 모르는 키로 서명
    let a = Addons::open(root.clone(), vec![key().pk], Some(index));
    assert_eq!(install(&a).unwrap_err(), TAMPERED);
    assert!(a.get(ID).is_none());
    assert!(!root.join(ID).exists());
}

#[test]
fn wrong_hash_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let (root, srv) = (tmp.path().join("addons"), tmp.path().join("srv"));
    std::fs::create_dir(&srv).unwrap();
    let k = key();
    let index = publish(&srv, &k, &["1.0.0"]);
    let mut idx: Index = serde_json::from_slice(&std::fs::read(&index).unwrap()).unwrap();
    idx.tools[0].sha256 = "00".repeat(32);
    std::fs::write(&index, serde_json::to_vec(&idx).unwrap()).unwrap();
    let a = Addons::open(root, vec![k.pk], Some(index));
    assert_eq!(install(&a).unwrap_err(), TAMPERED);
}

/// 설치 뒤 파일이 바뀌면 시작할 때 열지 않고 "업데이트 필요"로 보인다
#[test]
fn tampered_after_install_is_not_loaded() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("addons");
    let k = key();
    let dir = version_dir(&root, ID, "1.0.0");
    write_version(&dir, b"not really a dll", &sign(&k, b"not really a dll")).unwrap();
    std::fs::write(dir.join("tool.dll"), b"swapped").unwrap();
    record(&root, "1.0.0", ABI_VERSION);
    let a = Addons::open(root, vec![k.pk], None);
    assert!(a.get(ID).is_none());
    assert_eq!(a.info(ID).status, AddonStatus::UpdateRequired);
}

/// 앱 ABI가 바뀌면 열지 않는다 (서명은 맞아도)
#[test]
fn other_abi_is_not_loaded() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("addons");
    let k = key();
    let dir = version_dir(&root, ID, "1.0.0");
    write_version(&dir, dll(), &sign(&k, dll())).unwrap();
    assert!(dir.join(SIG).is_file());
    record(&root, "1.0.0", ABI_VERSION + 1);
    let a = Addons::open(root, vec![k.pk], None);
    assert!(a.get(ID).is_none());
    assert_eq!(a.info(ID).status, AddonStatus::UpdateRequired);
}

#[test]
fn unsafe_names_in_index_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let (root, srv) = (tmp.path().join("addons"), tmp.path().join("srv"));
    std::fs::create_dir(&srv).unwrap();
    let k = key();
    let index = publish(&srv, &k, &["1.0.0"]);
    let mut idx: Index = serde_json::from_slice(&std::fs::read(&index).unwrap()).unwrap();
    idx.tools[0].version = "../../evil".into();
    std::fs::write(&index, serde_json::to_vec(&idx).unwrap()).unwrap();
    let a = Addons::open(root, vec![k.pk], Some(index));
    assert_eq!(install(&a).unwrap_err(), TAMPERED);
    assert!(!tmp.path().join("evil").exists());
}

fn record(root: &Path, version: &str, abi: u32) {
    let mut inst = Installed::default();
    inst.tools.insert(
        ID.into(),
        InstalledTool {
            version: version.into(),
            abi,
        },
    );
    write_installed(root, &inst).unwrap();
}
