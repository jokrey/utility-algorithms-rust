extern crate ring;
extern crate untrusted;
extern crate crypto;
extern crate base64;

use ring::rand;
use ring::digest;
use ring::agreement;
use ring::error;
use ring::rand::SecureRandom;

use crypto::aes;
use crypto::aes::{KeySize};
use crypto::symmetriccipher::SynchronousStreamCipher;
use network::mcnp::mcnp_connection::McnpConnection;
use std::str;
use network::mcnp::mcnp_connection::McnpConnectionTraits;
use encoding::bytes::libae_storage_system::StorageSystemError;


//ECDH Key Agreement
pub fn generate_private_key() -> Result<agreement::EphemeralPrivateKey, error::Unspecified> {
    agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rand::SystemRandom::new())
}
///allocates space on the heap, if that is undesired copy paste this code
pub fn compute_public_key(private_key:&agreement::EphemeralPrivateKey) -> Result<Vec<u8>, error::Unspecified> {
    let mut my_public_key = [0u8; agreement::PUBLIC_KEY_MAX_LEN];
    let my_public_key = &mut my_public_key[..private_key.public_key_len()];
    private_key.compute_public_key(my_public_key)?;
    Ok(Vec::from(my_public_key))
}
pub fn generate_secure_secret(shared_secret:&[u8], pub_key_1:&[u8], pub_key_2:&[u8]) -> digest::Digest {
    let mut pub1bigger:Option<bool> = None;
    for i in 0..pub_key_1.len() {
        if pub_key_1[i] > pub_key_2[i] {
            pub1bigger = Some(true);
            break;
        } else if pub_key_1[i] < pub_key_2[i] {
            pub1bigger = Some(false);
            break;
        }
    }

    let mut ctx = digest::Context::new(&digest::SHA256);
    ctx.update(shared_secret);
    if pub1bigger == Some(true) {
        ctx.update(pub_key_1);
        ctx.update(pub_key_2);
    } else if pub1bigger == Some(false) {
        ctx.update(pub_key_2);
        ctx.update(pub_key_1);
    } else { //if pub1bigger == None, then both public keys are equal by impossible chance and the order does not matter.
        ctx.update(pub_key_2);
        ctx.update(pub_key_1);
    }
    ctx.finish()
}

//AES CTR NO PADDING
pub fn generate_128bit_nonce() -> Vec<u8> {
    let rng = rand::SystemRandom::new();
    let mut nonce = vec![0u8; 16];
    rng.fill(nonce.as_mut_slice()).expect("randomness could not be generated");
    return nonce;
}
pub fn aes_crt_np_128_encrypt(message:&[u8], key:&[u8], nonce:&[u8]) -> Vec<u8> {
    let mut cipher = aes::ctr(KeySize::KeySize128, &key[..16], &nonce[..16]);
    let mut output = vec![0u8; message.len()];
    cipher.process(message, output.as_mut_slice());
    return output;
}
pub fn aes_crt_np_128_decrypt(message:&[u8], key:&[u8], nonce:&[u8]) -> Vec<u8> {
    aes_crt_np_128_encrypt(message, key, nonce)
}

pub fn do_key_exchange(private_key:agreement::EphemeralPrivateKey, my_public_key:&[u8], received_remote_public_key:&[u8]) -> Result<Vec<u8>, ring::error::Unspecified> {
    agreement::agree_ephemeral(private_key, &agreement::ECDH_P256,untrusted::Input::from(&received_remote_public_key), ring::error::Unspecified,
                               |key_material| {
                                   let generated_secure_secret = generate_secure_secret(key_material, my_public_key, &received_remote_public_key);
                                   Ok(Vec::from(generated_secure_secret.as_ref()))
                               })
}

pub fn sha256(message:&[u8]) -> Vec<u8> {
    Vec::from(digest::digest(&digest::SHA256, message).as_ref())
}
pub fn sha1(message:&[u8]) -> Vec<u8> {
    Vec::from(digest::digest(&digest::SHA1, message).as_ref())
}
pub fn base64(message:&[u8]) -> String {
    base64::encode(message)
}


pub fn hashed(user_name:&[u8]) -> String {
    base64(&sha1(user_name))
}
pub fn actual_tag_from(user_name:&str, tag:&str) -> String {
    format!("{}{}", &hashed(user_name.as_bytes()), tag)
}
pub fn actual_tag(user_name_hash:&str, tag:&[u8]) -> String {
    format!("{}{}", user_name_hash, str::from_utf8(tag).unwrap())
}
pub fn password_store_tag_for_user_name_hash(user_name_hash:&str) -> String {
    let mut password_store_tag_for_user_name = "#*".to_string(); //can never by access by any user, because base64 chars don't include # or *
    password_store_tag_for_user_name.push_str(&user_name_hash);
    password_store_tag_for_user_name
}

