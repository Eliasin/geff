#[cfg(target_arch = "wasm32")]
use serde::de::DeserializeOwned;

#[cfg(target_arch = "wasm32")]
use serde::Serialize;

#[cfg(target_arch = "wasm32")]
impl<C: std::fmt::Debug + Serialize + Clone + Default + DeserializeOwned> PersistentState<C> {
    pub async fn load() -> Self {
        todo!()
    }
}
