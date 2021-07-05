use openssl_sys::*;
use std::borrow::{Borrow, BorrowMut};
use std::os::raw::{c_int, c_uchar, c_void};
use std::ptr;

use libc::*;

pub const EVP_PKEY_SM2: c_int = NID_sm2;
pub const NID_sm2: c_int = 1172;

extern "C" {
    pub fn EVP_sm4_ecb() -> *const EVP_CIPHER;
    pub fn EVP_sm4_cbc() -> *const EVP_CIPHER;
    pub fn EVP_sm4_ofb() -> *const EVP_CIPHER;
    pub fn EVP_sm4_ctr() -> *const EVP_CIPHER;

    pub fn EVP_PKEY_set1_EC_KEY(pkey: *mut EVP_PKEY, key: *mut EC_KEY) -> c_int;
    pub fn EVP_PKEY_set_alias_type(pkey: *mut EVP_PKEY, ttype: c_int) -> c_int;

    pub fn EVP_PKEY_CTX_set1_id(ctx: *mut EVP_PKEY_CTX, id: *mut c_uchar, id_len: c_int) -> c_int;
    pub fn EVP_MD_CTX_set_pkey_ctx(ctx: *mut EVP_MD_CTX, sctx: *mut EVP_PKEY_CTX) -> c_int;
}

pub struct SM3 {}

impl SM3 {
    pub fn hash(data: &Vec<u8>) -> Vec<u8> {
        let mut res = vec![0; 32].into_boxed_slice();
        let res_len: *mut u32 = Box::into_raw(Box::new(0));
        unsafe {
            let md = EVP_sm3();
            let md_ctx = EVP_MD_CTX_new();
            let engin: *mut ENGINE = ptr::null_mut();
            EVP_DigestInit_ex(md_ctx, md, engin);
            EVP_DigestUpdate(md_ctx, data.as_ptr() as *const c_void, data.len());
            EVP_DigestFinal_ex(md_ctx, res.as_mut_ptr(), res_len);
            EVP_MD_CTX_free(md_ctx);
            drop(Box::from_raw(res_len));
        }

        res.to_vec()
    }
}

pub struct SM4 {}

impl SM4 {
    pub fn encrypt(data: &Vec<u8>, key: &Vec<u8>, iv: &Vec<u8>) -> Vec<u8> {
        let cipher_length: *mut c_int = Box::into_raw(Box::new(data.len() as i32 + 32));
        let mut cipher_text = vec![0 as u8; data.len() + 32].into_boxed_slice();
        let final_length: *mut c_int = Box::into_raw(Box::new(0));
        let mut len = 0;
        unsafe {
            let evp_cipher = EVP_sm4_cbc();
            let ctx = EVP_CIPHER_CTX_new();
            let engin: *mut ENGINE = ptr::null_mut();
            EVP_EncryptInit_ex(
                ctx,
                evp_cipher,
                engin,
                key.as_ptr() as *const c_uchar,
                iv.as_ptr() as *const c_uchar,
            );
            EVP_EncryptUpdate(
                ctx,
                cipher_text.as_mut_ptr() as *mut c_uchar,
                cipher_length,
                data.as_ptr(),
                data.len() as i32,
            );
            // 从指定位置向后赋值
            EVP_EncryptFinal_ex(
                ctx,
                cipher_text.as_mut_ptr().offset(*cipher_length as isize) as *mut c_uchar,
                final_length,
            );
            EVP_CIPHER_CTX_free(ctx);
            len = *cipher_length + *final_length;

            drop(Box::from_raw(cipher_length));
            drop(Box::from_raw(final_length));
        }
        let mut r = cipher_text.to_vec();
        r.truncate(len as usize);
        r
    }

