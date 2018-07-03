use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;

/// A type-aware factory.
pub struct Factory {
    inner: HashMap<TypeId, Box<Any + Send + Sync + 'static>>,
}

impl fmt::Debug for Factory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Factory").finish()
    }
}

struct Constructor<T: 'static>(Box<Fn() -> T + Send + Sync + 'static>);

impl<T: 'static> Constructor<T> {
    fn instantiate(&self) -> T {
        (self.0)()
    }
}

impl Factory {
    /// Create a new `Factory` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use batch_core::Factory;
    ///
    /// let factory = Factory::new();
    /// ```
    pub fn new() -> Self {
        Factory {
            inner: HashMap::new(),
        }
    }

    /// Provide a constructor for a given type to the factory.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use batch_core::Factory;
    /// # struct PgConn;
    ///
    /// fn init_pg_conn() -> PgConn {
    ///     // ...
    /// # PgConn
    /// }
    ///
    /// let mut factory = Factory::new();
    /// factory.provide(init_pg_conn);
    /// ```
    pub fn provide<T: 'static>(&mut self, ctor: impl Fn() -> T + Send + Sync + 'static) {
        let ctor = Constructor(Box::new(ctor));
        self.inner.insert(TypeId::of::<T>(), Box::new(ctor));
    }

    /// Instantiate a value of type `T` if a constructor was provided.
    /// # Examples
    ///
    /// ```rust
    /// use batch_core::Factory;
    /// # struct PgConn;
    ///
    /// let factory = Factory::new();
    /// // ...
    /// # if false {
    /// let conn: PgConn = factory.instantiate().unwrap();
    /// # }
    /// ```
    pub fn instantiate<T: 'static>(&self) -> Option<T> {
        self.inner
            .get(&TypeId::of::<T>())
            .and_then(|raw| raw.downcast_ref::<Constructor<T>>())
            .map(|ctor| ctor.instantiate())
    }
}
