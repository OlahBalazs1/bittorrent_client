use std::ops::{Deref, DerefMut};

use log::info;
use rand::random_range;

const IDENTIFIER: &'static [u8] = b"-CW";
const VERSION_NUMBER: &'static [u8; 4] = b"0001";

pub fn generate_peer_id() -> [u8; 20] {
    let mut id: [u8; 20] = [0; 20];
    id[0..3].clone_from_slice(IDENTIFIER);
    id[3..7].clone_from_slice(VERSION_NUMBER);
    for index in 7..20 {
        // keep everything nice and mostly ASCII
        id[index] = match random_range(0..3u8) {
            0 => random_range(b'a'..=b'z'),
            1 => random_range(b'A'..=b'Z'),
            2 => random_range(b'0'..=b'9'),
            _ => unreachable!(),
        }
    }
    info!("generated id: {}", String::from_utf8_lossy(&id));
    id
}

#[async_trait::async_trait]
pub trait Cancel {
    async fn cancel(&self);
}

#[repr(transparent)]
pub struct CancelGuard<T: Cancel + Send + Sync + 'static>(Option<T>);

impl<T: Cancel + Send + Sync + 'static> CancelGuard<T> {
    pub fn new(inner: T) -> Self {
        Self(Some(inner))
    }
    pub fn get(&self) -> &T {
        self.0.as_ref().unwrap()
    }
    pub fn get_mut(&mut self) -> &mut T {
        self.0.as_mut().unwrap()
    }
    pub fn into_inner(mut self) -> T {
        let inner = self.0.take();
        std::mem::forget(self);
        inner.unwrap()
    }
}

impl<T: Cancel + Send + Sync + 'static> Deref for CancelGuard<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}
impl<T: Cancel + Send + Sync + 'static> DerefMut for CancelGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap()
    }
}

impl<T: Cancel + Send + Sync + 'static> Drop for CancelGuard<T> {
    fn drop(&mut self) {
        if let Some(inner) = self.0.take() {
            tokio::spawn(async move {
                inner.cancel().await;
            });
        }
    }
}

#[async_trait::async_trait]
impl<T, U> Cancel for T
where
    T: Deref<Target = U> + Send + Sync,
{
    async fn cancel(&self) {
        (&*self).cancel().await
    }
}