pub fn send_tag(connection:&mut McnpConnection, tag:&str, session_key:&[u8]) -> Result<(), StorageSystemError> {
    let nonce = generate_128bit_nonce();
    connection.send_fixed_chunk_u8_arr(&nonce)?;
    connection.send_variable_chunk(&aes_crt_np_128_encrypt(&concat(&nonce,tag.as_bytes()), session_key, &nonce))?;
    Ok(())
}
pub fn receive_tag(connection:&mut McnpConnection, user_name_hash:&str, session_key:&[u8]) -> Result<String, StorageSystemError> {
    let nonce = connection.read_fixed_chunk_u8_arr(16)?;
    let noncesig_tagbytes = split(&aes_crt_np_128_decrypt(&connection.read_variable_chunk()?, session_key, &nonce), 16);
    let nonce_signature = &noncesig_tagbytes[0];
    if &nonce != nonce_signature {
        Err(StorageSystemError::new("Signature Authentication failed. User not verified."))
    } else {
        let tag_bytes = &noncesig_tagbytes[1];
        Ok(actual_tag(user_name_hash, tag_bytes))
    }
}



//helper
fn concat(b1:&[u8], b2:&[u8]) -> Vec<u8> {
    let mut concat = Vec::with_capacity(b1.len() + b2.len());
    concat.extend_from_slice(b1);
    concat.extend_from_slice(b2);
    concat
}
fn split(b:&[u8], pos:usize) -> [Vec<u8>;2] {
    let b1 = Vec::from(&b[..pos]);
    let b2 = Vec::from(&b[pos..]);
    return [b1, b2];
}














#[test]
fn test_and_demonstrate_ecdh_func() {
    let my_private_key = generate_private_key().expect("priv gen failed");
    let my_public_key = compute_public_key(&my_private_key).expect("priv gen failed");

    //send key_pair
    //SEND key_pair.1
    println!("send: {:?}", my_public_key);
    println!("send len: {:?}", my_public_key.len());

    let received_remote_public_key = compute_public_key(&generate_private_key().unwrap()).unwrap();

    agreement::agree_ephemeral(my_private_key, &agreement::ECDH_P256,untrusted::Input::from(&received_remote_public_key), ring::error::Unspecified,
                               |key_material| {
                                   println!("key material??:  {:?}", key_material);
                                   println!("key material.len:  {}", key_material.len());


                                   let generated_secure_secret = generate_secure_secret(key_material, &my_public_key, &received_remote_public_key);

                                   println!("hashed_secret??:  {:?}", generated_secure_secret.as_ref());



                                   //SUBSEQUENT AES  (need to use 128, because java(without some work) is restricted to 128 bit keys. Which is dumb, but just a fact)
//                                   let mut key = generated_secure_secret.as_ref();
//                                   let nonce = generate_128bit_nonce();
                                   let key = vec![1,2,3,4,5,6,7,8,9,0,1,2,3,4,5,6];
                                   let nonce = vec![1,2,3,4,5,6,7,8,9,0,1,2,3,4,5,6];
                                   let secret = "111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111";

                                   let encrypted = aes_crt_np_128_encrypt(secret.as_bytes(), &key, &nonce);
                                   let decrypted = aes_crt_np_128_encrypt(&encrypted, &key, &nonce);


                                   //TRANSFER THE FOLLOWING
                                   println!("Nonce: {:?}", nonce);
                                   println!("Ciphertext: {:?}", encrypted);
                                   println!("secret: {:?}", secret.as_bytes());
                                   println!("output2: {:?}", decrypted);

                                   Ok(())
                               }).expect("agreement::agree_ephemeral failed");
}


























