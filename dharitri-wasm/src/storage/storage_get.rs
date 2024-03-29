use crate::{
    api::{ErrorApi, ErrorApiImpl, ManagedTypeApi, StorageReadApi, StorageReadApiImpl},
    err_msg,
    types::{
        BigInt, BigUint, ManagedBuffer, ManagedBufferNestedDecodeInput, ManagedRef, ManagedType,
    },
};
use alloc::boxed::Box;
use dharitri_codec::*;

use super::StorageKey;

struct StorageGetInput<'k, A>
where
    A: StorageReadApi + ManagedTypeApi + ErrorApi + 'static,
{
    key: ManagedRef<'k, A, StorageKey<A>>,
}

impl<'k, A> StorageGetInput<'k, A>
where
    A: StorageReadApi + ManagedTypeApi + ErrorApi + 'static,
{
    #[inline]
    fn new(key: ManagedRef<'k, A, StorageKey<A>>) -> Self {
        StorageGetInput { key }
    }

    fn to_managed_buffer(&self) -> ManagedBuffer<A> {
        let mbuf_handle = A::storage_read_api_impl()
            .storage_load_managed_buffer_raw(self.key.buffer.get_raw_handle());
        ManagedBuffer::from_raw_handle(mbuf_handle)
    }

    fn to_big_uint(&self) -> BigUint<A> {
        BigUint::from_bytes_be_buffer(&self.to_managed_buffer())
    }

    fn to_big_int(&self) -> BigInt<A> {
        BigInt::from_signed_bytes_be_buffer(&self.to_managed_buffer())
    }

    fn load_len_managed_buffer(&self) -> usize {
        A::storage_read_api_impl().storage_load_managed_buffer_len(self.key.buffer.get_raw_handle())
    }
}

impl<'k, A> TopDecodeInput for StorageGetInput<'k, A>
where
    A: StorageReadApi + ManagedTypeApi + ErrorApi + 'static,
{
    type NestedBuffer = ManagedBufferNestedDecodeInput<A>;

    fn byte_len(&self) -> usize {
        self.load_len_managed_buffer()
    }

    fn into_boxed_slice_u8(self) -> Box<[u8]> {
        let key_bytes = self.key.to_boxed_bytes();
        A::storage_read_api_impl()
            .storage_load_boxed_bytes(key_bytes.as_slice())
            .into_box()
    }

    fn into_u64(self) -> u64 {
        let mb = self.to_managed_buffer();
        if let Some(num) = mb.parse_as_u64() {
            num
        } else {
            storage_get_exit::<A>((), DecodeError::INPUT_TOO_LONG)
        }
    }

    fn into_i64(self) -> i64 {
        let key_bytes = self.key.to_boxed_bytes();
        A::storage_read_api_impl().storage_load_i64(key_bytes.as_slice())
    }

    fn into_specialized<T, F>(self, else_deser: F) -> Result<T, DecodeError>
    where
        T: TryStaticCast,
        F: FnOnce(Self) -> Result<T, DecodeError>,
    {
        if let Some(result) = try_execute_then_cast(|| self.to_managed_buffer()) {
            Ok(result)
        } else if let Some(result) = try_execute_then_cast(|| self.to_big_uint()) {
            Ok(result)
        } else if let Some(result) = try_execute_then_cast(|| self.to_big_int()) {
            Ok(result)
        } else {
            else_deser(self)
        }
    }

    fn into_nested_buffer(self) -> Self::NestedBuffer {
        ManagedBufferNestedDecodeInput::new(self.to_managed_buffer())
    }
}

pub fn storage_get<A, T>(key: ManagedRef<'_, A, StorageKey<A>>) -> T
where
    T: TopDecode,
    A: StorageReadApi + ManagedTypeApi + ErrorApi,
{
    T::top_decode_or_exit(StorageGetInput::new(key), (), storage_get_exit::<A>)
}

/// Useful for storage mappers.
/// Also calls to it generated by macro.
pub fn storage_get_len<A>(key: ManagedRef<'_, A, StorageKey<A>>) -> usize
where
    A: StorageReadApi + ManagedTypeApi + ErrorApi,
{
    A::storage_read_api_impl().storage_load_managed_buffer_len(key.get_raw_handle())
}

#[inline(always)]
fn storage_get_exit<A>(_: (), de_err: DecodeError) -> !
where
    A: StorageReadApi + ManagedTypeApi + ErrorApi + 'static,
{
    let mut message_buffer = ManagedBuffer::<A>::new_from_bytes(err_msg::STORAGE_DECODE_ERROR);
    message_buffer.append_bytes(de_err.message_bytes());
    A::error_api_impl().signal_error_from_buffer(message_buffer.get_raw_handle())
}
