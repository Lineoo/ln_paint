use ln_world::{Element, Handle, HandleGeneric, World};

type GetFunc<T> = Box<dyn Fn(&World) -> &T + Send + 'static>;
type SetFunc<T> = Box<dyn Fn(&World, T) + Send + 'static>;
type TransFunc<T> = Box<dyn Fn(&World) -> T + Send + 'static>;

/// simple field for data, without any insert/remove behavior
pub struct Val<T> {
    pub val: T,
}

/// accessor-like val binding
pub struct ValProp<T> {
    pub get: GetFunc<T>,
    pub set: SetFunc<T>,
}

/// Not guaranteed changes. May be triggered on Elements other than Val/ValProp.
pub struct ValChanged;

/// Binding a function for data, and listen to nodes.
pub struct ValBind<'w, T: Send + 'static, TG: Element> {
    world: &'w World,
    handle: Handle<TG>,
    fun: Handle<Val<TransFunc<T>>>,
}

impl<T> Val<T> {
    pub const fn new(val: T) -> Self {
        Val { val }
    }
}

impl<T> ValProp<T> {
    pub fn new(
        get: impl Fn(&World) -> &T + Send + 'static,
        set: impl Fn(&World, T) + Send + 'static,
    ) -> Self {
        ValProp {
            get: Box::new(get),
            set: Box::new(set),
        }
    }
}

impl<'w, T: Send + 'static> ValBind<'w, T, Val<T>> {
    pub fn new(
        world: &'w World,
        fun: impl Fn(&World) -> T + Send + 'static,
    ) -> ValBind<'w, T, Val<T>> {
        let fun = world.insert(Val::new(Box::new(fun) as TransFunc<T>));
        let handle = world.insert_with(move |world| {
            let fun = world.fetch(fun);
            Val::new((fun.val)(world))
        });
        ValBind { world, handle, fun }
    }

    /// standard val bind transfer
    pub fn transfer<U: Send + 'static, UG: ValGeneric<U> + Element>(
        world: &World,
        src: Handle<UG>,
        fun: impl Fn(&U) -> T + Send + 'static,
    ) -> Handle<Val<T>> {
        ValBind::new(world, move |world| {
            let src = world.fetch(src);
            let src = src.get(world);
            let val = fun(src);
            val
        })
        .listen(src)
        .finish()
    }

    /// any elements
    pub fn fetch<U: Element>(
        world: &World,
        src: Handle<U>,
        fun: impl Fn(&U, &World) -> T + Send + 'static,
    ) -> Handle<Val<T>> {
        ValBind::new(world, move |world| {
            let src = world.fetch(src);
            let val = fun(&src, world);
            val
        })
        .listen(src)
        .finish()
    }
}

impl<'w, T: Send + 'static, TG: ValGeneric<T> + Element> ValBind<'w, T, TG> {
    pub fn attach(
        world: &'w World,
        handle: Handle<TG>,
        fun: impl Fn(&World) -> T + Send + 'static,
    ) -> ValBind<'w, T, TG> {
        let fun = world.insert(Val::new(Box::new(fun) as TransFunc<T>));
        world.queue(move |world| {
            let fun = world.fetch(fun);
            let val = (fun.val)(world);
            let mut dst = world.fetch_mut(handle);
            dst.set(world, val);
            world.queue_trigger(dst.handle(), ValChanged);
        });
        ValBind { world, handle, fun }
    }

    /// standard val bind observer
    pub fn listen(self, this: impl HandleGeneric) -> Self {
        let ValBind { world, handle, fun } = self;
        world.observer(this, move |ValChanged, world| {
            let fun = world.fetch(fun);
            let val = (fun.val)(world);
            let mut dst = world.fetch_mut(handle);
            dst.set(world, val);
            world.queue_trigger(dst.handle(), ValChanged);
        });
        self
    }

    /// any observer
    pub fn observer<E: Send + 'static>(
        self,
        this: impl HandleGeneric,
        mut action: impl FnMut(&E, &World) -> T + Send + 'static,
    ) -> Self {
        let ValBind { world, handle, .. } = self;
        world.observer(this, move |event: &E, world| {
            let val = (action)(event, world);
            let mut dst = world.fetch_mut(handle);
            dst.set(world, val);
            world.queue_trigger(dst.handle(), ValChanged);
        });
        self
    }

    pub fn finish(self) -> Handle<TG> {
        self.handle
    }
}

impl<T: Send + 'static> Element for Val<T> {}
impl<T: Send + 'static> Element for ValProp<T> {}

pub trait ValGeneric<T> {
    fn get<'w>(&'w self, world: &'w World) -> &'w T;
    fn set<'w>(&'w mut self, world: &'w World, val: T);
}

impl<T> ValGeneric<T> for Val<T> {
    fn get<'w>(&'w self, _world: &'w World) -> &'w T {
        &self.val
    }

    fn set<'w>(&'w mut self, _world: &'w World, val: T) {
        self.val = val;
    }
}

impl<T> ValGeneric<T> for ValProp<T> {
    fn get<'w>(&'w self, world: &'w World) -> &'w T {
        (self.get)(world)
    }

    fn set<'w>(&'w mut self, world: &'w World, val: T) {
        (self.set)(world, val);
    }
}
