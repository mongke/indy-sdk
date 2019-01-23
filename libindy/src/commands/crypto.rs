use std::collections::HashMap;
use std::rc::Rc;
use std::str;

use domain::crypto::key::{Key, KeyInfo, KeyMetadata};
use errors::prelude::*;
use services::crypto::CryptoService;
use services::wallet::{RecordOptions, WalletService};

pub enum CryptoCommand {
    CreateKey(
        i32, // wallet handle
        KeyInfo, // key info
        Box<Fn(IndyResult<String/*verkey*/>) + Send>),
    SetKeyMetadata(
        i32, // wallet handle
        String, // verkey
        String, // metadata
        Box<Fn(IndyResult<()>) + Send>),
    GetKeyMetadata(
        i32, // wallet handle
        String, // verkey
        Box<Fn(IndyResult<String>) + Send>),
    CryptoSign(
        i32, // wallet handle
        String, // my vk
        Vec<u8>, // msg
        Box<Fn(IndyResult<Vec<u8>>) + Send>),
    CryptoVerify(
        String, // their vk
        Vec<u8>, // msg
        Vec<u8>, // signature
        Box<Fn(IndyResult<bool>) + Send>),
    AuthenticatedEncrypt(
        i32, // wallet handle
        String, // my vk
        String, // their vk
        Vec<u8>, // msg
        Box<Fn(IndyResult<Vec<u8>>) + Send>),
    AuthenticatedDecrypt(
        i32, // wallet handle
        String, // my vk
        Vec<u8>, // encrypted msg
        Box<Fn(IndyResult<(String, Vec<u8>)>) + Send>),
    AnonymousEncrypt(
        String, // their vk
        Vec<u8>, // msg
        Box<Fn(IndyResult<Vec<u8>>) + Send>),
    AnonymousDecrypt(
        i32, // wallet handle
        String, // my vk
        Vec<u8>, // msg
        Box<Fn(IndyResult<Vec<u8>>) + Send>)
}

pub struct CryptoCommandExecutor {
    wallet_service: Rc<WalletService>,
    crypto_service: Rc<CryptoService>,
}

impl CryptoCommandExecutor {
    pub fn new(wallet_service: Rc<WalletService>,
               crypto_service: Rc<CryptoService>,
    ) -> CryptoCommandExecutor {
        CryptoCommandExecutor {
            wallet_service,
            crypto_service,
        }
    }

    pub fn execute(&self, command: CryptoCommand) {
        match command {
            CryptoCommand::CreateKey(wallet_handle, key_info, cb) => {
                info!("CreateKey command received");
                cb(self.create_key(wallet_handle, &key_info));
            }
            CryptoCommand::SetKeyMetadata(wallet_handle, verkey, metadata, cb) => {
                info!("SetKeyMetadata command received");
                cb(self.set_key_metadata(wallet_handle, &verkey, &metadata));
            }
            CryptoCommand::GetKeyMetadata(wallet_handle, verkey, cb) => {
                info!("GetKeyMetadata command received");
                cb(self.get_key_metadata(wallet_handle, &verkey));
            }
            CryptoCommand::CryptoSign(wallet_handle, my_vk, msg, cb) => {
                info!("CryptoSign command received");
                cb(self.crypto_sign(wallet_handle, &my_vk, &msg));
            }
            CryptoCommand::CryptoVerify(their_vk, msg, signature, cb) => {
                info!("CryptoVerify command received");
                cb(self.crypto_verify(&their_vk, &msg, &signature));
            }
            CryptoCommand::AuthenticatedEncrypt(wallet_handle, my_vk, their_vk, msg, cb) => {
                info!("AuthenticatedEncrypt command received");
                cb(self.authenticated_encrypt(wallet_handle, &my_vk, &their_vk, &msg));
            }
            CryptoCommand::AuthenticatedDecrypt(wallet_handle, my_vk, encrypted_msg, cb) => {
                info!("AuthenticatedDecrypt command received");
                cb(self.authenticated_decrypt(wallet_handle, &my_vk, &encrypted_msg));
            }
            CryptoCommand::AnonymousEncrypt(their_vk, msg, cb) => {
                info!("AnonymousEncrypt command received");
                cb(self.anonymous_encrypt(&their_vk, &msg));
            }
            CryptoCommand::AnonymousDecrypt(wallet_handle, my_vk, encrypted_msg, cb) => {
                info!("AnonymousDecrypt command received");
                cb(self.anonymous_decrypt(wallet_handle, &my_vk, &encrypted_msg));
            }
        };
    }

    fn create_key(&self, wallet_handle: i32, key_info: &KeyInfo) -> IndyResult<String> {
        debug!("create_key >>> wallet_handle: {:?}, key_info: {:?}", wallet_handle, secret!(key_info));

        let key = self.crypto_service.create_key(key_info)?;
        self.wallet_service.add_indy_object(wallet_handle, &key.verkey, &key, &HashMap::new())?;

        let res = key.verkey.to_string();
        debug!("create_key <<< res: {:?}", res);
        Ok(res)
    }

