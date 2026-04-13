use std::{
    borrow::{Borrow, Cow},
    collections::HashSet,
    hash::Hash,
    sync::Mutex,
};

/// 一个线程安全的驻留池（interner）。
///
/// 它会将传入值去重后泄漏为 `'static` 引用，并在后续相同内容输入时复用同一地址。
/// 默认类型参数是 `[u8]`，因此 `Interner` 仍等价于 `Interner<[u8]>`。
#[derive(Default)]
pub struct Interner<T: ?Sized + 'static = [u8]> {
    cache: Mutex<HashSet<&'static T>>,
}

impl<T: ?Sized> Interner<T>
where
    T: ToOwned + Eq + Hash + 'static,
    T::Owned: Borrow<T> + Into<Box<T>>,
{
    /// 创建一个空的 `Interner`。
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashSet::new()),
        }
    }

    /// 驻留一个值并返回稳定的 `'static` 引用。
    ///
    /// 若池中已存在相同内容，则直接返回已有引用；
    /// 否则会将值所有权化后泄漏并插入池中。
    pub fn intern<'a>(&self, value: impl Into<Cow<'a, T>>) -> &'static T {
        let value = value.into();
        let mut guard = self.cache.lock().expect("Lock poisoned");

        if let Some(&cached) = guard.get(value.as_ref()) {
            return cached;
        }

        let leaked: &'static T = Box::leak(value.into_owned().into());
        guard.insert(leaked);
        leaked
    }
}