//pub struct RSAPublicKey {
//    public_key_mod:Vec<u8>,
//    public_key_exp:Vec<u8>
//}
//
//impl RSAPublicKey {
//    pub fn new(public_key_mod:Vec<u8>, public_key_exp:Vec<u8>) -> RSAPublicKey {
//        RSAPublicKey { public_key_mod, public_key_exp }
//    }
//    fn get_ring_representation(&self) -> (untrusted::Input, untrusted::Input) {
//        (untrusted::Input::from(&self.public_key_mod), untrusted::Input::from(&self.public_key_exp))
//    }
//}
//
//pub fn hash_and_sign(message: &[u8], private_key_pkcs8_encoded:&[u8]) -> Vec<u8> {
//    sign(digest::digest(&digest::SHA256, message).as_ref(), private_key_pkcs8_encoded)
//}
//pub fn sign(message: &[u8], private_key_pkcs8_encoded:&[u8]) -> Vec<u8> {
//    let key_pair = signature::RSAKeyPair::from_pkcs8(untrusted::Input::from(&private_key_pkcs8_encoded)).expect("getting private key failed");
//
//    let key_pair = Arc::new(key_pair);
//    let mut signing_state = signature::RSASigningState::new(key_pair).expect("getting signing state failed");
//
//    let rng = rand::SystemRandom::new();
//    let mut signature = vec![0; signing_state.key_pair().public_modulus_len()];
//    signing_state.sign(&signature::RSA_PKCS1_SHA256, &rng, message,
//                       &mut signature).expect("signing failed");
//    signature
//}
//
//pub fn verify_hashed(message: &[u8], signature: &[u8], public_key:&RSAPublicKey) -> bool {
//    verify(digest::digest(&digest::SHA256, message).as_ref(), signature, public_key)
//}
/////the public key components are to be unsigned, big endian encoded integers. They tend to be quite huge.
//pub fn verify(message: &[u8], signature: &[u8], public_key:&RSAPublicKey) -> bool {
//    match signature::primitive::verify_rsa(&signature::RSA_PKCS1_2048_8192_SHA256, public_key.get_ring_representation(), untrusted::Input::from(message), untrusted::Input::from(signature)) {
//        Ok(_) => true,
//        Err(_) => {
//            false
//        }
//    }
//}
//
//
//#[test]
//fn test_and_demonstrate_rsa_signing_functionality() {
//    //keys generated in java using a sha-256 hash of the byte array utf8 representation of "hallo12345"
//    let private_key_pkcs8 = vec![48, 130, 4, 190, 2, 1, 0, 48, 13, 6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 1, 5, 0, 4, 130, 4, 168, 48, 130, 4, 164, 2, 1, 0, 2, 130, 1, 1, 0, 137, 205, 227, 190, 75, 112, 157, 234, 49, 30, 35, 187, 217, 193, 80, 119, 61, 185, 110, 230, 244, 154, 80, 210, 53, 106, 244, 83, 218, 67, 98, 6, 239, 224, 93, 224, 70, 13, 78, 232, 87, 97, 161, 74, 119, 115, 173, 150, 238, 119, 20, 51, 31, 200, 192, 9, 160, 252, 164, 190, 40, 189, 64, 119, 20, 19, 25, 90, 239, 82, 247, 88, 245, 105, 190, 114, 69, 114, 15, 15, 248, 172, 78, 130, 234, 130, 172, 167, 128, 92, 211, 5, 64, 181, 73, 217, 34, 233, 66, 28, 58, 3, 63, 219, 69, 243, 194, 35, 101, 238, 212, 15, 137, 40, 50, 223, 60, 20, 48, 161, 225, 236, 199, 116, 155, 87, 175, 208, 235, 39, 56, 152, 44, 193, 67, 95, 129, 104, 61, 185, 170, 15, 143, 44, 232, 245, 106, 67, 107, 132, 123, 145, 234, 137, 63, 32, 230, 45, 35, 202, 94, 181, 153, 180, 140, 24, 170, 44, 252, 174, 221, 243, 168, 30, 24, 79, 34, 117, 245, 28, 40, 229, 104, 33, 123, 11, 83, 252, 112, 235, 23, 160, 120, 51, 220, 13, 230, 29, 107, 49, 94, 237, 216, 199, 109, 205, 73, 61, 128, 94, 166, 101, 108, 129, 11, 224, 192, 59, 146, 5, 117, 151, 144, 250, 155, 217, 242, 223, 139, 168, 38, 48, 218, 126, 248, 243, 124, 171, 22, 132, 55, 43, 12, 161, 60, 194, 211, 151, 61, 170, 58, 37, 85, 183, 225, 145, 2, 3, 1, 0, 1, 2, 130, 1, 0, 123, 67, 157, 217, 212, 37, 82, 59, 239, 223, 163, 219, 30, 119, 27, 0, 238, 71, 118, 122, 68, 133, 220, 145, 139, 146, 182, 38, 99, 112, 46, 185, 65, 204, 146, 108, 80, 125, 10, 254, 45, 91, 121, 40, 225, 28, 170, 67, 253, 222, 170, 68, 232, 195, 107, 115, 177, 123, 11, 233, 197, 11, 52, 36, 207, 226, 29, 166, 7, 185, 80, 227, 83, 242, 88, 150, 98, 164, 25, 241, 17, 97, 31, 129, 95, 63, 176, 44, 204, 87, 59, 178, 209, 36, 216, 127, 208, 8, 146, 72, 41, 100, 74, 180, 91, 40, 37, 154, 0, 77, 215, 134, 102, 11, 125, 37, 205, 217, 201, 126, 164, 86, 102, 59, 89, 208, 223, 196, 82, 239, 105, 179, 152, 208, 119, 37, 184, 88, 13, 166, 11, 163, 71, 156, 108, 4, 147, 33, 36, 13, 41, 81, 116, 16, 22, 246, 211, 126, 34, 183, 248, 100, 203, 162, 2, 234, 81, 224, 172, 76, 235, 89, 51, 189, 125, 151, 189, 154, 187, 3, 9, 253, 197, 55, 170, 142, 28, 203, 157, 19, 246, 128, 211, 90, 232, 90, 240, 89, 58, 227, 179, 11, 100, 235, 99, 188, 26, 116, 171, 99, 156, 51, 110, 134, 102, 72, 246, 164, 4, 76, 217, 218, 45, 127, 222, 206, 74, 128, 59, 169, 239, 126, 75, 46, 74, 94, 34, 190, 110, 60, 68, 166, 153, 188, 189, 178, 175, 48, 1, 109, 87, 144, 198, 50, 239, 153, 2, 129, 129, 0, 187, 235, 70, 192, 207, 254, 183, 3, 222, 216, 90, 117, 218, 142, 76, 10, 59, 157, 24, 251, 154, 255, 154, 52, 245, 109, 145, 165, 170, 171, 70, 187, 61, 190, 168, 12, 223, 212, 133, 79, 193, 105, 98, 45, 152, 186, 99, 31, 194, 68, 131, 124, 236, 246, 114, 168, 56, 83, 159, 234, 146, 181, 62, 39, 137, 143, 214, 54, 85, 164, 243, 34, 169, 154, 117, 126, 54, 241, 189, 198, 68, 179, 173, 149, 223, 251, 236, 93, 64, 189, 238, 24, 216, 184, 228, 233, 188, 21, 206, 211, 154, 209, 8, 145, 59, 163, 5, 94, 31, 12, 103, 233, 27, 134, 78, 133, 134, 133, 155, 159, 119, 73, 95, 121, 39, 218, 47, 179, 2, 129, 129, 0, 187, 186, 171, 206, 234, 21, 81, 223, 242, 138, 33, 211, 86, 150, 51, 99, 182, 39, 53, 219, 116, 209, 254, 176, 94, 126, 93, 210, 241, 251, 29, 154, 49, 180, 71, 31, 34, 19, 157, 103, 203, 95, 202, 166, 49, 170, 199, 81, 238, 63, 146, 151, 244, 82, 10, 58, 89, 229, 252, 190, 247, 86, 246, 45, 102, 223, 128, 137, 111, 179, 62, 44, 119, 14, 140, 22, 99, 132, 47, 240, 138, 9, 63, 103, 9, 30, 111, 81, 123, 200, 135, 148, 50, 216, 31, 224, 164, 223, 100, 199, 186, 102, 169, 82, 84, 196, 75, 19, 249, 253, 81, 141, 55, 152, 214, 42, 18, 169, 252, 173, 210, 117, 207, 175, 78, 147, 103, 171, 2, 129, 129, 0, 164, 21, 223, 116, 242, 233, 61, 211, 18, 93, 166, 55, 108, 60, 126, 39, 29, 64, 162, 148, 232, 21, 178, 7, 246, 25, 211, 104, 109, 235, 26, 90, 218, 162, 68, 200, 225, 21, 7, 198, 201, 98, 132, 136, 189, 232, 90, 47, 92, 9, 73, 42, 231, 26, 150, 169, 78, 109, 174, 160, 59, 180, 40, 110, 139, 142, 94, 4, 153, 169, 235, 103, 99, 226, 236, 30, 230, 73, 21, 101, 47, 142, 24, 207, 90, 129, 246, 52, 195, 24, 84, 243, 187, 33, 79, 56, 204, 179, 218, 34, 40, 247, 199, 92, 81, 79, 154, 155, 65, 207, 42, 88, 128, 97, 56, 229, 28, 190, 67, 81, 237, 237, 210, 128, 207, 12, 148, 67, 2, 129, 128, 59, 38, 8, 198, 11, 249, 37, 175, 226, 242, 100, 207, 250, 195, 30, 115, 247, 75, 137, 107, 152, 246, 37, 66, 26, 179, 196, 10, 23, 214, 32, 48, 154, 34, 140, 26, 34, 25, 126, 9, 219, 9, 86, 135, 96, 180, 199, 82, 104, 55, 189, 143, 133, 26, 104, 64, 148, 92, 163, 114, 227, 233, 145, 109, 34, 177, 159, 5, 46, 157, 146, 36, 94, 106, 197, 246, 179, 234, 77, 84, 131, 153, 128, 81, 141, 140, 250, 83, 249, 37, 104, 154, 104, 30, 178, 132, 140, 78, 26, 169, 215, 112, 75, 63, 54, 152, 22, 115, 183, 219, 121, 219, 125, 189, 249, 20, 142, 134, 226, 167, 61, 221, 130, 207, 96, 121, 143, 59, 2, 129, 129, 0, 151, 38, 55, 176, 113, 124, 244, 85, 191, 164, 56, 133, 129, 49, 108, 5, 149, 217, 208, 119, 214, 81, 152, 31, 5, 139, 80, 134, 207, 238, 52, 221, 5, 186, 114, 141, 157, 205, 78, 9, 33, 181, 106, 53, 143, 51, 232, 19, 75, 137, 76, 89, 33, 23, 240, 181, 70, 235, 155, 25, 140, 69, 171, 144, 76, 254, 43, 64, 15, 139, 242, 150, 123, 208, 100, 95, 231, 237, 166, 86, 196, 86, 192, 82, 101, 54, 233, 230, 230, 130, 195, 213, 220, 68, 51, 59, 255, 99, 200, 6, 43, 43, 67, 22, 192, 106, 52, 95, 178, 205, 108, 167, 181, 189, 100, 222, 194, 17, 61, 246, 206, 247, 127, 3, 52, 125, 222, 68];
//    let public_key_exp_bytes = vec![1, 0, 1];
//    let public_key_mod_bytes = vec![137, 205, 227, 190, 75, 112, 157, 234, 49, 30, 35, 187, 217, 193, 80, 119, 61, 185, 110, 230, 244, 154, 80, 210, 53, 106, 244, 83, 218, 67, 98, 6, 239, 224, 93, 224, 70, 13, 78, 232, 87, 97, 161, 74, 119, 115, 173, 150, 238, 119, 20, 51, 31, 200, 192, 9, 160, 252, 164, 190, 40, 189, 64, 119, 20, 19, 25, 90, 239, 82, 247, 88, 245, 105, 190, 114, 69, 114, 15, 15, 248, 172, 78, 130, 234, 130, 172, 167, 128, 92, 211, 5, 64, 181, 73, 217, 34, 233, 66, 28, 58, 3, 63, 219, 69, 243, 194, 35, 101, 238, 212, 15, 137, 40, 50, 223, 60, 20, 48, 161, 225, 236, 199, 116, 155, 87, 175, 208, 235, 39, 56, 152, 44, 193, 67, 95, 129, 104, 61, 185, 170, 15, 143, 44, 232, 245, 106, 67, 107, 132, 123, 145, 234, 137, 63, 32, 230, 45, 35, 202, 94, 181, 153, 180, 140, 24, 170, 44, 252, 174, 221, 243, 168, 30, 24, 79, 34, 117, 245, 28, 40, 229, 104, 33, 123, 11, 83, 252, 112, 235, 23, 160, 120, 51, 220, 13, 230, 29, 107, 49, 94, 237, 216, 199, 109, 205, 73, 61, 128, 94, 166, 101, 108, 129, 11, 224, 192, 59, 146, 5, 117, 151, 144, 250, 155, 217, 242, 223, 139, 168, 38, 48, 218, 126, 248, 243, 124, 171, 22, 132, 55, 43, 12, 161, 60, 194, 211, 151, 61, 170, 58, 37, 85, 183, 225, 145];
//
//    const MESSAGE: &'static [u8] = b"hello, world";
//
//    let public_key = RSAPublicKey::new(public_key_mod_bytes, public_key_exp_bytes);
//
//    let signature = sign(MESSAGE, &private_key_pkcs8);
//    let signature_from_hashed = hash_and_sign(MESSAGE, &private_key_pkcs8);
//
//    //transfer message, signature, and public key parts to remote connection
//
//    assert!(verify(MESSAGE, &signature, &public_key));
//    assert!(!   verify_hashed(MESSAGE, &signature, &public_key));
//
//    assert!(verify_hashed(MESSAGE, &signature_from_hashed, &public_key));
//    assert!(!   verify(MESSAGE, &signature_from_hashed, &public_key));
//
////    println!("get_sha256_hash: {:?}", digest::digest(&digest::SHA256,MESSAGE).as_ref());
//}