    pub fn decrypt(data: &Vec<u8>, key: &Vec<u8>, iv: &Vec<u8>) -> Vec<u8> {
        let cipher_length: *mut c_int = Box::into_raw(Box::new(data.len() as i32 + 32));
        let mut cipher_text = vec![0; data.len() + 32].into_boxed_slice();
        let final_length: *mut c_int = Box::into_raw(Box::new(0));
        let mut len = 0;
        unsafe {
            let evp_cipher = EVP_sm4_cbc();
            let ctx = EVP_CIPHER_CTX_new();
            let engin: *mut ENGINE = ptr::null_mut();
            EVP_DecryptInit_ex(
                ctx,
                evp_cipher,
                engin,
                key.as_ptr() as *const c_uchar,
                iv.as_ptr() as *const c_uchar,
            );
            EVP_DecryptUpdate(
                ctx,
                cipher_text.as_mut_ptr() as *mut c_uchar,
                cipher_length,
                data.as_ptr(),
                data.len() as i32,
            );
            // 从指定位置向后赋值
            EVP_DecryptFinal_ex(
                ctx,
                cipher_text.as_mut_ptr().offset(*cipher_length as isize) as *mut c_uchar,
                final_length,
            );
            EVP_CIPHER_CTX_free(ctx);
            len = *cipher_length + *final_length;
            drop(Box::from_raw(cipher_length));
            drop(Box::from_raw(final_length));
        }
        let mut r = cipher_text.to_vec();
        r.truncate(len as usize);
        r
    }
}

pub struct SM2 {}

impl SM2 {
    fn create_evp_pkey(key: &Vec<u8>, is_pub: bool) -> Result<*mut EVP_PKEY, String> {
        unsafe {
            let mut evp_key = EVP_PKEY_new();
            let mut ec_key = ptr::null_mut();
            let userdata = ptr::null_mut();
            let keybio = BIO_new_mem_buf(key.as_ptr() as *const c_void, key.len() as i32);
            if keybio == ptr::null_mut() {
                return Err(String::from("BIO_new_mem_buf failed."));
            }
            let pem_passwd_cb = Option::None;
            if is_pub {
                let ec_key = PEM_read_bio_EC_PUBKEY(keybio, &mut ec_key, pem_passwd_cb, userdata);
                if ec_key == ptr::null_mut() {
                    BIO_free_all(keybio);
                    return Err(String::from("PEM_read_bio_EC_PUBKEY failed"));
                }
                if EVP_PKEY_set1_EC_KEY(evp_key, ec_key) != 1 {
                    EC_KEY_free(ec_key);
                    BIO_free_all(keybio);
                    return Err(String::from("EVP_KEY_set1_EC_KEY failed"));
                }
                EC_KEY_free(ec_key);
            } else {
                let ec_key =
                    PEM_read_bio_ECPrivateKey(keybio, &mut ec_key, pem_passwd_cb, userdata);
                if ec_key == ptr::null_mut() {
                    BIO_free_all(keybio);
                    return Err(String::from("PEM_read_bio_ECPrivateKey failed"));
                }
                if EVP_PKEY_set1_EC_KEY(evp_key, ec_key) != 1 {
                    EC_KEY_free(ec_key);
                    BIO_free_all(keybio);
                    return Err(String::from("EVP_KEY_set1_EC_KEY failed"));
                }
                EC_KEY_free(ec_key);
            }
            BIO_free_all(keybio);

            Ok(evp_key)
        }
    }

    pub fn encrypt(data: &Vec<u8>, pubKey: &Vec<u8>) -> Result<Vec<u8>, String> {
        let mut r = vec![];
        unsafe {
            let ciphertext_len: *mut size_t = Box::into_raw(Box::new(0));
            let evp_key = match SM2::create_evp_pkey(pubKey, true) {
                Ok(evp_key) => evp_key,
                Err(e) => return Err(e),
            };
            EVP_PKEY_set_alias_type(evp_key, EVP_PKEY_SM2);
            let ectx = EVP_PKEY_CTX_new(evp_key, ptr::null_mut());
            EVP_PKEY_encrypt_init(ectx);
            EVP_PKEY_encrypt(
                ectx,
                ptr::null_mut(),
                ciphertext_len,
                data.as_ptr(),
                data.len(),
            );
            // 应该在长度初始化之后再定义cipherText大小
            let mut cipher_text = vec![0; *ciphertext_len].into_boxed_slice();
            EVP_PKEY_encrypt(
                ectx,
                cipher_text.as_mut_ptr(),
                ciphertext_len,
                data.as_ptr(),
                data.len(),
            );
            EVP_PKEY_free(evp_key);
            EVP_PKEY_CTX_free(ectx);
            r = cipher_text.to_vec();
        }
        Ok(r)
    }

