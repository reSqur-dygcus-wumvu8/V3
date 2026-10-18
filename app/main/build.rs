//! Script de build pour générer les bindings MLA

use std::env;
use std::path::PathBuf;

fn main() {
    // Chemin vers le header MLA
    let mla_header = match env::var("MLA_HEADER_PATH") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            // Chemins par défaut pour trouver mla.h
            let possible_paths = [
                "/usr/local/include/mla.h",
                "/usr/include/mla.h",
                "../MLA/include/mla.h",
                "MLA/include/mla.h",
            ];
            
            let mut found_path: Option<PathBuf> = None;
            for path in possible_paths.iter() {
                let path_buf = PathBuf::from(path);
                if path_buf.exists() {
                    found_path = Some(path_buf);
                    break;
                }
            }
            
            match found_path {
                Some(path) => path,
                None => {
                    // Si aucun header trouvé, on utilise un stub
                    println!("cargo:warning=Header MLA non trouvé, utilisation du mode stub");
                    PathBuf::from("stub_mla.h")
                }
            }
        }
    };
    
    // Générer les bindings seulement si le header existe
    if mla_header.exists() {
        println!("cargo:rerun-if-changed={}", mla_header.display());
        
        // Configurer bindgen
        let bindings = bindgen::Builder::default()
            .header(mla_header.to_string_lossy())
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .allowlist_function("mla_.*")
            .allowlist_type("mla_.*")
            .generate()
            .expect("Impossible de générer les bindings MLA");
        
        // Écrire le fichier de bindings
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Impossible d'écrire les bindings MLA");
    } else {
        // Créer un fichier stub
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        std::fs::write(
            out_path.join("bindings.rs"),
            r#"
// Bindings stub pour MLA
// En production, remplacer par les vrais bindings générés par bindgen

use std::ffi::*;

#[repr(C)]
pub struct mla_ctx {
    _private: [u8; 0],
}

extern "C" {
    pub fn mla_init() -> c_int {
        0
    }
    
    pub fn mla_new() -> *mut c_void {
        std::ptr::null_mut()
    }
    
    pub fn mla_free(_ctx: *mut c_void) {
    }
    
    pub fn mla_set_key(_ctx: *mut c_void, _key: *const c_char, _key_len: c_uint) -> c_int {
        0
    }
    
    pub fn mla_encrypt(
        _ctx: *mut c_void,
        _key: *const c_char,
        _key_len: c_uint,
        _plaintext: *const c_char,
        _plaintext_len: c_uint,
        _ciphertext: *mut c_char,
        _ciphertext_len: *mut c_uint,
        _iv: *mut c_char,
        _auth_tag: *mut c_char,
    ) -> c_int {
        0
    }
    
    pub fn mla_decrypt(
        _ctx: *mut c_void,
        _key: *const c_char,
        _key_len: c_uint,
        _ciphertext: *const c_char,
        _ciphertext_len: c_uint,
        _iv: *const c_char,
        _auth_tag: *const c_char,
        _plaintext: *mut c_char,
        _plaintext_len: *mut c_uint,
    ) -> c_int {
        0
    }
}
"#
        ).expect("Impossible de créer le fichier stub");
    }
}