    fn crypto_sign(&self,
                   wallet_handle: i32,
                   my_vk: &str,
                   msg: &[u8]) -> IndyResult<Vec<u8>> {
        debug!("crypto_sign >>> wallet_handle: {:?}, sender_vk: {:?}, msg: {:?}", wallet_handle, my_vk, msg);

        self.crypto_service.validate_key(my_vk)?;

        let key: Key = self.wallet_service.get_indy_object(wallet_handle, &my_vk, &RecordOptions::id_value())?;

        let res = self.crypto_service.sign(&key, msg)?;

        debug!("crypto_sign <<< res: {:?}", res);

        Ok(res)
    }

    fn crypto_verify(&self,
                     their_vk: &str,
                     msg: &[u8],
                     signature: &[u8]) -> IndyResult<bool> {
        debug!("crypto_verify >>> their_vk: {:?}, msg: {:?}, signature: {:?}", their_vk, msg, signature);

        self.crypto_service.validate_key(their_vk)?;

        let res = self.crypto_service.verify(their_vk, msg, signature)?;

        debug!("crypto_verify <<< res: {:?}", res);

        Ok(res)
    }

    fn authenticated_encrypt(&self,
                             wallet_handle: i32,
                             my_vk: &str,
                             their_vk: &str,
                             msg: &[u8]) -> IndyResult<Vec<u8>> {
        debug!("authenticated_encrypt >>> wallet_handle: {:?}, my_vk: {:?}, their_vk: {:?}, msg: {:?}", wallet_handle, my_vk, their_vk, msg);

        self.crypto_service.validate_key(my_vk)?;
        self.crypto_service.validate_key(their_vk)?;

        let my_key: Key = self.wallet_service.get_indy_object(wallet_handle, my_vk, &RecordOptions::id_value())?;

        let res = self.crypto_service.authenticated_encrypt(&my_key, their_vk, msg)?;

        debug!("authenticated_encrypt <<< res: {:?}", res);

        Ok(res)
    }

    fn authenticated_decrypt(&self,
                             wallet_handle: i32,
                             my_vk: &str,
                             msg: &[u8]) -> IndyResult<(String, Vec<u8>)> {
        debug!("authenticated_decrypt >>> wallet_handle: {:?}, my_vk: {:?}, msg: {:?}", wallet_handle, my_vk, msg);

        self.crypto_service.validate_key(my_vk)?;

        let my_key: Key = self.wallet_service.get_indy_object(wallet_handle, my_vk, &RecordOptions::id_value())?;

        let res = self.crypto_service.authenticated_decrypt(&my_key, &msg)?;

        debug!("authenticated_decrypt <<< res: {:?}", res);
        Ok(res)
    }

    fn anonymous_encrypt(&self,
                         their_vk: &str,
                         msg: &[u8]) -> IndyResult<Vec<u8>> {
        debug!("anonymous_encrypt >>> their_vk: {:?}, msg: {:?}", their_vk, msg);

        self.crypto_service.validate_key(their_vk)?;

        let res = self.crypto_service.crypto_box_seal(their_vk, &msg)?;

        debug!("anonymous_encrypt <<< res: {:?}", res);

        Ok(res)
    }

    fn anonymous_decrypt(&self,
                         wallet_handle: i32,
                         my_vk: &str,
                         encrypted_msg: &[u8]) -> IndyResult<Vec<u8>> {
        debug!("anonymous_decrypt >>> wallet_handle: {:?}, my_vk: {:?}, encrypted_msg: {:?}", wallet_handle, my_vk, encrypted_msg);

        self.crypto_service.validate_key(&my_vk)?;

        let my_key: Key = self.wallet_service.get_indy_object(wallet_handle, &my_vk, &RecordOptions::id_value())?;

        let res = self.crypto_service.crypto_box_seal_open(&my_key, &encrypted_msg)?;

        debug!("anonymous_decrypt <<< res: {:?}", res);

        Ok(res)
    }

    fn set_key_metadata(&self, wallet_handle: i32, verkey: &str, metadata: &str) -> IndyResult<()> {
        debug!("set_key_metadata >>> wallet_handle: {:?}, verkey: {:?}, metadata: {:?}", wallet_handle, verkey, metadata);

        self.crypto_service.validate_key(verkey)?;

        let metadata = KeyMetadata {value: metadata.to_string()};

        self.wallet_service.upsert_indy_object(wallet_handle, &verkey, &metadata)?;

        debug!("set_key_metadata <<<");

        Ok(())
    }

    fn get_key_metadata(&self,
                        wallet_handle: i32,
                        verkey: &str) -> IndyResult<String> {
        debug!("get_key_metadata >>> wallet_handle: {:?}, verkey: {:?}", wallet_handle, verkey);

        self.crypto_service.validate_key(verkey)?;

        let metadata = self.wallet_service.get_indy_object::<KeyMetadata>(wallet_handle, &verkey, &RecordOptions::id_value())?;

        let res = metadata.value;

        debug!("get_key_metadata <<< res: {:?}", res);

        Ok(res)
    }
}
