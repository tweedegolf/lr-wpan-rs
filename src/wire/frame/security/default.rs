//! Provides a default AEAD, key descriptor lookup, and device descriptor lookups
//!  to satisfy the type requirements for (de-)serializing frames without providing any security

use ccm::aead::generic_array::{
    typenum::consts::{U1, U16},
    GenericArray,
};
use cipher::{Block, BlockCipher, BlockCipherKey, BlockDecrypt, BlockEncrypt, NewBlockCipher};

use super::{
    auxiliary_security_header::KeyIdentifier, AddressingMode, DeviceDescriptor,
    DeviceDescriptorLookup, KeyDescriptorLookup,
};
use crate::wire::Address;

/// A struct that fullfills all of the trait bounds for serialization and deserializtion, but is not
/// actually capable of performing any of the operations
pub struct Unimplemented;

impl KeyDescriptorLookup<U16> for Unimplemented {
    fn lookup_key_descriptor(
        &self,
        _address_mode: AddressingMode,
        _key_identifier: Option<KeyIdentifier>,
        _device_address: Option<Address>,
    ) -> Option<(u64, GenericArray<u8, U16>)> {
        None
    }
}

impl BlockCipher for Unimplemented {
    type BlockSize = U16;

    type ParBlocks = U1;
}

impl BlockEncrypt for Unimplemented {
    fn encrypt_block(&self, _block: &mut Block<Self>) {}
}

impl BlockDecrypt for Unimplemented {
    fn decrypt_block(&self, _block: &mut Block<Self>) {}
}

impl NewBlockCipher for Unimplemented {
    type KeySize = U16;

    fn new(_key: &BlockCipherKey<Self>) -> Self {
        Unimplemented {}
    }
}

impl DeviceDescriptorLookup for Unimplemented {
    fn lookup_device(
        &mut self,
        _addressing_mode: AddressingMode,
        _address: Address,
    ) -> Option<&mut DeviceDescriptor> {
        None
    }
}
