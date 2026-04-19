//! T066: Firecracker rootfs preparation tests.

use worldcompute::data_plane::cid_store::CidStore;
use worldcompute::sandbox::firecracker::{assemble_rootfs, collect_layers_from_store};

#[test]
fn collect_layers_retrieves_stored_data() {
    let store = CidStore::new();
    let cid1 = store.put(b"layer-one-data").unwrap();
    let cid2 = store.put(b"layer-two-data").unwrap();

    let layers = collect_layers_from_store(&store, &[cid1, cid2]).unwrap();
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0], b"layer-one-data");
    assert_eq!(layers[1], b"layer-two-data");
}

#[test]
fn collect_layers_skips_missing_cids() {
    let store = CidStore::new();
    let cid1 = store.put(b"present-layer").unwrap();

    // Create a CID that is NOT in the store
    let other_store = CidStore::new();
    let missing_cid = other_store.put(b"not-in-main-store").unwrap();

    let layers = collect_layers_from_store(&store, &[cid1, missing_cid]).unwrap();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0], b"present-layer");
}

#[test]
fn assemble_rootfs_creates_file_with_layers() {
    let tmp = std::env::temp_dir().join("wc-t066-rootfs");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let rootfs_path = tmp.join("rootfs.ext4");
    let layers = [b"first-layer-bytes".to_vec(), b"second-layer-bytes".to_vec()];

    assemble_rootfs(&rootfs_path, &layers).unwrap();

    assert!(rootfs_path.exists());
    let contents = std::fs::read_to_string(&rootfs_path).unwrap();
    assert!(contents.contains("first-layer-bytes"));
    assert!(contents.contains("second-layer-bytes"));
    assert!(contents.contains("# worldcompute rootfs"));
    assert!(contents.contains("layer 0"));
    assert!(contents.contains("layer 1"));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn assemble_rootfs_empty_layers() {
    let tmp = std::env::temp_dir().join("wc-t066-rootfs-empty");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let rootfs_path = tmp.join("rootfs.ext4");
    assemble_rootfs(&rootfs_path, &[]).unwrap();

    assert!(rootfs_path.exists());
    let contents = std::fs::read_to_string(&rootfs_path).unwrap();
    assert!(contents.contains("# worldcompute rootfs"));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn end_to_end_store_to_rootfs() {
    let store = CidStore::new();
    let cid1 = store.put(b"bin/hello-world").unwrap();
    let cid2 = store.put(b"etc/config.yaml").unwrap();

    let layers = collect_layers_from_store(&store, &[cid1, cid2]).unwrap();

    let tmp = std::env::temp_dir().join("wc-t066-e2e");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let rootfs_path = tmp.join("rootfs.ext4");
    assemble_rootfs(&rootfs_path, &layers).unwrap();

    let contents = std::fs::read_to_string(&rootfs_path).unwrap();
    assert!(contents.contains("bin/hello-world"));
    assert!(contents.contains("etc/config.yaml"));

    let _ = std::fs::remove_dir_all(&tmp);
}