    pub fn decrypt(data: &Vec<u8>, priKey: &Vec<u8>) -> Result<Vec<u8>, String> {
        let mut r = vec![];
        unsafe {
            let ciphertext_len: *mut size_t = Box::into_raw(Box::new(0));

            let mut pkey = match SM2::create_evp_pkey(priKey, false) {
                Ok(evp_key) => evp_key,
                Err(e) => return Err(e),
            };
            EVP_PKEY_set_alias_type(pkey, EVP_PKEY_SM2);
            let ectx = EVP_PKEY_CTX_new(pkey, ptr::null_mut());
            EVP_PKEY_decrypt_init(ectx);
            EVP_PKEY_decrypt(
                ectx,
                ptr::null_mut(),
                ciphertext_len,
                data.as_ptr(),
                data.len(),
            );
            let mut cipher_text = vec![0; *ciphertext_len].into_boxed_slice();
            EVP_PKEY_decrypt(
                ectx,
                cipher_text.as_mut_ptr(),
                ciphertext_len,
                data.as_ptr(),
                data.len(),
            );
            EVP_PKEY_free(pkey);
            EVP_PKEY_CTX_free(ectx);
            // 处理返回值的长度
            let mut result_vec = cipher_text.to_vec();
            result_vec.truncate(*ciphertext_len);
            r = result_vec;
        }
        Ok(r)
    }

    // https://nongguangxin.cn/%E5%9B%BD%E5%AF%86SM2-OpenSSL-EVP%E6%8E%A5%E5%8F%A3%E4%BE%8B%E5%AD%90.html
    
    pub fn sign(data: &Vec<u8>, priKey: &Vec<u8>) -> Result<Vec<u8>, String> {
        let mut r = vec![];
        unsafe{
            let mut pkey = match SM2::create_evp_pkey(priKey, false) {
                Ok(evp_key) => evp_key,
                Err(e) => return Err(e),
            };
            // EVP_PKEY_set_alias_type(pkey, EVP_PKEY_SM2);
            
            let mut evpMdCtx: *mut EVP_MD_CTX = EVP_MD_CTX_new();
            let mut sctx = EVP_PKEY_CTX_new(pkey, ptr::null_mut());
            // EVP_PKEY_CTX_set1_id(sctx, data.as_mut_ptr(), data.len() as i32);
            EVP_MD_CTX_set_pkey_ctx(evpMdCtx, sctx);

            EVP_DigestSignInit(evpMdCtx, ptr::null_mut(), EVP_sm3(), ptr::null_mut(), pkey.as_ptr());
            EVP_DigestSign(evpMdCtx, ptr::null_mut(), )

            if EVP_SignInit_ex(evpMdCtx, EVP_sm3(), ptr::null_mut()) != 1 {
                return Err(String::from(""));
            }
            if EVP_SignUpdate(evpMdCtx, data.as_ptr(), data.len() as i32) != 1 {
                return Err(String::from(""));
            }
            let len_sig: *mut i32 = Box::into_raw(Box::new(0));
            let mut sig = vec![0; *len_sig as usize].into_boxed_slice();
            if EVP_SignFinal(evpMdCtx, sig.as_mut_ptr(), len_sig, pkey) != 1 {
                return Err(String::from(""));
            }
            EVP_MD_CTX_free(evpMdCtx);
            EVP_PKEY_free(pkey);
            let mut result_vec=  sig.to_vec();
            result_vec.truncate(*len_sig as usize);
            r = result_vec;
        }
        Ok(r)
    }
    
