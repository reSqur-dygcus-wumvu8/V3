
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