    pub fn verify(data: &Vec<u8>, oldData: &Vec<u8>, pubKey: &Vec<u8>) -> Result<bool, String> {
        unsafe {
            let evp_key = match SM2::create_evp_pkey(pubKey, true) {
                Ok(evp_key) => evp_key,
                Err(e) => return Err(e),
            };
            EVP_PKEY_set_alias_type(evp_key, EVP_PKEY_SM2);
            let mut evpMdCtx: *mut EVP_MD_CTX = EVP_MD_CTX_new();
            EVP_MD_CTX_init(evpMdCtx);
            if EVP_VerifyInit_ex(evpMdCtx, EVP_sm3(), ptr::null_mut()) != 1  {
                return Err(String::from(""));
            }
            if EVP_VerifyUpdate(evpMdCtx, oldData.as_ptr(), oldData.len() as i32) != 1 {
                return Err(String::from(""));
            }
            if EVP_VerifyFinal(evpMdCtx, data.as_ptr(), data.len(), evp_key) != 1 {
                return Err(String::from(""));
            }
            
            EVP_PKEY_free(evp_key);
            EVP_MD_CTX_free(evpMdCtx);
        }
        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sm3() {
        let v = vec![
            1, 3, 52, 3, 63, 64, 63, 2, 54, 36, 92, 67, 26, 7, 46, 87, 64,
        ];
        println!("{:#?}", SM3::hash(&v));
    }

    #[test]
    fn sm4() {
        let buffer = vec![
            1, 3, 52, 3, 63, 64, 63, 2, 54, 36, 92, 67, 26, 7, 46, 87, 64,
        ];
        let key = vec![12, 21, 43, 53, 21, 1, 42, 53, 53, 5, 4, 67, 5, 6, 7, 8];
        let iv = vec![12, 21, 43, 53, 21, 1, 42, 53, 53, 5, 4, 67, 5, 6, 7, 8];
        let enc_data = SM4::encrypt(&buffer, &key, &iv);
        println!("enc_data: {:#?}", enc_data);
        let dec_data = SM4::decrypt(&enc_data, &key, &iv);
        assert_eq!(buffer, dec_data);
    }

    #[test]
    fn sm2() {
        let private_key = String::from(
            "-----BEGIN EC PARAMETERS-----
BggqgRzPVQGCLQ==
-----END EC PARAMETERS-----
-----BEGIN EC PRIVATE KEY-----
MHcCAQEEINJRYi7nHKfAkCwCKnEAzjLmpnYsj3lXJhU0WGXiNdKooAoGCCqBHM9V
AYItoUQDQgAEFtXYB9anklMdp9c19S6Gq/lgaxUiv6T0BhtziIZx5XKcnj1NnUvb
DXLMUBv1v60nxmNYvzACZ1/HMTpmi7jCRg==
-----END EC PRIVATE KEY-----",
        )
        .into_bytes();
        let public_key = String::from(
            "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoEcz1UBgi0DQgAEFtXYB9anklMdp9c19S6Gq/lgaxUi
v6T0BhtziIZx5XKcnj1NnUvbDXLMUBv1v60nxmNYvzACZ1/HMTpmi7jCRg==
-----END PUBLIC KEY-----",
        )
        .into_bytes();
        let buffer = vec![
            1, 3, 52, 3, 63, 64, 63, 2, 54, 36, 92, 67, 26, 7, 46, 87, 64,
        ];
        let enc_data = SM2::encrypt(&buffer, &public_key).unwrap();
        let dec_data = SM2::decrypt(&enc_data, &private_key).unwrap();
        assert_eq!(buffer, dec_data);
    }

    #[test]
    fn sm2_signverify() {
        let private_key = String::from(
            "-----BEGIN EC PARAMETERS-----
BggqgRzPVQGCLQ==
-----END EC PARAMETERS-----
-----BEGIN EC PRIVATE KEY-----
MHcCAQEEINJRYi7nHKfAkCwCKnEAzjLmpnYsj3lXJhU0WGXiNdKooAoGCCqBHM9V
AYItoUQDQgAEFtXYB9anklMdp9c19S6Gq/lgaxUiv6T0BhtziIZx5XKcnj1NnUvb
DXLMUBv1v60nxmNYvzACZ1/HMTpmi7jCRg==
-----END EC PRIVATE KEY-----",
        )
        .into_bytes();
        let public_key = String::from(
            "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoEcz1UBgi0DQgAEFtXYB9anklMdp9c19S6Gq/lgaxUi
v6T0BhtziIZx5XKcnj1NnUvbDXLMUBv1v60nxmNYvzACZ1/HMTpmi7jCRg==
-----END PUBLIC KEY-----",
        )
        .into_bytes();
        let buffer = vec![
            1, 3, 52, 3, 63, 64, 63, 2, 54, 36, 92, 67, 26, 7, 46, 87, 64,
        ];
        let sign_data = SM2::sign(&buffer, &public_key).unwrap();
        assert_eq!(true, SM2::verify(&sign_data, &buffer, &public_key).unwrap());
    }
}
